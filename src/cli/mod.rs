use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None )]
#[command(help_expected = true)]
pub struct SinkCLI {
    #[command(subcommand)]
    pub command: SinkSubcommands,

    /// Enable verbose (debug) output.
    ///
    /// This flag will set the default log level from ``info`` to ``debug``.
    /// TODO: Don't allow passing solely this flag
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Path to the sink TOML file to use.
    ///
    /// If not provided, it will look for a ``sink.toml`` in the current directory.
    #[arg(short, long, global = true)]
    pub file: Option<String>,
}

#[derive(Subcommand)]
#[command(arg_required_else_help = true)]
pub enum SinkSubcommands {
    /// Interact with the sink TOML file
    Config(SubcommandConfig),

    /// Install dependencies
    Install(SubcommandInstall),

    /// Add dependencies
    Add(SubcommandAdd),

    /// Remove dependencies
    Remove(SubcommandRemove),
}

#[derive(Args)]
#[command(arg_required_else_help = true)]
pub struct SubcommandConfig {
    /// Print the current sink TOML as a structure.
    ///
    /// This will print the currently loaded sink TOML with all ``includes`` resolved.
    #[arg(short, long)]
    pub all: bool,

    /// Print the current sink TOML as a TOML.
    ///
    /// This will print the currently loaded sink TOML with all ``includes`` resolved.
    #[arg(short, long)]
    pub toml: bool,

    /// List all dependencies.
    #[arg(short, long)]
    pub list: bool,

    /// Update the value of a config field.
    ///
    /// Expects a ``key=value`` pairing.
    /// This is **not** intended to be used on dependencies.
    #[arg(short, long)]
    pub update: Option<String>,
}

#[derive(Args, Debug)]
#[command(arg_required_else_help = true)]
pub struct SubcommandInstall {
    /// Install based on ``sink.lock``.
    ///
    /// Recommended to be used for reproducible builds.
    #[arg(short, long)]
    pub sink: bool,
}

#[derive(Args, Debug)]
#[command(arg_required_else_help = true)]
pub struct SubcommandAdd {
    /// The dependency to add.
    ///
    /// Supposed to be in the form of 'owner/repository:dependency'.
    /// The 'owner/repository' part will default to the default owner and repository, if set.
    /// TODO: Use an enum for this
    dependency: String,

    /// The local destination to download the dependency to.
    ///
    /// This is relative to the directory the 'sink.toml' is in.
    #[arg(short, long = "dest", long = "destination")]
    destination: Option<String>,

    /// The version to download.
    ///
    /// This is the git tag to download.
    /// TODO: Test if version works with the default version flag
    #[arg(short, long)]
    version: Option<String>,

    /// Whether to skip adding the downloaded asset(s) to the gitignore.
    ///
    /// Defaults to false.
    #[arg(long)]
    no_gitignore: bool,
}

#[derive(Args, Debug)]
#[command(arg_required_else_help = true)]
pub struct SubcommandRemove {
    /// The dependency to remove.
    ///
    /// **Must** to be in the form of 'owner/repository:dependency'.
    /// TODO: Use an enum for this
    dependency: String,
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    SinkCLI::command().debug_assert();
}
