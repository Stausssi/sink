use log::{debug, info, warn};
use std::path::{Path, PathBuf};
use std::{fmt, io};

/* ---------- [ Errors ] ---------- */
#[derive(Debug)]
pub struct SinkParseError {
    reason: SinkParseErrorTypes,
}

#[derive(Debug)]
enum SinkParseErrorTypes {
    IOError(io::Error),
    TOMLError(toml::de::Error),
    TOMLEditError(toml_edit::TomlError),
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
            Self::TOMLEditError(ref e) => write!(f, "{e}"),
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
impl From<toml_edit::TomlError> for SinkParseError {
    fn from(err: toml_edit::TomlError) -> SinkParseError {
        SinkParseError {
            reason: SinkParseErrorTypes::TOMLEditError(err),
        }
    }
}

/* ---------- [ TOML ] ---------- */
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self};
use toml::Table;
use toml_edit::{self, Document};

#[derive(Serialize, Deserialize, Debug)]
#[serde(
    rename_all(deserialize = "kebab-case", serialize = "snake_case"),
    deny_unknown_fields
)]
pub struct SinkTOML {
    #[serde(default)]
    includes: Vec<String>,
    default_group: Option<String>,

    #[serde(rename = "Python")]
    python: Option<PythonPluginOptions>,
    #[serde(rename = "Rust")]
    rust: Option<RustPluginOptions>,
    #[serde(rename = "GitHub")]
    github: Option<GitHubPluginOptions>,

    /// Contains the path to the read sink TOML
    #[serde(skip)]
    path: PathBuf,

    /// Contains the formatted document for in-place manipulation
    #[serde(skip)]
    formatted: Document,
}

impl SinkTOML {
    pub fn from_file(path: &str) -> Result<SinkTOML, SinkParseError> {
        debug!("Parsing sink TOML from '{path}'...");

        let string_contents = fs::read_to_string(path)?;

        let mut sink_toml: SinkTOML = toml::from_str(&string_contents)?;
        sink_toml.path = Path::new(path).to_owned();
        sink_toml.formatted = string_contents.parse::<Document>()?;
        let sink_toml = sink_toml;

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

            info!("Including {include_path}...");

            // TODO: Implement merge
            info!("Including is not yet implemented!");
        }

        debug!("Parsing done!");

        Ok(sink_toml)
    }

    fn save(&self) {
        fs::write(&self.path, self.to_string()).unwrap();
    }
}
impl ToString for SinkTOML {
    fn to_string(&self) -> String {
        self.formatted.to_string()
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
