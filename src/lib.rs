pub mod cli;
pub mod github;

pub use errors::SinkError;
pub use toml::SinkTOML;

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
    use log::{debug, error, info, warn};
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::fs::{self};
    use std::path::PathBuf;
    use toml_edit::{self, DocumentMut};

    use super::errors::SinkError;
    use super::github;

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(
        rename_all(deserialize = "kebab-case", serialize = "snake_case"),
        deny_unknown_fields
    )]
    pub struct SinkTOML {
        /// Optional: The default owner to fall back to if not explicitly set
        pub default_owner: Option<String>,

        /// Optional: Collection of paths to other sink TOMLs to include.
        #[serde(default)]
        pub includes: Vec<PathBuf>,

        /// The actual dependencies.
        pub dependencies: HashMap<String, DependencyType>,

        /// Contains the path to the this sink TOML
        #[serde(skip)]
        pub path: PathBuf,

        /// Contains the formatted document for in-place manipulation and writing back to the file.
        #[serde(skip)]
        pub formatted: DocumentMut,
    }
    impl SinkTOML {
        /// Checks the TOML syntax.
        ///
        /// This fails, if any of the fields could not be parsed correctly.
        fn _validate_toml_syntax(&self) -> Result<()> {
            for (key, value) in self.dependencies.iter() {
                if let DependencyType::Invalid(_) = value {
                    return Err(anyhow::anyhow!("Invalid dependency entry for '{key}'!"));
                }
            }
            Ok(())
        }

        /// Validates the TOML semantics.
        ///
        /// This checks for missing owner specification, etc.
        fn _validate_toml_semantics(&self) -> Result<()> {
            let has_default_owner = self.default_owner.is_some();

            Ok(())
        }

        /// Validate the sink TOML.
        ///
        /// This performs basic checks, such as checking for TOML errors, missing specification, etc.
        fn _validate(&self) -> Result<()> {
            if let Err(e) = self._validate_toml_syntax() {
                return Err(e.context("Failed to parse TOML data!"));
            }

            if let Err(e) = self._validate_toml_semantics() {
                return Err(e.context("Failed to validate TOML data!"));
            }

            Ok(())
        }

        fn _from_file(path: &PathBuf) -> Result<SinkTOML> {
            debug!("Parsing sink TOML from '{}'...", path.display());

            let string_contents = fs::read_to_string(path.clone())?;

            let mut sink_toml: SinkTOML = toml::from_str(&string_contents)?;
            sink_toml.path = PathBuf::from(path);
            sink_toml.formatted = string_contents.parse::<DocumentMut>()?;
            let sink_toml = sink_toml;

            // Extend with all files listed in include
            for include_path in sink_toml.includes.iter() {
                let included = SinkTOML::from_file(include_path);

                if included.is_err() {
                    warn!(
                        "Failed to include '{}': {}",
                        include_path.display(),
                        included.unwrap_err()
                    );
                    continue;
                }

                info!("Including {}...", include_path.display(),);

                // TODO: Implement merge
                error!("Including is not yet implemented!");
            }

            // Check for invalid entries
            sink_toml._validate()?;

            debug!("Parsing done!");

            Ok(sink_toml)
        }
        /// Try loading a sink TOML from a file.
        pub fn from_file(path: &PathBuf) -> Result<SinkTOML, SinkError> {
            match SinkTOML::_from_file(path) {
                Ok(sink_toml) => Ok(sink_toml),
                Err(e) => Err(SinkError::Any(e.context("Failed to load Sink TOML!"))),
            }
        }

        /// Returns the TOML representation of the parsed file.
        pub fn to_toml(&self) -> String {
            self.formatted.to_string()
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(untagged)]
    pub enum DependencyType {
        /// Single line declaration with only the version
        Version(github::GitHubVersion),

        /// Full declaration with all fields specified
        Full(github::GitHubDependency),

        /// Catch all potential TOML mismatches
        Invalid(toml::Value),
    }
}
