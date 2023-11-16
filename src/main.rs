use clap::Parser;
use env_logger::Env;
use log::{debug, error, info};
use sink::SinkTOML;
mod cli;

fn main() {
    let cli = cli::SinkCLI::parse();

    // Initialize logger
    {
        let logger_env =
            Env::default().default_filter_or(if cli.verbose { "debug" } else { "info" });
        env_logger::Builder::from_env(logger_env).init();
    }

    // Load sink TOML
    let path = "docs/sink_example.toml";
    let sink_toml = SinkTOML::from_file(path);

    if let Err(sink_err) = sink_toml {
        error!("{sink_err}");
        return;
    }

    let sink_toml = sink_toml.unwrap();
    debug!("Loaded sink TOML from '{path}'!");

    match &cli.command {
        cli::SinkSubcommands::Config(params) => {
            if params.all {
                info!("{:#?}", sink_toml);
            } else if params.toml {
                info!("{}", sink_toml.to_string());
            }
        }
        cli::SinkSubcommands::Install(params) => {
            info!("{:#?}", params);
        }
        cli::SinkSubcommands::GitHub(params) => {
            info!("{:#?}", params);
        }
    }
}
