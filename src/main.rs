mod cli;
mod config;
mod github;
mod package;
mod state;

use std::{env, fs::create_dir_all, path::PathBuf};

use clap::Parser;

use crate::{
    cli::{Cli, Commands},
    package::{declare_package, list_packages, remove_package, sync_package},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let home = env::var("HOME")?;
    let state_root = PathBuf::from(&home).join(".local/state/github-repository-manager");
    let config_root = PathBuf::from(&home).join(".config/github-repository-manager");
    let cache_root = PathBuf::from(&home).join(".cache/github-repository-manager");

    create_dir_all(state_root.as_path())?;
    create_dir_all(config_root.as_path())?;
    create_dir_all(cache_root.as_path())?;

    let cli = Cli::parse();

    match cli.command {
        Commands::List => list_packages(state_root.as_path())?,
        Commands::Declare { owner, repo } => {
            declare_package(owner, repo, config_root.as_path(), state_root.as_path())?
        }
        Commands::Sync { owner, repo } => {
            sync_package(
                owner,
                repo,
                state_root.as_path(),
                config_root.as_path(),
                cache_root.as_path(),
            )
            .await?
        }
        Commands::Remove { owner, repo } => remove_package(
            owner,
            repo,
            state_root.as_path(),
            cache_root.as_path(),
            config_root.as_path(),
        )?,
    }

    Ok(())
}
