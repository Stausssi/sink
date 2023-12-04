pub use functions::add;
pub use functions::download;

use anyhow::Result;
use clap::Args;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Args, Serialize, Deserialize, Debug)]
#[command(arg_required_else_help = true)]
#[serde(rename_all(deserialize = "kebab-case", serialize = "snake_case"))]
pub struct GitHubDependency {
    /// The name of the dependency.
    ///
    /// This is expected in the format [owner]/[repo]/[file-pattern].
    /// If 'default-owner' is set, [owner] will default to it.
    /// Same goes for 'default-repo'.
    #[serde(skip)]
    pub dependency: String,

    /// The local destination to download the file(s) into.
    ///
    /// Either an absolute path or a relative path starting from the directory of the sink TOML.
    #[arg(short, long = "destination", alias = "dest")]
    pub destination: PathBuf,

    /// The version to download.
    ///
    /// This corresponds to the git release tag.
    /// If set to 'latest', the latest release will be downloaded.
    /// If set to 'prerelease', the latest prerelease will be downloaded.
    #[arg(short, long)]
    pub version: String,

    /// The group to add the dependency to.
    ///
    /// If the group does not exist, it will be created automatically.
    #[arg(short, long)]
    #[serde(skip)]
    pub group: Option<String>,

    #[arg(skip)]
    pub repository: String,
}
impl GitHubDependency {
    /// Split the dependency name into it's components.
    ///
    /// The name must be "owner/repo/file-pattern".
    /// This can be ensured via validate().
    pub fn parts(&self) -> Result<_DependencyParts> {
        match self.dependency.matches('/').count() {
            0 | 2 => Ok(match self.dependency.rsplit_once('/') {
                Some(parts) => _DependencyParts(Some(String::from(parts.0)), String::from(parts.1)),
                None => _DependencyParts(None, self.dependency.clone()),
            }),
            c => Err(anyhow::anyhow!(
                "Invalid number ({c}) of '/' in dependency!"
            )),
        }
    }
}
impl Default for GitHubDependency {
    fn default() -> Self {
        GitHubDependency {
            dependency: String::from(""),
            destination: PathBuf::new(),
            version: String::from(""),
            group: None,
            repository: String::from(""),
        }
    }
}

/// The two components of a GitHub dependency.
///
/// The first part is the repo in owner/repository format.
/// The second part is the name of the dependency. It represents the file-pattern to download.
pub struct _DependencyParts(pub Option<String>, pub String);

/* ---------- [ CLI ] ---------- */
pub mod cli {

    use clap::Subcommand;

    #[derive(Subcommand, Debug)]
    #[command(arg_required_else_help = true)]
    pub enum SubcommandGitHub {
        /// Add and install a dependency.
        ///
        /// This downloads assets from GitHub releases to a local destination.
        Add(super::GitHubDependency),
    }
}

/* ---------- [ TOML ] ---------- */
pub mod toml {
    use serde::{Deserialize, Serialize};

    use crate::PluginOptions;

    use super::GitHubDependency;

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all(deserialize = "kebab-case", serialize = "snake_case"))]
    pub struct GitHubPluginOptions {
        #[serde(flatten)]
        pub sink_options: PluginOptions<GitHubDependency>,

        pub default_owner: Option<String>,
        pub default_repository: Option<String>,
    }
    impl Default for GitHubPluginOptions {
        fn default() -> Self {
            Self {
                sink_options: PluginOptions {
                    provider: Some(String::from("gh")),
                    default_group: None,
                    dependencies: None,
                },
                default_owner: None,
                default_repository: None,
            }
        }
    }
}

/* ---------- [ Functions ] ---------- */

mod functions {
    use std::collections::HashMap;

    use anyhow::Result;
    use log::{error, info};

    extern crate toml as ex_toml;

    use super::toml;
    use crate::{
        github::GitHubDependency,
        toml::{DependencyContainer, DependencyType},
        SinkError, SinkTOML,
    };

    const KEY_REPOSITORY: &str = "repository";
    const KEY_VERSION: &str = "version";
    const KEY_DESTINATION: &str = "destination";

    fn _add(sink_toml: &mut SinkTOML, dependency: &super::GitHubDependency) -> Result<()> {
        info!("Adding {}@{}...", dependency.dependency, dependency.version);

        let github_options = sink_toml
            .github
            .get_or_insert(toml::GitHubPluginOptions::default());

        let repository;
        let file_pattern;

        match dependency.parts() {
            Ok(parts) => {
                repository = match parts.0 {
                    Some(value) => value,
                    None => {
                        match &github_options.default_repository {
                            Some(value) => String::from(value),
                            None => {
                                return Err(anyhow::anyhow!("No repository provided and default-repository is also not set!"));
                            }
                        }
                    }
                };
                file_pattern = parts.1;
            }
            Err(e) => {
                return Err(e);
            }
        }

        // Check if it can be installed
        download(dependency)?;

        // Add to sink TOML
        sink_toml.formatted["GitHub"]["dependencies"][&file_pattern] = toml_edit::Item::Table({
            let mut new_table = toml_edit::Table::new();
            new_table.insert(KEY_REPOSITORY, toml_edit::value(repository.clone()));
            new_table.insert(KEY_VERSION, toml_edit::value(dependency.version.clone()));
            new_table.insert(
                KEY_DESTINATION,
                toml_edit::value(dependency.destination.display().to_string()),
            );

            new_table
        });

        match &github_options.sink_options.dependencies {
            Some(dependencies) => match dependencies {
                DependencyType::Grouped(dependencies) => {
                    Ok(())
                }
                DependencyType::Singular(dependencies) => {
                    Ok(())
                }
                DependencyType::Invalid(_) => {
                    Err(anyhow::anyhow!("Current GitHub configuration contains invalid entries. Please fix them before adding new ones!"))
                }
            },
            None => match &dependency.group {
                Some(group) => {
                    Ok(())
                }
                None => {
                    Ok(())
                }
            },
        }
    }
    /// Add a dependency.
    pub fn add(sink_toml: &mut SinkTOML, dependency: &super::GitHubDependency) {
        match _add(sink_toml, dependency) {
            Ok(_) => {
                info!("Added {}!", dependency.dependency);
            }
            Err(add_error) => {
                error!(
                    "{}",
                    SinkError::Any(
                        add_error.context(format!("Failed to add '{}'!", dependency.dependency))
                    )
                );
            }
        };
    }

    pub fn download(dependency: &super::GitHubDependency) -> Result<()> {
        info!(
            "Downloading {}@{} into {}...",
            dependency.dependency,
            dependency.version,
            dependency.destination.display()
        );

        // TODO: Actually install

        info!(
            "Downloaded {}@{} into {}!",
            dependency.dependency,
            dependency.version,
            dependency.destination.display()
        );

        Ok(())
    }
}
