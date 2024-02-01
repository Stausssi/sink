pub use functions::add;
pub use functions::download;

use anyhow::Result;
use clap::Args;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::SinkDependency;

pub const PLUGIN_NAME: &str = "GitHub";

#[derive(Args, Serialize, Deserialize, Debug, Clone)]
#[command(arg_required_else_help = true)]
#[serde(rename_all(deserialize = "kebab-case", serialize = "snake_case"))]
pub struct GitHubDependency {
    /// The name of the dependency.
    ///
    /// This is expected in the format [owner]/[repo]/[file-pattern].
    /// If 'default-owner' is set, [owner] will default to it.
    /// Same goes for 'default-repo'.
    #[serde(skip)]
    pub dependency: String, // This is only used to parse the CLI arguments

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
    pub group: Option<String>, // This is only used to parse the CLI arguments

    /// The GitHub repository to download from.
    ///
    /// This includes the owner and the repository name in the format [owner]/[repository_name].
    #[arg(skip)]
    pub repository: String, // This is only used inside the TOML
}
impl GitHubDependency {
    /// Split the dependency name into it's components.
    ///
    /// The name must be "owner/repo/file-pattern".
    /// This can be ensured via validate().
    ///
    /// TODO: Validate in combination with Sink TOML
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
impl SinkDependency for GitHubDependency {}

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
    use anyhow::Result;
    use log::{error, info};

    extern crate toml as ex_toml;

    use super::{toml, PLUGIN_NAME};
    use crate::{toml::Dependency, SinkError, SinkTOML};

    const KEY_REPOSITORY: &str = "repository";
    const KEY_VERSION: &str = "version";
    const KEY_DESTINATION: &str = "destination";

    fn _add(sink_toml: &mut SinkTOML, dependency: &mut super::GitHubDependency) -> Result<()> {
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
                        let default_owner = match &github_options.default_owner {
                            Some(value) => String::from(value),
                            None => {
                                return Err(anyhow::anyhow!(
                                    "No owner provided and default-owner is also not set!"
                                ));
                            }
                        };
                        let default_repository = match &github_options.default_repository {
                            Some(value) => String::from(value),
                            None => {
                                return Err(anyhow::anyhow!("No repository provided and default-repository is also not set!"));
                            }
                        };

                        format!("{}/{}", default_owner, default_repository)
                    }
                };
                file_pattern = parts.1;

                dependency.repository = repository.clone();
            }
            Err(e) => {
                return Err(e);
            }
        }

        // Check if it can be installed
        download(dependency)?;

        // Create objects to reference later on
        let new_dependency = Dependency::Full(dependency.to_owned());
        let formatted_value = toml_edit::Item::Table({
            let mut new_table = toml_edit::Table::new();
            new_table.insert(KEY_REPOSITORY, toml_edit::value(repository));
            new_table.insert(KEY_VERSION, toml_edit::value(dependency.version.clone()));
            new_table.insert(
                KEY_DESTINATION,
                toml_edit::value(dependency.destination.display().to_string()),
            );
            new_table
        });

        sink_toml.add_dependency(
            PLUGIN_NAME,
            dependency.group.as_ref(),
            new_dependency,
            &file_pattern,
            formatted_value,
        )
    }
    /// Add a dependency.
    pub fn add(sink_toml: &mut SinkTOML, mut dependency: super::GitHubDependency) {
        match _add(sink_toml, &mut dependency) {
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
