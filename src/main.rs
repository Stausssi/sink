use std::collections::HashMap;
use std::error::Error;
use std::fs::{self};
use std::{error, fmt, io};

use env_logger::Env;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use toml::Table;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let sink_toml = SinkTOML::from_file("docs/sink_example.toml");

    if let Err(sink_err) = sink_toml {
        error!("Failed to parse sink TOML: {sink_err}");
        return;
    }

    info!("Loaded sink TOML!");
}

#[derive(Debug)]
enum SinkParseError {
    IOError(io::Error),
    TOMLError(toml::de::Error),
}
impl fmt::Display for SinkParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            SinkParseError::IOError(ref e) => write!(f, "{e}"),
            SinkParseError::TOMLError(ref e) => write!(f, "{e}"),
        }
    }
}
impl error::Error for SinkParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self)
    }
}
impl From<io::Error> for SinkParseError {
    fn from(err: io::Error) -> SinkParseError {
        SinkParseError::IOError(err)
    }
}
impl From<toml::de::Error> for SinkParseError {
    fn from(err: toml::de::Error) -> SinkParseError {
        SinkParseError::TOMLError(err)
    }
}
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
        debug!("{:#?}", sink_toml);

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
