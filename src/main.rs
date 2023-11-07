use std::collections::HashMap;
use std::fs::{self};
use std::{fmt, io};

use clap::{Args, Parser, Subcommand};
use env_logger::Env;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use toml::Table;

fn main() {
    let cli = SinkCLI::parse();

    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or(if cli.verbose {
        "debug"
    } else {
        "info"
    }))
    .init();

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
        SinkSubcommands::Config(config) => {
            if config.show {
                info!("{:#?}", sink_toml);
            }
        }
    }
}

/* ---------- [ CLI ] ---------- */

#[derive(Parser)]
#[command(author, version, about, long_about = None )]
#[command(help_expected = true)]
struct SinkCLI {
    #[command(subcommand)]
    command: SinkSubcommands,

    /// Enable verbose (debug) output.
    ///
    /// This flag will set the default log level from 'info' to 'debug'.
    /// TODO: Don't allow passing solely this flag
    #[arg(long, short, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum SinkSubcommands {
    /// Interact with the sink TOML file
    Config(SubcommandConfig),
}

#[derive(Args)]
#[command(arg_required_else_help = true)]
struct SubcommandConfig {
    /// Print the current sink TOML.
    ///
    /// This will print the currently loaded sink TOML with all 'includes' resolved.
    #[arg(short, long)]
    show: bool,
}

/* ---------- [ Errors ] ---------- */

#[derive(Debug)]
struct SinkParseError {
    reason: SinkParseErrorTypes,
}

#[derive(Debug)]
enum SinkParseErrorTypes {
    IOError(io::Error),
    TOMLError(toml::de::Error),
}

impl fmt::Display for SinkParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to parse sink TOML! Caused by: '{}'", self.reason)
    }
}
impl fmt::Display for SinkParseErrorTypes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::IOError(ref e) => write!(f, "{e}"),
            Self::TOMLError(ref e) => write!(f, "{e}"),
        }
    }
}

impl From<io::Error> for SinkParseError {
    fn from(err: io::Error) -> SinkParseError {
        SinkParseError {
            reason: SinkParseErrorTypes::IOError(err),
        }
    }
}
impl From<toml::de::Error> for SinkParseError {
    fn from(err: toml::de::Error) -> SinkParseError {
        SinkParseError {
            reason: SinkParseErrorTypes::TOMLError(err),
        }
    }
}

/* ---------- [ TOML ] ---------- */

#[derive(Serialize, Deserialize, Debug)]
#[serde(
    rename_all(deserialize = "kebab-case", serialize = "snake_case"),
    deny_unknown_fields
)]
struct SinkTOML {
    #[serde(default)]
    includes: Vec<String>,
    default_group: Option<String>,

    #[serde(rename = "Python")]
    python: Option<PythonPluginOptions>,
    #[serde(rename = "Rust")]
    rust: Option<RustPluginOptions>,
    #[serde(rename = "GitHub")]
    github: Option<GitHubPluginOptions>,
}

impl SinkTOML {
    fn from_file(path: &str) -> Result<SinkTOML, SinkParseError> {
        debug!("Parsing sink TOML from '{path}'...");

        let string_contents = fs::read_to_string(path)?;

        // TODO: Set mutable if config merge is implemented
        let sink_toml: SinkTOML = toml::from_str(&string_contents)?;

        // Extend with all files listed in include
        for include_path in sink_toml.includes.iter() {
            let included = SinkTOML::from_file(include_path);

            if included.is_err() {
                warn!(
                    "Failed to include '{include_path}': {}",
                    included.unwrap_err()
                );
                continue;
            }

            // TODO: Implement merge
            info!("Including {include_path}...");
        }

        debug!("Parsing done!");

        Ok(sink_toml)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "kebab-case", serialize = "snake_case"))]
struct PluginOptions {
    provider: Option<String>,
    default_group: Option<String>,

    #[serde(flatten)]
    dependencies: HashMap<String, Table>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "kebab-case", serialize = "snake_case"))]
struct PythonPluginOptions {
    #[serde(flatten)]
    sink_options: Option<PluginOptions>,

    version: String,
    venv: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "kebab-case", serialize = "snake_case"))]
struct RustPluginOptions {
    #[serde(flatten)]
    sink_options: Option<PluginOptions>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(deserialize = "kebab-case", serialize = "snake_case"))]
struct GitHubPluginOptions {
    #[serde(flatten)]
    sink_options: Option<PluginOptions>,

    default_repository: Option<String>,
}
