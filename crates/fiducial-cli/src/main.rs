use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod config;
mod guard;
mod lock;

#[derive(Parser)]
#[command(
    name = "fid",
    about = "Fiducial — one declaration, many derivations",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scaffold a new product repository
    New {
        /// Product name (becomes the directory name and package identifier)
        name: String,
    },
    /// Add an app or module to the current product
    Add {
        #[command(subcommand)]
        target: commands::add::AddTarget,
    },
    /// Check for drift: outdated deps, stale templates, un-applied migrations
    Doctor,
    /// Shell-aware guard hook — reads PreToolUse JSON from stdin, exits non-zero to block
    #[command(hide = true)]
    GuardCheck,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::New { name } => commands::new::run(&name),
        Commands::Add { target } => commands::add::run(target),
        Commands::Doctor => commands::doctor::run(),
        Commands::GuardCheck => guard::check_from_stdin(),
    }
}
