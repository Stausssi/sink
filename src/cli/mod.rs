use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::github;

#[derive(Parser)]
#[command(author, version, about, long_about = None )]
#[command(help_expected = true)]
pub struct SinkCLI {
    #[command(subcommand)]
    pub command: SinkSubcommands,

    /// Enable verbose (debug) output.
    ///
    /// This flag will set the default log level from 'info' to 'debug'.
    /// TODO: Don't allow passing solely this flag
    #[arg(long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
#[command(arg_required_else_help = true)]
pub enum SinkSubcommands {
    /// Interact with the sink TOML file
    Config(SubcommandConfig),

    /// Install dependencies
    Install(SubcommandInstall),

    /// Manage GitHub dependencies
    #[command(subcommand, name = "github", alias = "gh")]
    GitHub(github::cli::SubcommandGitHub),
}

#[derive(Args)]
#[command(arg_required_else_help = true)]
pub struct SubcommandConfig {
    /// Print the current sink TOML as a structure.
    ///
    /// This will print the currently loaded sink TOML with all 'includes' resolved.
    #[arg(short, long)]
    pub all: bool,

    /// Print the current sink TOML as a TOML.
    ///
    /// This will print the currently loaded sink TOML with all 'includes' resolved.
    #[arg(short, long)]
    pub toml: bool,

    /// List a specific type of entry contained in the sink TOML.
    #[arg(value_enum, short, long)]
    pub list: Option<ConfigListOptions>,

    /// Show a singular field by identifier.
    ///
    /// The identifier is expected to a '.' separated path to the field inside the sink TOML.
    #[arg(short, long)]
    pub field: Option<String>,

    /// Update the value of a config field.
    ///
    /// Expects a key=value pairing. This is NOT intended to be used on dependencies.
    #[arg(short, long)]
    pub update: Option<String>,
}

#[derive(ValueEnum, Clone)]
pub enum ConfigListOptions {
    /// Shows all groups (Dev, Prod, etc.).
    Groups,

    /// Shows all languages (Python, Rust, ...).
    Languages,

    /// Shows all dependencies independent of group and language.
    Dependencies,
}

#[derive(Args, Debug)]
#[command(arg_required_else_help = true)]
pub struct SubcommandInstall {
    /// Install all dependencies.
    ///
    /// Regardless of group and language.
    #[arg(short, long)]
    pub all: bool,

    /// Install only dependencies of a specific language.
    ///
    /// Can be combined with --group
    #[arg(value_enum, short, long)]
    pub lang: Option<Languages>,

    /// Install only a specific group of dependencies.
    ///
    /// Available groups are determined case-insensitive at runtime. Can be combined with --lang.
    #[arg(short, long)]
    pub group: Option<String>,

    /// Install based on sink.lock.
    ///
    /// Recommended to be used for reproducible builds.
    #[arg(short, long)]
    pub sink: bool,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum Languages {
    /// Python. Alias: 'py'
    #[value(alias = "py")]
    Python,

    /// Rust. Alias: 'rs'
    #[value(alias = "rs")]
    Rust,

    /// GitHub. Alias: 'gh'
    #[value(name = "github", alias = "gh")]
    GitHub,
}
