mod cli;
mod config;
mod data;
mod error;
mod github;
mod package;

use std::fs::create_dir_all;

use clap::Parser;
use directories::BaseDirs;

use crate::{
    cli::{Cli, Commands},
    error::GrmError,
    package::{declare_package, list_packages, remove_package, sync_package},
};

#[tokio::main]
async fn main() -> Result<(), GrmError> {
    let base_dirs = BaseDirs::new().ok_or(GrmError::Custom(
        "Failed to get user base directories".to_string(),
    ))?;
    let data_root = base_dirs.data_dir().join("github-repository-manager");
    let config_root = base_dirs.config_dir().join("github-repository-manager");
    let cache_root = base_dirs.cache_dir().join("github-repository-manager");

    create_dir_all(data_root.as_path())?;
    create_dir_all(config_root.as_path())?;
    create_dir_all(cache_root.as_path())?;

    let cli = Cli::parse();

    match cli.command {
        Commands::List => list_packages(data_root.as_path())?,
        Commands::Declare { owner, repo } => {
            declare_package(owner, repo, config_root.as_path(), data_root.as_path())?
        }
        Commands::Sync { owner, repo } => {
            sync_package(
                owner,
                repo,
                data_root.as_path(),
                config_root.as_path(),
                cache_root.as_path(),
            )
            .await?
        }
        Commands::Remove {
            owner,
            repo,
            config,
        } => remove_package(
            owner,
            repo,
            data_root.as_path(),
            cache_root.as_path(),
            config_root.as_path(),
            config,
        )?,
    }

    Ok(())
}
