use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;

use clap::Parser;
use env_logger::Env;
use log::{debug, error, info};

extern crate sink;
use sink::cli;
use sink::github;
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
    let path = &PathBuf::from(&cli.file.unwrap_or(String::from("docs/sink_example.toml")));
    let sink_toml = SinkTOML::from_file(path);

    if let Err(sink_err) = sink_toml {
        error!("{sink_err}");
        return;
    }

    let mut sink_toml = sink_toml.unwrap();
    debug!("Loaded sink TOML from '{}'!", path.display());

    match cli.command {
        cli::SinkSubcommands::Config(params) => {
            if params.all {
                info!("{:#?}", sink_toml);
            } else if params.toml {
                info!("{}", sink_toml.to_toml());
            }
        }
        cli::SinkSubcommands::Install(params) => {
            info!("{:#?}", params);
        }
        cli::SinkSubcommands::Add(params) => {
            info!("{:#?}", params);
        }
        cli::SinkSubcommands::Remove(params) => {
            info!("{:#?}", params);
        }
    }
}
