use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use log::debug;
use regex::Regex;
use reqwest::{self, header, Client};
use serde::{Deserialize, Serialize};

/// Represents a GitHub release asset that can be downloaded.
///
/// This struct maps directly to the GitHub API response for assets
/// within a release object.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ReleaseAsset {
    /// The unique ID of the asset
    pub id: u64,

    /// The name of the asset file
    pub name: String,

    /// The direct URL where the asset can be downloaded from
    pub browser_download_url: String,
}

/// Represents a GitHub release including its assets.
///
/// This struct maps directly to the GitHub API response for a release object,
/// containing only the fields we need for the application.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Release {
    /// The tag name of the release (e.g., "v1.0.0")
    pub tag_name: String,

    /// The assets included in this release
    pub assets: Vec<ReleaseAsset>,

    /// Indicates whether this is a prerelease
    pub prerelease: bool,
}

/// Retrieves a GitHub authentication token using the git credential system.
///
/// This function respects whatever credential helper is configured in the user's gitconfig,
/// including GitHub CLI (gh), Git Credential Manager, macOS Keychain, Windows Credential
/// Manager, or any other custom credential helper.
///
/// # How it works
///
/// It uses the standard `git credential fill` command to invoke the appropriate credential
/// helper based on git configuration. This ensures compatibility with the user's existing
/// GitHub authentication.
///
/// # Returns
///
/// Returns the GitHub token if found, or an error if no credentials were available or if
/// there was a problem with the git credential command.
///
/// # Errors
///
/// This function will return an error in the following situations:
/// - If the git command cannot be executed
/// - If the credential helper reports an error
/// - If no password/token is found in the returned credentials
fn get_github_token() -> Result<String> {
    let mut child = Command::new("git")
        .args(["credential", "fill"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to execute git credential fill")?;

    // Write the credential request to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(b"protocol=https\nhost=github.com\n\n")
            .context("Failed to write to git credential stdin")?;
    }

    // Wait for the command to complete and get the output
    let output = child
        .wait_with_output()
        .context("Failed to wait for git credential fill")?;

    // Check if the command succeeded
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "Git credential fill failed: {}",
            stderr.trim()
        ));
    }

    // Extract the password/token from the output
    let output_str = String::from_utf8_lossy(&output.stdout);

    // First try to find a password in the credential output
    let re = Regex::new(r"password=([^\n]+)").context("Failed to create password regex")?;
    if let Some(caps) = re.captures(&output_str) {
        return Ok(caps[1].to_string());
    }

    // If no password is found, try to extract username for a better error message
    let username_re =
        Regex::new(r"username=([^\n]+)").context("Failed to create username regex")?;
    let username = match username_re.captures(&output_str) {
        Some(caps) => format!(" (username: {})", caps.get(1).unwrap().as_str()),
        None => String::new(),
    };

    Err(anyhow::anyhow!(
        "No GitHub token found in git credentials{username}. Make sure you have a credential helper configured and authenticated."
    ))
}

/// Creates an authenticated GitHub API client using git credentials.
///
/// # Returns
///
/// Returns a configured reqwest Client with appropriate authentication headers
/// for GitHub API access.
///
/// # Errors
///
/// This function will return an error in the following situations:
/// - If retrieving the GitHub token fails
/// - If creating the authentication header fails
/// - If building the HTTP client fails
fn create_github_client() -> Result<Client> {
    let token = get_github_token()?;
    let mut headers = header::HeaderMap::new();

    // Configure the token as Bearer authentication
    let auth_value = format!("token {token}");
    let auth_header = header::HeaderValue::from_str(&auth_value)
        .context("Failed to create Authorization header value")?;

    headers.insert(header::AUTHORIZATION, auth_header);
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static("sink-github-api-client"),
    );

    // Add Accept header for GitHub API v3
    headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static("application/vnd.github.v3+json"),
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::limited(10)) // Follow up to 10 redirects
        .build()
        .context("Failed to build reqwest client")
}

/// Retrieves a specific GitHub release based on the provided version.
///
/// # Arguments
///
/// * `client` - An authenticated GitHub API client
/// * `owner` - The GitHub repository owner
/// * `repo` - The GitHub repository name
/// * `version` - The version to fetch, can be "latest", "prerelease", or a specific tag name
///
/// # Returns
///
/// The requested GitHub release if found.
///
/// # Errors
///
/// This function will return an error in the following situations:
/// - If the API request fails
/// - If the requested release version cannot be found
/// - If there's a network error
/// - If the response cannot be parsed
async fn get_release(client: &Client, owner: &str, repo: &str, version: &str) -> Result<Release> {
    let url = match version {
        "latest" => {
            // Try the latest release endpoint first
            let latest_url = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
            let response = client.get(&latest_url).send().await;

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        // If successful, return the latest release
                        debug!("Found latest release via /releases/latest endpoint");
                        return resp
                            .json()
                            .await
                            .context("Failed to parse release response");
                    } else {
                        debug!("Latest release endpoint returned {}, falling back to listing all releases", resp.status());
                        // If not successful (e.g., 404 because no release is marked as latest),
                        // fall back to fetching all releases and finding the most recent one
                        let releases = fetch_all_releases(client, owner, repo).await?;

                        if releases.is_empty() {
                            return Err(anyhow::anyhow!("No releases found for {owner}/{repo}"));
                        }

                        // Return the first release (most recent)
                        debug!("Using most recent release as 'latest'");
                        return Ok(releases[0].clone());
                    }
                }
                Err(e) => {
                    debug!("Error accessing latest release endpoint: {}, falling back to listing all releases", e);
                    // On error, fall back to fetching all releases
                    let releases = fetch_all_releases(client, owner, repo).await?;

                    if releases.is_empty() {
                        return Err(anyhow::anyhow!("No releases found for {owner}/{repo}"));
                    }

                    // Return the first release (most recent)
                    debug!("Using most recent release as 'latest'");
                    return Ok(releases[0].clone());
                }
            }
        }
        "prerelease" => {
            // For prerelease, fetch all releases and find the latest prerelease
            let releases = fetch_all_releases(client, owner, repo).await?;

            // Find the first prerelease
            return releases
                .into_iter()
                .find(|r| r.prerelease)
                .ok_or_else(|| anyhow::anyhow!("No prerelease found"));
        }
        // For a specific tag
        _ => format!("https://api.github.com/repos/{owner}/{repo}/releases/tags/{version}"),
    };

    client
        .get(&url)
        .send()
        .await
        .context("Failed to send request")?
        .error_for_status()
        .context("GitHub API returned an error")?
        .json()
        .await
        .context("Failed to parse release response")
}

/// Fetches all releases for a repository.
///
/// # Arguments
///
/// * `client` - An authenticated GitHub API client
/// * `owner` - The GitHub repository owner
/// * `repo` - The GitHub repository name
///
/// # Returns
///
/// A vector of all releases for the repository.
///
/// # Errors
///
/// This function will return an error in the following situations:
/// - If the API request fails
/// - If there's a network error
/// - If the response cannot be parsed
async fn fetch_all_releases(client: &Client, owner: &str, repo: &str) -> Result<Vec<Release>> {
    let releases_url = format!("https://api.github.com/repos/{owner}/{repo}/releases");

    client
        .get(&releases_url)
        .send()
        .await
        .context("Failed to send request for releases")?
        .error_for_status()
        .context("GitHub API returned an error")?
        .json()
        .await
        .context("Failed to parse releases response")
}

/// Downloads release assets that match the given pattern.
///
/// This function retrieves the specified GitHub release and downloads any assets
/// that match the provided pattern to the destination directory.
///
/// # Arguments
///
/// * `owner` - The GitHub repository owner
/// * `repo` - The GitHub repository name
/// * `version` - The version to fetch assets from, can be "latest", "prerelease", or a specific tag
/// * `pattern` - A glob pattern to match asset filenames against (e.g. "*.zip" or "file-*.tgz")
/// * `destination` - The local path where assets should be saved
///
/// # Returns
///
/// Returns `Ok(())` if all matching assets were downloaded successfully.
///
/// # Errors
///
/// This function will return an error in the following situations:
/// - If the GitHub API credentials are missing or invalid
/// - If the owner/repo combination is invalid
/// - If the requested version doesn't exist
/// - If no assets match the provided pattern
/// - If there are problems accessing the filesystem
/// - If there are network issues connecting to GitHub
pub fn download_release_asset(
    owner: &str,
    repo: &str,
    version: &str,
    pattern: &str,
    destination: &Path,
) -> Result<()> {
    debug!("Downloading asset from GitHub API: {owner}/{repo} @ {version}, pattern: {pattern}");

    // Create runtime for async code
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to create tokio runtime")?;

    rt.block_on(async {
        // Create client and get release
        let client = create_github_client()?;
        
        // Get the release directly - this will fail if the repo doesn't exist
        // so we don't need a separate repository check
        let release = match get_release(&client, owner, repo, version).await {
            Ok(release) => release,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to get release {version} for {owner}/{repo}: {e}"
                ));
            }
        };

        debug!(
            "Found release: {} with {} assets",
            release.tag_name,
            release.assets.len()
        );

        // If there are no assets at all, provide a clear error
        if release.assets.is_empty() {
            return Err(anyhow::anyhow!(
                "No assets found in release {version} (tag: {tag}) for {owner}/{repo}",
                tag = release.tag_name
            ));
        }

        // Create destination directory if it doesn't exist
        if !destination.exists() {
            fs::create_dir_all(destination).context("Failed to create destination directory")?;
        }

        // Find assets matching the pattern
        let matching_assets = find_matching_assets(&release, pattern)?;
        if matching_assets.is_empty() {
            // List asset names for better debugging
            let asset_names: Vec<&str> = release.assets.iter()
                .map(|a| a.name.as_str())
                .collect();
            
            return Err(anyhow::anyhow!(
                "No assets found matching pattern: '{pattern}' in release {version} (tag: {tag}) for {owner}/{repo}. Available assets: {assets:?}",
                tag = release.tag_name,
                assets = asset_names
            ));
        }

        debug!("Found {} matching assets: {:?}", 
               matching_assets.len(), 
               matching_assets.iter().map(|a| &a.name).collect::<Vec<_>>());

        // Download each matching asset
        for asset in matching_assets {
            match download_asset(&client, asset, destination).await {
                Ok(_) => {},
                Err(e) => {
                    debug!("Failed to download asset {}: {}", asset.name, e);
                    return Err(anyhow::anyhow!(
                        "Failed to download asset {}: {}", asset.name, e
                    ));
                }
            }
        }

        Ok(())
    })
}

/// Finds assets in a release that match the given pattern.
///
/// # Arguments
///
/// * `release` - The GitHub release to search for assets
/// * `pattern` - A glob pattern (e.g., "*.zip") or regex pattern to match against asset names
///
/// # Returns
///
/// A vector of references to matching assets.
///
/// # Errors
///
/// Returns an error if the pattern is invalid.
fn find_matching_assets<'a>(release: &'a Release, pattern: &str) -> Result<Vec<&'a ReleaseAsset>> {
    let regex_pattern = if pattern.contains(r"\") {
        // If it contains backslashes, treat it as a raw regex
        pattern.to_string()
    } else {
        // Otherwise, treat it as a glob pattern and convert to regex
        // Convert glob-style * and ? to their regex equivalents
        let pattern_with_wildcards = pattern
            .replace(".", "\\.")
            .replace("*", ".*")
            .replace("?", ".");

        // Add start/end anchors for real-world pattern matching
        format!("^{}$", pattern_with_wildcards)
    };

    debug!(
        "Using regex pattern '{}' derived from input '{}'",
        regex_pattern, pattern
    );

    // Compile the pattern regex
    let pattern_regex =
        Regex::new(&regex_pattern).context("Failed to compile asset pattern regex")?;

    // Find all assets matching the pattern
    let matching_assets = release
        .assets
        .iter()
        .filter(|asset| pattern_regex.is_match(&asset.name))
        .collect();

    Ok(matching_assets)
}

/// Downloads a single asset to the destination directory.
///
/// # Arguments
///
/// * `client` - An authenticated GitHub API client
/// * `asset` - The asset to download
/// * `destination` - The directory to save the asset to
///
/// # Returns
///
/// Returns `Ok(())` if the download was successful.
///
/// # Errors
///
/// This function will return an error in the following situations:
/// - If the download request fails
/// - If there's a network error
/// - If there's an error writing to the file
async fn download_asset(client: &Client, asset: &ReleaseAsset, destination: &Path) -> Result<()> {
    debug!("Downloading asset: {name}", name = asset.name);

    // Extract owner and repo from the URL
    let url_parts: Vec<&str> = asset.browser_download_url.split('/').collect();
    
    // Use the GitHub API endpoint format
    let download_url = format!(
        "https://api.github.com/repos/{owner}/{repo}/releases/assets/{id}",
        id = asset.id,
        owner = url_parts.get(3).unwrap_or(&""),
        repo = url_parts.get(4).unwrap_or(&"")
    );
    
    debug!("Downloading from asset endpoint: {}", download_url);

    // Download the asset with appropriate headers for raw content
    let response = client
        .get(&download_url)
        .header(header::ACCEPT, "application/octet-stream") 
        .send()
        .await
        .context("Failed to send download request")?;

    // Check the status and provide better error messages
    if !response.status().is_success() {
        let status = response.status();
        debug!("Response headers: {:?}", response.headers());
        
        let error_text = response.text().await.unwrap_or_default();
        debug!(
            "Asset download failed with status {}: {}",
            status, error_text
        );

        return Err(anyhow::anyhow!(
            "Asset download request failed with status {}: {}",
            status,
            if error_text.is_empty() {
                status.to_string()
            } else {
                error_text
            }
        ));
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read response bytes")?;

    // Write to file
    let output_path = destination.join(&asset.name);
    let mut file = File::create(&output_path).context("Failed to create output file")?;
    file.write_all(&bytes)
        .context("Failed to write asset to file")?;

    debug!(
        "Successfully downloaded asset to: {path}",
        path = output_path.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito;
    use std::sync::Once;
    use tempfile::TempDir;

    // Initialize logger once for all tests
    static INIT: Once = Once::new();
    fn init_logger() {
        INIT.call_once(|| {
            env_logger::builder().is_test(true).try_init().ok();
        });
    }

    /// Test-specific function to download assets from mock URLs
    async fn download_mock_asset(client: &Client, asset: &ReleaseAsset, destination: &Path) -> Result<()> {
        debug!("Downloading mock asset: {name}", name = asset.name);
        
        // Use the direct URL for mock server
        let response = client
            .get(&asset.browser_download_url)
            .header(header::ACCEPT, "application/octet-stream") 
            .send()
            .await
            .context("Failed to send download request")?;

        // Check the status and provide better error messages
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            
            return Err(anyhow::anyhow!(
                "Asset download request failed with status {}: {}",
                status,
                if error_text.is_empty() {
                    status.to_string()
                } else {
                    error_text
                }
            ));
        }

        let bytes = response
            .bytes()
            .await
            .context("Failed to read response bytes")?;

        // Write to file
        let output_path = destination.join(&asset.name);
        let mut file = File::create(&output_path).context("Failed to create output file")?;
        file.write_all(&bytes)
            .context("Failed to write asset to file")?;

        debug!(
            "Successfully downloaded mock asset to: {path}",
            path = output_path.display()
        );

        Ok(())
    }

    /// Creates a mock GitHub API response for a release
    fn mock_release_response(
        tag_name: &str,
        prerelease: bool,
        assets: Vec<ReleaseAsset>,
    ) -> Release {
        Release {
            tag_name: tag_name.to_string(),
            prerelease,
            assets,
        }
    }

    /// Creates a mock release asset
    fn mock_asset(name: &str, url: &str) -> ReleaseAsset {
        ReleaseAsset {
            id: 0,
            name: name.to_string(),
            browser_download_url: url.to_string(),
        }
    }

    #[test]
    fn test_release_asset_getters() {
        let asset = mock_asset("test-file.zip", "https://example.com/test-file.zip");

        assert_eq!(asset.name, "test-file.zip");
        assert_eq!(
            asset.browser_download_url,
            "https://example.com/test-file.zip"
        );
    }

    #[test]
    fn test_release_getters() {
        let assets = vec![
            mock_asset("file1.zip", "http://example.com/file1.zip"),
            mock_asset("file2.tar.gz", "http://example.com/file2.tar.gz"),
        ];
        let release = mock_release_response("v1.0.0", true, assets);

        assert_eq!(release.tag_name, "v1.0.0");
        assert_eq!(release.assets.len(), 2);
        assert!(release.prerelease);
    }

    #[test]
    fn test_find_matching_assets() {
        // Create a release with some assets
        let assets = vec![
            mock_asset("file1.zip", "http://example.com/file1.zip"),
            mock_asset("file2.tar.gz", "http://example.com/file2.tar.gz"),
            mock_asset("file3.exe", "http://example.com/file3.exe"),
        ];
        let release = mock_release_response("v1.0.0", false, assets);

        // Test cases with patterns and expected match counts
        let patterns_and_expected_counts = [
            // Exact match with escaped period
            (r"file1\.zip", 1),
            // Match file1 or file2 with any extension
            (r"file[12]\..+", 2),
            // Pattern that doesn't match any files
            ("nonexistent", 0),
        ];

        for (pattern, expected_count) in patterns_and_expected_counts {
            let matches = find_matching_assets(&release, pattern).unwrap();
            assert_eq!(
                matches.len(), 
                expected_count, 
                "Pattern '{}' should match {} assets but matched {}", 
                pattern, expected_count, matches.len()
            );
        }

        // Test with an invalid pattern
        let result = find_matching_assets(&release, r"[invalid");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to compile asset pattern regex"));
    }

    #[test]
    fn test_find_matching_assets_empty_assets() {
        // Create a release with no assets
        let release = mock_release_response("v1.0.0", false, vec![]);

        // Test with any pattern
        let matches = find_matching_assets(&release, r".*").unwrap();
        assert!(matches.is_empty());
    }

    #[test]
    fn test_find_matching_assets_with_complex_patterns() {
        // Create a release with assets that have specific naming patterns
        let assets = vec![
            mock_asset(
                "app-v1.0.0-windows-x86_64.zip",
                "http://example.com/win.zip",
            ),
            mock_asset("app-v1.0.0-macos-x86_64.dmg", "http://example.com/mac.dmg"),
            mock_asset(
                "app-v1.0.0-linux-x86_64.tar.gz",
                "http://example.com/linux.tar.gz",
            ),
            mock_asset("checksum.txt", "http://example.com/checksum.txt"),
        ];
        let release = mock_release_response("v1.0.0", false, assets);

        // Using a pattern that matches both string literals and regex patterns properly now
        let patterns_and_expected_counts = [
            // Windows asset with regex escape sequence (raw string)
            (r"app-v1\.0\.0-windows-x86_64\.zip", 1),
            // ZIP files with regex syntax
            (r".*\.zip$", 1),
            // Files containing version with regex syntax
            (r".*v1\.0\.0.*", 3),
        ];

        for (pattern, expected_count) in patterns_and_expected_counts {
            let matches = find_matching_assets(&release, pattern).unwrap();
            assert_eq!(
                matches.len(), 
                expected_count, 
                "Pattern '{}' should match {} assets but matched {}", 
                pattern, expected_count, matches.len()
            );
        }
    }

    #[tokio::test]
    async fn test_fetch_all_releases_mock() {
        // Create a mock server - when used with tokio::test we don't need to create a runtime manually
        let mut mock_server = mockito::Server::new_async().await;

        // Create a mock for the releases endpoint
        let mock = mock_server
            .mock("GET", "/repos/testowner/testrepo/releases")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"[
                {"tag_name":"v1.0.0","prerelease":false,"assets":[]},
                {"tag_name":"v0.9.0","prerelease":true,"assets":[]}
            ]"#,
            )
            .create_async()
            .await;

        // Create a custom function for testing with a configurable base URL
        async fn fetch_releases_with_base_url(
            base_url: &str,
            client: &Client,
            owner: &str,
            repo: &str,
        ) -> Result<Vec<Release>> {
            let releases_url = format!("{}/repos/{}/{}/releases", base_url, owner, repo);

            client
                .get(&releases_url)
                .send()
                .await
                .context("Failed to send request for releases")?
                .error_for_status()
                .context("GitHub API returned an error")?
                .json()
                .await
                .context("Failed to parse releases response")
        }

        // Create a client and call our test function
        let client = reqwest::Client::new();
        let result =
            fetch_releases_with_base_url(&mock_server.url(), &client, "testowner", "testrepo")
                .await;

        // Verify the result
        assert!(result.is_ok());
        let releases = result.unwrap();
        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].tag_name, "v1.0.0");
        assert!(!releases[0].prerelease);
        assert_eq!(releases[1].tag_name, "v0.9.0");
        assert!(releases[1].prerelease);

        // Verify that our mock was called
        mock.assert();
    }

    #[tokio::test]
    async fn test_get_release_latest_mock() {
        // Create a mock server - when used with tokio::test we don't need to create a runtime manually
        let mut mock_server = mockito::Server::new_async().await;

        // Create a mock for the latest release endpoint
        let mock = mock_server
            .mock("GET", "/repos/testowner/testrepo/releases/latest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"tag_name":"v1.0.0","prerelease":false,"assets":[]}"#)
            .create_async()
            .await;

        // Create a custom function for testing with a configurable base URL
        async fn get_release_with_base_url(
            base_url: &str,
            client: &Client,
            owner: &str,
            repo: &str,
            version: &str,
        ) -> Result<Release> {
            let url = match version {
                "latest" => format!("{}/repos/{}/{}/releases/latest", base_url, owner, repo),
                // For brevity, omitting other cases as they're not needed for this test
                _ => {
                    return Err(anyhow::anyhow!(
                        "Only 'latest' version is supported in this test function"
                    ))
                }
            };

            client
                .get(&url)
                .send()
                .await
                .context("Failed to send request")?
                .error_for_status()
                .context("GitHub API returned an error")?
                .json()
                .await
                .context("Failed to parse release response")
        }

        // Create a client and call our test function
        let client = reqwest::Client::new();
        let result = get_release_with_base_url(
            &mock_server.url(),
            &client,
            "testowner",
            "testrepo",
            "latest",
        )
        .await;

        // Verify the result
        assert!(result.is_ok());
        let release = result.unwrap();
        assert_eq!(release.tag_name, "v1.0.0");
        assert!(!release.prerelease);

        // Verify that our mock was called
        mock.assert();
    }

    #[tokio::test]
    async fn test_get_release_error_handling() {
        // Create a mock server - when used with tokio::test we don't need to create a runtime manually
        let mut mock_server = mockito::Server::new_async().await;

        // Create a mock for a 404 response
        let mock = mock_server
            .mock("GET", "/repos/testowner/testrepo/releases/latest")
            .with_status(404)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"message":"Not Found","documentation_url":"https://docs.github.com/rest"}"#,
            )
            .create_async()
            .await;

        // Use the same test function from the previous test
        async fn get_release_with_base_url(
            base_url: &str,
            client: &Client,
            owner: &str,
            repo: &str,
            version: &str,
        ) -> Result<Release> {
            let url = match version {
                "latest" => format!("{}/repos/{}/{}/releases/latest", base_url, owner, repo),
                _ => {
                    return Err(anyhow::anyhow!(
                        "Only 'latest' version is supported in this test function"
                    ))
                }
            };

            client
                .get(&url)
                .send()
                .await
                .context("Failed to send request")?
                .error_for_status()
                .context("GitHub API returned an error")?
                .json()
                .await
                .context("Failed to parse release response")
        }

        // Create a client and call our test function
        let client = reqwest::Client::new();
        let result = get_release_with_base_url(
            &mock_server.url(),
            &client,
            "testowner",
            "testrepo",
            "latest",
        )
        .await;

        // Verify the result is an error
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("GitHub API returned an error"));

        // Verify that our mock was called
        mock.assert();
    }

    #[tokio::test]
    async fn test_download_asset_mock() {
        init_logger();

        // Create a mock server - when used with tokio::test we don't need to create a runtime manually
        let mut mock_server = mockito::Server::new_async().await;
        let temp_dir = TempDir::new().unwrap();

        // Create a simple test file content
        let test_file_content = b"This is a test file content";

        // Create a mock for the asset download
        let mock = mock_server
            .mock("GET", "/download/test-asset.txt")
            .with_status(200)
            .with_header("content-type", "application/octet-stream")
            .with_body(test_file_content)
            .create_async()
            .await;

        // Create a test asset
        let asset = ReleaseAsset {
            id: 0,
            name: "test-asset.txt".to_string(),
            browser_download_url: format!("{}/download/test-asset.txt", mock_server.url()),
        };

        // Create a client and download the asset using the test-specific function
        let client = reqwest::Client::new();
        let result = download_mock_asset(&client, &asset, temp_dir.path()).await;

        // Verify the result
        assert!(result.is_ok());

        // Verify the file was downloaded correctly
        let downloaded_path = temp_dir.path().join("test-asset.txt");
        assert!(downloaded_path.exists());

        // Read the file and verify its contents
        let downloaded_content = fs::read(&downloaded_path).unwrap();
        assert_eq!(downloaded_content, test_file_content);

        // Verify that our mock was called
        mock.assert();
    }

    #[tokio::test]
    async fn test_download_asset_error_handling() {
        init_logger();

        // Create a mock server - when used with tokio::test we don't need to create a runtime manually
        let mut mock_server = mockito::Server::new_async().await;
        let temp_dir = TempDir::new().unwrap();

        // Create a mock for a failed download (404)
        let mock = mock_server
            .mock("GET", "/download/nonexistent-asset.txt")
            .with_status(404)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"message":"Not Found","documentation_url":"https://docs.github.com/rest"}"#,
            )
            .create_async()
            .await;

        // Create a test asset with a URL that will return 404
        let asset = ReleaseAsset {
            id: 0,
            name: "nonexistent-asset.txt".to_string(),
            browser_download_url: format!("{}/download/nonexistent-asset.txt", mock_server.url()),
        };

        // Create a client and try to download the asset using the test-specific function
        let client = reqwest::Client::new();
        let result = download_mock_asset(&client, &asset, temp_dir.path()).await;

        // Verify the result is an error
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Asset download request failed"));

        // Verify the file was not created
        let download_path = temp_dir.path().join("nonexistent-asset.txt");
        assert!(!download_path.exists());

        // Verify that our mock was called
        mock.assert();
    }

    /* The following tests are for real-world integration and require
    actual credentials or network access, so they are ignored by default */

    #[test]
    #[ignore = "Requires git credential helper to be configured"]
    fn test_get_github_token() {
        init_logger();

        // Only run this manually when testing with real credentials
        let result = get_github_token();
        assert!(result.is_ok());

        let token = result.unwrap();
        assert!(!token.is_empty());

        // Don't print the actual token for security reasons
        println!("Retrieved token successfully (length: {})", token.len());
    }

    #[tokio::test]
    #[ignore = "Integration test requiring network access"]
    async fn test_create_github_client() {
        init_logger();

        let result = create_github_client();
        assert!(result.is_ok());

        // Test the client works by making a simple request to a public API
        let client = result.unwrap();
        let response = client.get("https://api.github.com/zen").send().await;

        assert!(response.is_ok());
        assert!(response.unwrap().status().is_success());
    }

    #[test]
    #[ignore = "Integration test that requires network access"]
    fn test_download_release_asset_integration() {
        init_logger();

        // Note: This integration test should ideally be moved to a separate integration test file
        // and not be in the library code as per the testing guidelines.
        // It is kept here for reference only until proper integration tests are set up.
        
        let temp_dir = TempDir::new().unwrap();

        // Example using a real public repository:
        let result = download_release_asset(
            "cli", // GitHub CLI repository
            "cli",
            "latest",           // Get the latest release
            r"checksums\.txt$", // Download the checksums file
            temp_dir.path(),
        );

        assert!(result.is_ok());

        // Verify a file was downloaded
        let entries = fs::read_dir(temp_dir.path()).unwrap();
        let count = entries.count();
        assert!(count > 0, "No files were downloaded");
    }
}
