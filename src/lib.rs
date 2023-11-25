use anyhow::Result;
use log::{debug, info, warn};

use std::fmt::Display;
use std::path::{Path, PathBuf};

/* ---------- [ Errors ] ---------- */

/// Wrapper around anyhow::Error to allow for custom Display trait
#[derive(Debug)]
pub enum SinkError {
    Any(anyhow::Error),
}
impl Display for SinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self::Any(as_error) = self;
        let mut error_string = as_error.to_string();
        as_error
            .chain()
            .skip(1)
            .for_each(|cause| error_string.push_str(&format!(" Caused by: {}", cause)));
        write!(f, "{error_string}")
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
    fn _from_file(path: &str) -> Result<SinkTOML> {
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

    pub fn from_file(path: &str) -> Result<SinkTOML, SinkError> {
        let internal_result = SinkTOML::_from_file(path);

        if let Err(err_result) = internal_result {
            return Err(SinkError::Any(
                err_result.context("Failed to parse Sink TOML!"),
            ));
        }
        Ok(internal_result.unwrap())
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
