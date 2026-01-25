use std::{
    env,
    fs::{create_dir_all, read_dir, read_to_string, remove_dir_all, remove_file, write},
    io::Cursor,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
};

use clap::{Parser, Subcommand};
use flate2::read::GzDecoder;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use tar::Archive;

#[derive(Parser)]
#[command(name = "raw-package-manager")]
#[command(about = "Manage raw package instances")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Lists all installed packages
    List,

    /// Declares a package matching the provided owner and repository
    /// Build script and config is created in "~/.config/raw-ṕackage-manager/"
    /// Make sure to edit these files before sync
    Declare { owner: String, repo: String },

    /// Sync a package matching the provided owner and repository
    /// A package needs to be declared with "raw-package-manager declare" before sync
    Sync { owner: String, repo: String },

    /// Removes a package matching the provided owner and repository
    /// Note that any additional files created by the package won't be deleted
    Remove { owner: String, repo: String },
}

#[derive(Serialize, Deserialize, Debug)]
struct PackageState {
    owner: String,
    repo: String,
    installed_version: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct PackageConfig {
    binaries_path: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct Release {
    tag_name: String,
    tarball_url: String,
}

fn list_packages(state_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for res in read_dir(state_root)? {
        let state_file = res?.path();

        if let Ok(package_state) =
            serde_json::from_str::<PackageState>(read_to_string(state_file)?.as_str())
        {
            println!(
                "Owner: {}\nRepository: {}\nVersion: {}\n",
                package_state.owner, package_state.repo, package_state.installed_version
            );
        }
    }

    Ok(())
}

fn declare_package(
    owner: String,
    repo: String,
    config_root: &Path,
    state_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Write template to build script
    let config_dir = config_root.join(format!("{}-{}", owner, repo));
    create_dir_all(config_dir.as_path())?;
    let script_file = config_dir.join(format!("{}-{}.sh", owner, repo));

    if script_file.exists() {
        println!("Already existing build script: {}", script_file.display());
    } else {
        write(script_file.as_path(), include_str!("templates/build.sh"))?;
        println!("Created missing build script: {}", script_file.display());
    }

    // Write package state file
    if load_state(owner.as_str(), repo.as_str(), state_root).is_ok() {
        println!(
            "Already existing state file: {}/{}-{}.json",
            state_root.display(),
            owner,
            repo
        );
    } else {
        save_state(
            PackageState {
                owner: owner.clone(),
                repo: repo.clone(),
                installed_version: "none".to_string(),
            },
            state_root,
        )?;
        println!(
            "Created missing state file: {}/{}-{}.json",
            state_root.display(),
            owner,
            repo
        );
    }

    // Write package config file
    if load_config(owner.as_str(), repo.as_str(), config_root).is_ok() {
        println!(
            "Already existing config file: {}/{}-{}.json",
            config_dir.display(),
            owner,
            repo
        );
    } else {
        save_config(
            owner.as_str(),
            repo.as_str(),
            PackageConfig {
                binaries_path: Vec::new(),
            },
            config_root,
        )?;
        println!(
            "Created missing config file: {}/{}-{}.json",
            config_dir.display(),
            owner,
            repo
        );
    }

    Ok(())
}

async fn sync_package(
    owner: String,
    repo: String,
    state_root: &Path,
    config_root: &Path,
    cache_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // If package hasn't been declared, return
    if load_state(owner.as_str(), repo.as_str(), state_root).is_err() {
        return Err(format!(
            "Package '{}/{}' needs to be declared before sync, run 'raw-package-manager declare {} {}'",
            owner, repo, owner, repo
        ).into());
    }

    // Fetch the latest tag name and source code tarball from the GitHub repository
    let release_url = Url::from_str(
        format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            owner, repo
        )
        .as_str(),
    )?;

    let client = Client::builder()
        .user_agent("raw-package-manager")
        .build()?;
    let release = client
        .get(release_url)
        .header("accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json::<Release>()
        .await?;

    // Download the tarball
    let tarball = client
        .get(release.tarball_url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let cache_dir = cache_root.join(format!("{}-{}", owner, repo));

    if cache_dir.exists() {
        remove_dir_all(cache_dir.as_path())?;
    }
    create_dir_all(cache_dir.as_path())?;

    // Extract the tarball
    let mut archive = Archive::new(GzDecoder::new(Cursor::new(tarball)));
    archive.set_overwrite(true);
    archive.unpack(cache_dir.as_path())?;

    // Get the extracted subdirectory and replace in script
    let config_dir = config_root.join(format!("{}-{}", owner, repo));
    let script_file = config_dir.join(format!("{}-{}.sh", owner, repo));
    let script_content = read_to_string(script_file.as_path())?.replace(
        "{{SOURCE_CODE}}",
        read_dir(cache_dir.as_path())?
            .filter_map(|res| res.ok())
            .find(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .ok_or("Tarball did not contain a root directory")?
            .path()
            .to_str()
            .unwrap(),
    );

    write(script_file.as_path(), script_content)?;

    // Execute build script
    let script_status = Command::new("bash")
        .arg(script_file)
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .spawn()?
        .wait()?;

    if !script_status.success() {
        return Err(format!("Build script failed with status: {}", script_status).into());
    }

    // Update package state file
    save_state(
        PackageState {
            owner,
            repo,
            installed_version: release.tag_name,
        },
        state_root,
    )?;

    Ok(())
}

fn remove_package(
    owner: String,
    repo: String,
    state_root: &Path,
    cache_root: &Path,
    config_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let package_config = load_config(owner.as_str(), repo.as_str(), config_root)?;

    for binary_string in package_config.binaries_path {
        let binary_file = PathBuf::from_str(&binary_string)?;
        remove_file(binary_file)?;
    }

    remove_file(state_root.join(format!("{}-{}.json", owner, repo)))?;
    remove_dir_all(cache_root.join(format!("{}-{}", owner, repo)))?;

    Ok(())
}

fn save_state(package_state: PackageState, state_root: &Path) -> Result<(), std::io::Error> {
    let json_file = state_root.join(format!(
        "{}-{}.json",
        package_state.owner, package_state.repo
    ));
    let json_string = serde_json::to_string_pretty(&package_state)?;

    write(json_file, json_string)?;

    Ok(())
}

fn save_config(
    owner: &str,
    repo: &str,
    package_config: PackageConfig,
    config_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = config_root.join(format!("{}-{}", owner, repo));
    let json_file = config_dir.join(format!("{}-{}.json", owner, repo));
    let json_string = serde_json::to_string_pretty(&package_config)?;

    write(json_file, json_string)?;

    Ok(())
}

fn load_state(
    owner: &str,
    repo: &str,
    state_root: &Path,
) -> Result<PackageState, Box<dyn std::error::Error>> {
    let json_file = state_root.join(format!("{}-{}.json", owner, repo));
    let json_string = read_to_string(json_file)?;

    Ok(serde_json::from_str::<PackageState>(json_string.as_str())?)
}

fn load_config(
    owner: &str,
    repo: &str,
    config_root: &Path,
) -> Result<PackageConfig, Box<dyn std::error::Error>> {
    let config_dir = config_root.join(format!("{}-{}", owner, repo));
    let json_file = config_dir.join(format!("{}-{}.json", owner, repo));
    let json_string = read_to_string(json_file)?;

    Ok(serde_json::from_str::<PackageConfig>(json_string.as_str())?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let home = env::var("HOME")?;
    let state_root = PathBuf::from(&home).join(".local/state/raw-package-manager");
    let config_root = PathBuf::from(&home).join(".config/raw-package-manager");
    let cache_root = PathBuf::from(&home).join(".cache/raw-package-manager");

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
