pub mod cli;
pub mod github;

pub use errors::SinkError;
pub use toml::{PluginOptions, SinkTOML};

/* ---------- [ Errors ] ---------- */
pub mod errors {
    use std::fmt::Display;

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
}

/* ---------- [ TOML ] ---------- */
pub mod toml {
    use anyhow::Result;
    use log::{debug, info, warn};
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::fs::{self};
    use std::path::{Path, PathBuf};
    use toml::Table;
    use toml_edit::{self, Document};

    use super::errors::SinkError;
    use super::github;

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
        pub python: Option<PythonPluginOptions>,
        #[serde(rename = "Rust")]
        pub rust: Option<RustPluginOptions>,
        #[serde(rename = "GitHub")]
        pub github: Option<github::toml::GitHubPluginOptions>,

        /// Contains the path to the read sink TOML
        #[serde(skip)]
        path: PathBuf,

        /// Contains the formatted document for in-place manipulation
        #[serde(skip)]
        pub formatted: Document,
    }
    impl SinkTOML {
        fn _validate(&self) -> Result<()> {
            if let Some(github_options) = &self.github {
                if let Some(DependencyType::Invalid(invalid)) =
                    &github_options.sink_options.dependencies
                {
                    return Err(anyhow::anyhow!("Invalid GitHub dependencies! {invalid:#?}"));
                }
            }
            Ok(())
        }

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

            // Check for invalid entries
            sink_toml._validate()?;

            debug!("Parsing done!");

            Ok(sink_toml)
        }
        pub fn from_file(path: &str) -> Result<SinkTOML, SinkError> {
            match SinkTOML::_from_file(path) {
                Ok(sink_toml) => Ok(sink_toml),
                Err(e) => Err(SinkError::Any(e.context("Failed to parse Sink TOML!"))),
            }
        }
    }
    impl ToString for SinkTOML {
        fn to_string(&self) -> String {
            self.formatted.to_string()
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all(deserialize = "kebab-case", serialize = "snake_case"))]
    pub struct PluginOptions<TDependency> {
        pub provider: Option<String>,
        pub default_group: Option<String>,

        #[serde(flatten)]
        pub dependencies: Option<DependencyType<TDependency>>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(untagged)]
    pub enum DependencyType<T> {
        Singular(DependencyContainer<T>),
        Grouped(HashMap<String, DependencyContainer<T>>),
        Invalid(Table),
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct DependencyContainer<T> {
        pub includes: Option<String>,

        pub dependencies: HashMap<String, Dependency<T>>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(untagged)]
    pub enum Dependency<T> {
        Version(String),
        Full(T),
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all(deserialize = "kebab-case", serialize = "snake_case"))]
    pub struct PythonPluginOptions {
        #[serde(flatten)]
        sink_options: PluginOptions<Table>,

        version: String,
        venv: Option<String>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all(deserialize = "kebab-case", serialize = "snake_case"))]
    pub struct RustPluginOptions {
        #[serde(flatten)]
        sink_options: PluginOptions<Table>,
    }
}
