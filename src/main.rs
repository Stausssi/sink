use std::path::PathBuf;

use clap::Parser;
use env_logger::Env;
use log::{debug, error, info};

extern crate sink;
use sink::cli;
use sink::github;
use sink::toml::DependencyType;
use sink::SinkError;
use sink::SinkTOML;

fn main() {
    let cli = cli::SinkCLI::parse();

    // Initialize logger
    {
        let logger_env =
            Env::default().default_filter_or(if cli.verbose { "debug" } else { "info" });
        env_logger::Builder::from_env(logger_env).init();
    }

    // Load sink TOML
    let mut path = PathBuf::from(&cli.file);
    if !path.exists() {
        debug!(
            "'{}' does not exist, failing back to 'docs/sink_example.toml'!",
            path.display()
        );
        path = PathBuf::from("docs/sink_example.toml");
    }
    let sink_toml = SinkTOML::from_file(&path);

    if let Err(sink_err) = sink_toml {
        error!("{sink_err}");
        return;
    }

    let sink_toml = sink_toml.unwrap();
    debug!("Loaded sink TOML from '{}'!", path.display());

    match cli.command {
        cli::SinkSubcommands::Config(params) => {
            if params.all {
                info!("{:#?}", sink_toml);
            } else if params.toml {
                info!("{}", sink_toml.to_toml());
            }
        }
        cli::SinkSubcommands::Install(_) => {
            for (pattern, dependency) in sink_toml.dependencies.iter() {
                let github_dependency = match dependency {
                    DependencyType::Full(github_dependency) => github_dependency,
                    DependencyType::Version(version) => &match github::GitHubDependency::new(
                        pattern.to_string(),
                        None,
                        Some(version.to_owned()),
                        true,
                        &None,
                    ) {
                        Ok(new_dependency) => new_dependency,
                        Err(e) => {
                            error!("{e}");
                            continue;
                        }
                    },
                    DependencyType::Invalid(_) => {
                        error!("Invalid dependency entry for '{}'!", pattern);
                        continue;
                    }
                };
                if let Err(e) = github::download(github_dependency) {
                    error!("{}", SinkError::Any(e));
                }
            }
        }
        cli::SinkSubcommands::Add(params) => {
            match github::GitHubDependency::new(
                params.dependency,
                params.destination,
                params.version,
                !params.no_gitignore,
                &sink_toml.default_owner,
            ) {
                Ok(dependency) => {
                    if let Err(e) = github::add(sink_toml, dependency, params.short) {
                        error!("{e}");
                    }
                }
                Err(sink_err) => {
                    error!("{sink_err}");
                }
            }
        }
        cli::SinkSubcommands::Remove(params) => {
            info!("{:#?}", params);
        }
    };
}
