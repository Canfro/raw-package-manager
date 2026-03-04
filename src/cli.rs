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
    Declare { owner: String, repo: String },

    /// Sync a package matching the provided owner and repository.
    Sync { owner: String, repo: String },

    /// Removes a package matching the provided owner and repository.
    Remove {
        owner: String,
        repo: String,

        /// Delete cofig files as well.
        #[arg(short, long)]
        config: bool,
    },
}
