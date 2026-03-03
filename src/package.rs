use crate::github::fetch_latest_release;
use std::{
    fs::{create_dir_all, read_dir, read_to_string, remove_dir_all, remove_file, write},
    io::Cursor,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
};

use flate2::read::GzDecoder;
use tar::Archive;

use crate::{
    config::{PackageConfig, load_config, save_config},
    state::{PackageState, load_state, save_state},
};

pub fn list_packages(state_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
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

pub fn declare_package(
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

pub async fn sync_package(
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

    let release = fetch_latest_release(&owner, &repo).await?;
    let tag_name = release.0;
    let tarball = release.1;

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
            installed_version: tag_name,
        },
        state_root,
    )?;

    Ok(())
}

pub fn remove_package(
    owner: String,
    repo: String,
    state_root: &Path,
    cache_root: &Path,
    config_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let package_config = load_config(owner.as_str(), repo.as_str(), config_root)?;

    for binary_string in package_config.binaries_path {
        let binary_file = PathBuf::from_str(&binary_string)?;
        let binary_status = Command::new("sudo")
            .arg("rm")
            .arg("-r")
            .arg(binary_file)
            .status()?;

        if !binary_status.success() {
            return Err("Failed to remove file with sudo".into());
        }
    }

    remove_file(state_root.join(format!("{}-{}.json", owner, repo)))?;
    remove_dir_all(cache_root.join(format!("{}-{}", owner, repo)))?;

    Ok(())
}
