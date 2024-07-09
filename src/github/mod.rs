use anyhow::Result;
use log::info;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, path::PathBuf};

extern crate toml as ex_toml;

use crate::{toml::DependencyType, SinkError, SinkTOML};

/// Provides a default value of `true` for [`serde`].
fn _default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all(deserialize = "kebab-case", serialize = "snake_case"))]
pub struct GitHubDependency {
    /// The origin of the dependency.
    ///
    /// This has to be in the 'owner/repo' format.
    /// TODO: Use an [`enum`] for this.
    pub origin: String,

    /// The name of the dependency.
    ///
    /// This is the name of the asset, supporting glob patterns.
    #[serde(skip)]
    pub name: String,

    /// The local destination to download the file(s) into.
    ///
    /// Either an absolute path or a relative path starting from the directory of the sink TOML.
    pub destination: PathBuf,

    /// The version to download.
    ///
    /// This corresponds to the git release tag.
    /// If set to 'latest', the latest release will be downloaded.
    /// If set to 'prerelease', the latest prerelease will be downloaded.
    pub version: GitHubVersion,

    /// Whether the downloaded asset should be added to the gitignore.
    ///
    /// This defaults to true.
    #[serde(default = "_default_true")]
    pub gitignore: bool,
}
impl GitHubDependency {
    pub fn new(
        dependency: String,
        destination: Option<String>,
        version: Option<GitHubVersion>,
        gitignore: bool,
        default_owner: &Option<String>,
    ) -> Result<Self> {
        let (origin, name) = match dependency.splitn(3, '/').collect::<Vec<&str>>()[..] {
            [owner, repo, pattern] => (format!("{}/{}", owner, repo), pattern),
            [repo, pattern] => {
                if default_owner.is_none() {
                    return Err(anyhow::anyhow!(
                        "No default owner set and not given explicitly!"
                    ));
                }
                (
                    format!("{}/{}", default_owner.as_ref().unwrap(), repo),
                    pattern,
                )
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Missing owner and/or repository specification!"
                ))
            }
        };

        Ok(GitHubDependency {
            origin,
            name: String::from(name),
            destination: PathBuf::from(destination.unwrap_or(String::from("."))),
            version: version.unwrap_or(GitHubVersion::Latest),
            gitignore,
        })
    }

    /// Get the full name (`owner/repo/file-pattern`) of the dependency
    fn get_full_name(&self) -> String {
        format!("{}/{}", self.origin, self.name)
    }
}
impl Default for GitHubDependency {
    fn default() -> Self {
        GitHubDependency {
            origin: String::from(""),
            name: String::from(""),
            destination: PathBuf::from("."),
            version: GitHubVersion::Latest,
            gitignore: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum GitHubVersion {
    Latest,
    Prerelease,

    #[serde(untagged)]
    Tag(String),
}
impl GitHubVersion {
    pub fn parse_cli(s: &str) -> Result<Self, String> {
        Ok(Self::from(s))
    }
}
impl Display for GitHubVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitHubVersion::Latest => write!(f, "latest"),
            GitHubVersion::Prerelease => write!(f, "prerelease"),
            GitHubVersion::Tag(tag) => write!(f, "{}", tag),
        }
    }
}
impl From<&str> for GitHubVersion {
    fn from(s: &str) -> Self {
        match s {
            "latest" => GitHubVersion::Latest,
            "prerelease" => GitHubVersion::Prerelease,
            _ => GitHubVersion::Tag(s.to_string()),
        }
    }
}

/* ---------- [ Functions ] ---------- */
fn _add(sink_toml: SinkTOML, dependency: GitHubDependency, short_form: bool) -> Result<SinkTOML> {
    let _full_name = dependency.get_full_name();
    info!("Adding {_full_name}@{}...", dependency.version);

    // Fail if the dependency is not fully specified.
    // It must:
    // - Have a valid origin (owner/repo) format
    // - Have a non-empty name
    if !dependency.origin.contains('/') || dependency.name.is_empty() {
        return Err(anyhow::anyhow!("Invalid dependency: '{_full_name}'!"));
    }

    // Fail if the dependency already exists
    if sink_toml.dependencies.contains_key(&dependency.name) {
        return Err(anyhow::anyhow!("Dependency '{_full_name}' already exists!"));
    }

    // Check if it can be installed
    download(&dependency)?;

    // Add the dependency to sink TOML
    let dependency_type;
    let formatted_value;
    if short_form {
        dependency_type = DependencyType::Version(dependency.version.clone());
        formatted_value = toml_edit::value(dependency.version.to_string())
    } else {
        let dep_clone = dependency.clone();
        let mut table = toml_edit::table();
        table["origin"] = toml_edit::value(dep_clone.origin.clone());
        table["version"] = toml_edit::value(dep_clone.version.to_string());
        table["destination"] = toml_edit::value(dep_clone.destination.display().to_string());
        table["gitignore"] = toml_edit::value(dep_clone.gitignore);

        dependency_type = DependencyType::Full(dep_clone);
        formatted_value = table;
    };

    match sink_toml.add_dependency(dependency, dependency_type, formatted_value) {
        Ok(sink_toml) => {
            info!("Added {_full_name}!");
            Ok(sink_toml)
        }
        Err(e) => Err(e),
    }
}
/// Add a dependency.
pub fn add(
    sink_toml: SinkTOML,
    dependency: GitHubDependency,
    short_form: bool,
) -> Result<SinkTOML, SinkError> {
    match _add(sink_toml, dependency, short_form) {
        Ok(sink_toml) => Ok(sink_toml),
        Err(add_error) => Err(SinkError::Any(
            add_error.context("Failed to add dependency!"),
        )),
    }
}

/// Download the given dependency.
pub fn download(dependency: &GitHubDependency) -> Result<()> {
    info!(
        "Downloading {}@{} into '{}' ...",
        dependency.get_full_name(),
        dependency.version,
        dependency.destination.display()
    );

    // TODO: Actually install

    // Use the GH CLI to download the asset
    // gh release download --repo owner/repo --pattern 'file-pattern' --destination 'destination'

    info!(
        "Downloaded {}@{} into '{}'!",
        dependency.get_full_name(),
        dependency.version,
        dependency.destination.display()
    );

    Ok(())
}

/* ---------- [ Tests ] ---------- */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_name() {
        let dependency = GitHubDependency {
            origin: String::from("owner/repo"),
            name: String::from("file-pattern"),
            destination: PathBuf::new(),
            version: GitHubVersion::Latest,
            gitignore: true,
        };

        assert_eq!(dependency.get_full_name(), "owner/repo/file-pattern");
    }

    #[test]
    fn test_new_dependency_full() {
        let dependency = GitHubDependency::new(
            String::from("owner/repo/file-pattern"),
            Some(String::from("destination")),
            Some(GitHubVersion::Tag(String::from("v1.0.0"))),
            false,
            &None,
        )
        .unwrap();

        assert_eq!(dependency.origin, "owner/repo");
        assert_eq!(dependency.name, "file-pattern");
        assert_eq!(dependency.destination, PathBuf::from("destination"));
        assert_eq!(dependency.version.to_string(), String::from("v1.0.0"));
        assert!(!dependency.gitignore);
    }

    #[test]
    fn test_new_dependency_invalid() {
        let dependency = GitHubDependency::new(
            String::from("pattern"),
            Some(String::from("destination")),
            Some(GitHubVersion::Tag(String::from("v1.0.0"))),
            false,
            &None,
        );

        assert!(dependency.is_err());

        let dependency = GitHubDependency::new(
            String::from("repo/pattern"),
            Some(String::from("destination")),
            Some(GitHubVersion::Tag(String::from("v1.0.0"))),
            false,
            &None,
        );

        assert!(dependency.is_err());
    }

    #[test]
    fn test_new_dependency_default() {
        let dependency = GitHubDependency::new(
            String::from("repo/pattern"),
            None,
            None,
            true,
            &Some(String::from("owner")),
        )
        .unwrap();

        assert_eq!(dependency.origin, "owner/repo");
        assert_eq!(dependency.name, "pattern");
        assert_eq!(dependency.destination, PathBuf::from("."));
        assert_eq!(dependency.version.to_string(), String::from("latest"));
        assert!(dependency.gitignore);
    }
}
