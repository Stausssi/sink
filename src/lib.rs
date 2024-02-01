pub mod cli;
pub mod github;

pub use errors::SinkError;
pub use toml::{PluginOptions, SinkTOML};

pub trait SinkDependency: Clone {}

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
    use log::{debug, info, warn, error};
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::fs::{self};
    use std::path::{Path, PathBuf};
    use toml::Table;
    use toml_edit::{self, Document};


    use super::SinkDependency;

    use super::errors::SinkError;
    use super::github;

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(
        rename_all(deserialize = "kebab-case", serialize = "snake_case"),
        deny_unknown_fields
    )]
    pub struct SinkTOML {
        #[serde(default)]
        pub includes: Vec<String>,
        pub default_group: Option<String>,

        #[serde(rename = "Python")]
        pub python: Option<PythonPluginOptions>,
        #[serde(rename = "Rust")]
        pub rust: Option<RustPluginOptions>,
        #[serde(rename = "GitHub")]
        pub github: Option<github::toml::GitHubPluginOptions>,

        /// Contains the path to the read sink TOML
        #[serde(skip)]
        pub path: PathBuf,

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
                error!("Including is not yet implemented!");
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

        pub fn _extend_formatted(
            &mut self,
            plugin_name: &str,
            group: Option<String>,
            dependency_key: &str,
            value: toml_edit::Item,
        ) {
            match group {
                Some(group) => {
                    self.formatted[plugin_name][group]["dependencies"][dependency_key] = value;
                }
                None => {
                    self.formatted[plugin_name]["dependencies"][dependency_key] = value;
                }
            }
        }

        pub fn add_dependency(
            &mut self,
            plugin_name: &str,
            group: Option<&String>,
            dependency: Dependency<github::GitHubDependency>,
            dependency_key: &str,
            formatted_value: toml_edit::Item,
        ) -> Result<()> {
            let sink_options = &mut self.github.as_mut().unwrap().sink_options;

            // The group is determined in this order:
            // 1. Given via argument
            // 2. Default group of GitHub
            // 3. Default group of global TOML
            let dependency_group =
                group.or(sink_options.default_group.as_ref().or(self.default_group.as_ref()));

            // Create object to reference later on
            let new_container = DependencyContainer {
                includes: None,
                dependencies: {
                    let mut new_map = HashMap::new();
                    new_map.insert(String::from(dependency_key), dependency.clone());
                    new_map
                },
            };

            // This decides how the new value will be added to the formatted contents
            let mut formatted_group: Option<String> = None;

            match &mut sink_options.dependencies {
                // Dependencies already exist
                Some(dependency_type) => match dependency_type {

                    // Existing dependencies are grouped
                    DependencyType::Grouped(grouped_containers) => 
                        match dependency_group {

                            // Group was given via the CLI or default-group is set
                            Some(final_group) =>{ 
                                formatted_group = Some(final_group.clone());

                                match grouped_containers.get_mut(final_group) {
    
                                    // Given group already exists
                                    Some(existing_dependencies) => {
                                        if existing_dependencies.dependencies.get(dependency_key).is_some() {
                                            return Err(anyhow::anyhow!("Dependency '{dependency_key}' already exists!"))
                                        }
                                        existing_dependencies.dependencies.insert(String::from(dependency_key), dependency.clone());
                                    },
        
                                    // Given group does not exist and has to be created anew
                                    None => {
                                        grouped_containers.insert(String::from(final_group), new_container);
                                    }
                                };
                            },
    
                            // No group given via CLI and no default-group exists in any scope
                            None => return Err(anyhow::anyhow!("Cannot add ungrouped dependency to grouped dependencies if default-group is not set!"))
                        }
                    
    
                    // Dependencies are not grouped
                    DependencyType::Singular(container) => match container.dependencies.get(dependency_key) {

                        // Dependency already exists
                        Some(_) => return Err(anyhow::anyhow!("Dependency '{dependency_key}' already exists!")),

                        // Dependency does not exist yet
                        None => match dependency_group {

                            // Adding a grouped dependency to singular does not work
                            Some(_) => return Err(anyhow::anyhow!("Adding grouped dependencies to singular dependencies are not supported yet!")),

                            // Add as singular dependency
                            None => {
                                container.dependencies.insert(String::from(dependency_key), dependency.clone());
                            }
                        }
                    }
    
                    // Invalid dependency structure in TOML
                    DependencyType::Invalid(_) => {
                        return Err(anyhow::anyhow!("Current {plugin_name} configuration contains invalid entries. Please fix them before adding new ones!"))
                    }
                },
    
                // This is the first dependency
                None => sink_options.dependencies = Some(
                    match dependency_group {
                        // Create grouped dependency container
                        Some(group) => {
                            let mut new_map = HashMap::new();
                            new_map.insert(String::from(group), new_container);
                            
                            formatted_group = Some(group.clone());
                            
                            DependencyType::Grouped(new_map)
                        }
                        // Create singular dependency container
                        None => {
                            DependencyType::Singular(new_container)
                        }
                    }
                )
            }

            self._extend_formatted(
                plugin_name,
                formatted_group,
                dependency_key,
                formatted_value,
            );
            Ok(())
        }
    }
    impl ToString for SinkTOML {
        fn to_string(&self) -> String {
            self.formatted.to_string()
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all(deserialize = "kebab-case", serialize = "snake_case"))]
    pub struct PluginOptions<T: SinkDependency> {
        pub provider: Option<String>,
        pub default_group: Option<String>,

        #[serde(flatten)]
        pub dependencies: Option<DependencyType<T>>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(untagged)]
    pub enum DependencyType<T: SinkDependency> {
        Singular(DependencyContainer<T>),
        Grouped(HashMap<String, DependencyContainer<T>>),
        Invalid(Table),
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct DependencyContainer<T: SinkDependency> {
        pub includes: Option<String>,

        pub dependencies: HashMap<String, Dependency<T>>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(untagged)]
    pub enum Dependency<T: SinkDependency> {
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

    impl SinkDependency for Table {}
}
