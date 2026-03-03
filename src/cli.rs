use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "github-repository-manager")]
#[command(about = "Manage gitHub repositories locally")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Lists all installed packages
    List,

    /// Declares a package matching the provided owner and repository.
    /// Build script and config is created in "~/.config/github-repository-manager/".
    /// Make sure to edit these files before sync.
    Declare { owner: String, repo: String },

    /// Sync a package matching the provided owner and repository.
    /// A package needs to be declared with "grm declare" before sync.
    Sync { owner: String, repo: String },

    /// Removes a package matching the provided owner and repository.
    /// Note that any additional files created by the package won't be deleted.
    Remove { owner: String, repo: String },
}
