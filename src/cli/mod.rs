use clap::{Args, Parser, Subcommand};

use crate::github;

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
#[command(arg_required_else_help = false)]
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
    pub dependency: String,

    /// The local destination to download the dependency to.
    ///
    /// This is relative to the directory the 'sink.toml' is in.
    #[arg(short, long, alias = "dest")]
    pub destination: Option<String>,

    /// The version to download.
    ///
    /// Defaults to 'latest'.
    ///
    /// Possible values: ['latest', 'prerelease', specific tag (e.g. 'v1.0.0')]
    #[arg(short, long, value_parser = github::GitHubVersion::parse_cli)]
    pub version: Option<github::GitHubVersion>,

    /// Whether to skip adding the downloaded asset(s) to the gitignore.
    ///
    /// Defaults to false.
    #[arg(long)]
    pub no_gitignore: bool,

    /// Whether to add the dependency in the short form.
    ///
    /// This will add a single line with just the version to the dependencies.
    /// Conflicts with both 'destination' and 'no_gitignore'.
    /// TODO: Maybe determine this automatically?
    #[arg(long, conflicts_with_all = ["destination", "no_gitignore"])]
    pub short: bool,
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
