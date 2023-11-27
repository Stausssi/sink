pub use functions::add;
pub use functions::download;

/* ---------- [ CLI ] ---------- */
pub mod cli {
    use clap::{Args, Subcommand};
    use std::path::PathBuf;

    use crate::SinkError;

    #[derive(Subcommand, Debug)]
    #[command(arg_required_else_help = true)]
    pub enum SubcommandGitHub {
        /// Add and install a dependency.
        ///
        /// This downloads assets from GitHub releases to a local destination.
        Add(GitHubDependency),
    }

    #[derive(Args, Debug)]
    #[command(arg_required_else_help = true)]
    pub struct GitHubDependency {
        /// The name of the dependency.
        ///
        /// This is expected in the format [owner]/[repo]/[file-pattern].
        /// If 'default-owner' is set, [owner] will default to it.
        /// Same goes for 'default-repo'.
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
        pub group: Option<String>,
    }

    impl GitHubDependency {
        pub fn parts(&self) -> Result<_DependencyParts, SinkError> {
            match self.dependency.matches('/').count() {
                0 | 2 => Ok(match self.dependency.rsplit_once('/') {
                    Some(parts) => {
                        _DependencyParts(Some(String::from(parts.0)), String::from(parts.1))
                    }
                    None => _DependencyParts(None, self.dependency.clone()),
                }),
                c => Err(SinkError::Any(anyhow::anyhow!(
                    "Invalid number ({c}) of '/' in dependency!"
                ))),
            }
        }
    }

    pub struct _DependencyParts(pub Option<String>, pub String);
}

/* ---------- [ TOML ] ---------- */
pub mod toml {
    use serde::{Deserialize, Serialize};

    use crate::PluginOptions;

    #[derive(Serialize, Deserialize, Debug, Default)]
    #[serde(rename_all(deserialize = "kebab-case", serialize = "snake_case"))]
    pub struct GitHubPluginOptions {
        #[serde(flatten)]
        pub sink_options: Option<PluginOptions>,

        pub default_repository: Option<String>,
    }
}

/* ---------- [ Functions ] ---------- */

mod functions {
    use anyhow::Result;
    use log::{error, info};

    extern crate toml as ex_toml;

    use super::cli;
    use super::toml;
    use crate::{PluginOptions, SinkError, SinkTOML};

    const KEY_REPOSITORY: &str = "repository";
    const KEY_VERSION: &str = "version";
    const KEY_DESTINATION: &str = "destination";

    /// Add a dependency.
    pub fn add(mut sink_toml: SinkTOML, dependency: &cli::GitHubDependency) -> SinkTOML {
        info!("Adding {}@{}...", dependency.dependency, dependency.version);

        let github_options = sink_toml
            .github
            .get_or_insert(toml::GitHubPluginOptions::default());

        // Validate the dependency name structure
        // It should contain either 0 or 2 slashes
        // If it contains 0 slashes, default-repository **must** be set
        let parts = match dependency.parts() {
            Ok(value) => value,
            Err(ref e) => {
                error!("{e}");
                return sink_toml;
            }
        };
        let repository = match parts.0 {
            Some(ref value) => value,
            None => match github_options.default_repository {
                Some(ref value) => value,
                None => {
                    error!("No repository provided and default-repository is also not set!");
                    return sink_toml;
                }
            },
        };
        let dependency_name = parts.1;

        info!("Format check passed!");

        // Check if it can be installed
        if let Err(download_error) = download(dependency) {
            error!(
                "{}",
                SinkError::Any(
                    download_error.context(format!("Failed to add '{}'!", dependency.dependency)),
                )
            );
            return sink_toml;
        }

        // Add to sink TOML
        github_options
            .sink_options
            .get_or_insert(PluginOptions::default())
            .dependencies
            .insert(dependency_name.clone(), {
                let mut new_table = ex_toml::Table::new();
                new_table.insert(
                    String::from(KEY_REPOSITORY),
                    ex_toml::Value::String(repository.clone()),
                );
                new_table.insert(
                    String::from(KEY_VERSION),
                    ex_toml::Value::String(dependency.version.clone()),
                );
                new_table.insert(
                    String::from(KEY_DESTINATION),
                    ex_toml::Value::String(dependency.destination.display().to_string()),
                );

                new_table
            });

        sink_toml.formatted["GitHub"]["dependencies"][dependency_name] = toml_edit::Item::Table({
            let mut new_table = toml_edit::Table::new();
            new_table.insert(KEY_REPOSITORY, toml_edit::value(repository.clone()));
            new_table.insert(KEY_VERSION, toml_edit::value(dependency.version.clone()));
            new_table.insert(
                KEY_DESTINATION,
                toml_edit::value(dependency.destination.display().to_string()),
            );

            new_table
        });

        info!("Added {}!", dependency.dependency);

        sink_toml
    }

    pub fn download(dependency: &cli::GitHubDependency) -> Result<()> {
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
