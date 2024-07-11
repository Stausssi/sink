use anyhow::Result;
use log::info;
use regex::Regex;
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
    /// The full pathspec of the dependency.
    ///
    /// See [`GitHubPathspec`].
    #[serde(skip)]
    pub pathspec: GitHubPathspec,

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
        let pathspec = match GitHubPathspec::try_from(dependency.clone()) {
            Ok(pathspec) => pathspec,
            Err(e) => {
                if default_owner.is_none() {
                    return Err(e);
                }
                match GitHubPathspec::try_from(format!(
                    "{}/{}",
                    default_owner.as_ref().unwrap(),
                    dependency
                )) {
                    Ok(pathspec) => pathspec,
                    Err(e) => return Err(e),
                }
            }
        };

        Ok(GitHubDependency {
            pathspec,
            destination: PathBuf::from(destination.unwrap_or(String::from("."))),
            version: version.unwrap_or(GitHubVersion::Latest),
            gitignore,
        })
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

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash, Default)]
#[serde(try_from = "String", into = "String")]
pub struct GitHubPathspec {
    owner: String,
    repository: String,
    pattern: String,
}
impl GitHubPathspec {
    pub fn is_valid(&self) -> bool {
        !self.owner.is_empty() && !self.repository.is_empty() && !self.pattern.is_empty()
    }
}
impl Display for GitHubPathspec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from(self.clone()))
    }
}
impl From<GitHubPathspec> for String {
    fn from(value: GitHubPathspec) -> Self {
        format!("{}/{}:{}", value.owner, value.repository, value.pattern)
    }
}
impl TryFrom<String> for GitHubPathspec {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let re = Regex::new(r"^(?<owner>.+)/(?<repo>.+):(?<pattern>.+)$").unwrap();
        match re.captures(&value) {
            Some(captures) => Ok(GitHubPathspec {
                owner: String::from(&captures["owner"]),
                repository: String::from(&captures["repo"]),
                pattern: String::from(&captures["pattern"]),
            }),
            None => Err(anyhow::anyhow!("Invalid dependency path specification: '{value}'! Please ensure it's in the form of 'owner/repo:pattern'!")),
        }
    }
}

/* ---------- [ Functions ] ---------- */
fn _add(sink_toml: SinkTOML, dependency: GitHubDependency, short_form: bool) -> Result<SinkTOML> {
    if !dependency.pathspec.is_valid() {
        return Err(anyhow::anyhow!(
            "Invalid dependency: '{}'!",
            dependency.pathspec
        ));
    }

    let _pathspec = dependency.pathspec.to_string();
    info!("Adding {_pathspec}@{}...", dependency.version);

    // Fail if the dependency already exists
    if sink_toml.dependencies.contains_key(&dependency.pathspec) {
        return Err(anyhow::anyhow!("Dependency '{_pathspec}' already exists!"));
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
        table["version"] = toml_edit::value(dep_clone.version.to_string());
        table["destination"] = toml_edit::value(dep_clone.destination.display().to_string());
        table["gitignore"] = toml_edit::value(dep_clone.gitignore);

        dependency_type = DependencyType::Full(dep_clone);
        formatted_value = table;
    };

    match sink_toml.add_dependency(dependency, dependency_type, formatted_value) {
        Ok(sink_toml) => {
            info!("Added {_pathspec}!");
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
        dependency.pathspec,
        dependency.version,
        dependency.destination.display()
    );

    // TODO: Actually install

    // Use the GH CLI to download the asset
    // gh release download --repo owner/repo --pattern 'file-pattern' --destination 'destination'

    info!(
        "Downloaded {}@{} into '{}'!",
        dependency.pathspec,
        dependency.version,
        dependency.destination.display()
    );

    Ok(())
}

/* ---------- [ Tests ] ---------- */
#[cfg(test)]
mod tests {
    use super::*;

    mod test_dependency {
        use super::*;

        #[test]
        fn test_new_full() {
            let dependency = GitHubDependency::new(
                String::from("owner/repo:file-pattern"),
                Some(String::from("destination")),
                Some(GitHubVersion::Tag(String::from("v1.0.0"))),
                false,
                &None,
            )
            .unwrap();

            assert_eq!(dependency.pathspec.to_string(), "owner/repo:file-pattern");
            assert_eq!(dependency.destination, PathBuf::from("destination"));
            assert_eq!(dependency.version.to_string(), String::from("v1.0.0"));
            assert!(!dependency.gitignore);
        }

        #[test]
        fn test_new_invalid() {
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

            let dependency = GitHubDependency::new(
                String::from("owner/repo/pattern"),
                Some(String::from("destination")),
                Some(GitHubVersion::Tag(String::from("v1.0.0"))),
                false,
                &None,
            );

            assert!(dependency.is_err());
        }

        #[test]
        fn test_new_default() {
            let dependency = GitHubDependency::new(
                String::from("repo:pattern"),
                None,
                None,
                true,
                &Some(String::from("owner")),
            )
            .unwrap();

            assert_eq!(dependency.pathspec.to_string(), "owner/repo:pattern");
            assert_eq!(dependency.destination, PathBuf::from("."));
            assert_eq!(dependency.version.to_string(), String::from("latest"));
            assert!(dependency.gitignore);
        }
    }

    mod test_pathspec {
        use super::*;

        #[test]
        fn test_from_string() {
            let path_spec = GitHubPathspec::try_from(String::from("owner/repo:pattern")).unwrap();

            assert_eq!(path_spec.owner, "owner");
            assert_eq!(path_spec.repository, "repo");
            assert_eq!(path_spec.pattern, "pattern");

            let path_spec = GitHubPathspec::try_from(String::from(
                "complex-owner/weird%%repo:patt[A-Z]ern*.txt",
            ))
            .unwrap();

            assert_eq!(path_spec.owner, "complex-owner");
            assert_eq!(path_spec.repository, "weird%%repo");
            assert_eq!(path_spec.pattern, "patt[A-Z]ern*.txt");
        }

        #[test]
        fn test_from_string_invalid() {
            assert!(GitHubPathspec::try_from(String::from("owner/repo")).is_err());
            assert!(GitHubPathspec::try_from(String::from("repo:pattern")).is_err());
            assert!(GitHubPathspec::try_from(String::from("/:")).is_err());
            assert!(GitHubPathspec::try_from(String::from("owner/:pattern")).is_err());
        }

        #[test]
        fn test_into_string() {
            let path_spec = GitHubPathspec {
                owner: String::from("owner"),
                repository: String::from("repo"),
                pattern: String::from("pattern"),
            };

            assert_eq!(path_spec.to_string(), "owner/repo:pattern");
        }
    }
}
