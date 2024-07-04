use anyhow::Result;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, path::PathBuf};

extern crate toml as ex_toml;

use crate::{SinkError, SinkTOML};

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
    pub name: String, // This is only used to parse the CLI arguments

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
            destination: PathBuf::new(),
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
impl Display for GitHubVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitHubVersion::Latest => write!(f, "latest"),
            GitHubVersion::Prerelease => write!(f, "prerelease"),
            GitHubVersion::Tag(tag) => write!(f, "{}", tag),
        }
    }
}

/* ---------- [ Functions ] ---------- */
fn _add(sink_toml: &mut SinkTOML, dependency: &mut GitHubDependency) -> Result<()> {
    info!(
        "Adding {}@{}...",
        dependency.get_full_name(),
        dependency.version
    );

    // Check if it can be installed
    download(dependency)?;

    Ok(())
}
/// Add a dependency.
pub fn add(sink_toml: &mut SinkTOML, mut dependency: GitHubDependency) {
    match _add(sink_toml, &mut dependency) {
        Ok(_) => {
            info!("Added {}!", dependency.get_full_name());
        }
        Err(add_error) => {
            error!(
                "{}",
                SinkError::Any(
                    add_error.context(format!("Failed to add '{}'!", dependency.get_full_name()))
                )
            );
        }
    };
}

/// Download the given dependency.
pub fn download(dependency: &GitHubDependency) -> Result<()> {
    info!(
        "Downloading {}@{} into {}...",
        dependency.get_full_name(),
        dependency.version,
        dependency.destination.display()
    );

    // TODO: Actually install

    info!(
        "Downloaded {}@{} into {}!",
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
}
