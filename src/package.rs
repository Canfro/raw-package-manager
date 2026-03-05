use crate::{error::GrmError, github::fetch_latest_release};
use std::{
    fs::{create_dir_all, read_dir, read_to_string, remove_dir_all, remove_file, write},
    io::{Cursor, ErrorKind},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
};

use flate2::read::GzDecoder;
use tar::Archive;

use crate::{
    config::{PackageConfig, load_config, save_config},
    data::{PackageData, load_data, save_data},
};

pub fn list_packages(data_root: &Path) -> Result<(), GrmError> {
    for res in read_dir(data_root)? {
        let data_file = res?.path();

        if let Ok(package_data) =
            serde_json::from_str::<PackageData>(read_to_string(data_file)?.as_str())
        {
            println!(
                "Owner: {}\nRepository: {}\nVersion: {}\n",
                package_data.owner, package_data.repo, package_data.installed_version
            );
        }
    }

    Ok(())
}

pub fn declare_package(
    owner: String,
    repo: String,
    config_root: &Path,
    data_root: &Path,
) -> Result<(), GrmError> {
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
    if load_data(owner.as_str(), repo.as_str(), data_root).is_ok() {
        println!(
            "Already existing data file: {}/{}-{}.json",
            data_root.display(),
            owner,
            repo
        );
    } else {
        save_data(
            PackageData {
                owner: owner.clone(),
                repo: repo.clone(),
                installed_version: "none".to_string(),
            },
            data_root,
        )?;
        println!(
            "Created missing state file: {}/{}-{}.json",
            data_root.display(),
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

    println!("Done!");
    Ok(())
}

pub async fn sync_package(
    owner: String,
    repo: String,
    data_root: &Path,
    config_root: &Path,
    cache_root: &Path,
) -> Result<(), GrmError> {
    // If package hasn't been declared, return
    if load_data(owner.as_str(), repo.as_str(), data_root).is_err() {
        return Err(format!(
            "Package '{}/{}' needs to be declared before sync, run 'grm declare {} {}'",
            owner, repo, owner, repo
        ))?;
    }

    let release = fetch_latest_release(&owner, &repo).await?;

    let cache_dir = cache_root.join(format!("{}-{}", owner, repo));

    if cache_dir.exists() {
        remove_dir_all(cache_dir.as_path())?;
    }
    create_dir_all(cache_dir.as_path())?;

    // Extract the tarball
    let mut archive = Archive::new(GzDecoder::new(Cursor::new(release.tarball_bytes)));
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
            .ok_or("Tarball did not contain a root directory".to_string())?
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
    save_data(
        PackageData {
            owner,
            repo,
            installed_version: release.tag_name,
        },
        data_root,
    )?;

    println!("Done!");
    Ok(())
}

pub fn remove_package(
    owner: String,
    repo: String,
    data_root: &Path,
    cache_root: &Path,
    config_root: &Path,
    config: bool,
) -> Result<(), GrmError> {
    let package_config = match load_config(owner.as_str(), repo.as_str(), config_root) {
        Ok(config) => config,
        Err(GrmError::Io(io_err)) if io_err.kind() == ErrorKind::NotFound => {
            return Err(GrmError::Custom(
                "No package configuration file exists, unable to remove package. Remove manually if desired.".to_string(),
            ));
        }
        Err(err) => return Err(err),
    };

    for binary_string in package_config.binaries_path {
        let binary_file = PathBuf::from_str(&binary_string)?;
        if !binary_file.exists() {
            continue;
        };

        let binary_status = Command::new("sudo")
            .arg("rm")
            .arg("-r")
            .arg(&binary_file)
            .status()?;
        if !binary_status.success() {
            return Err(GrmError::Custom(
                "Failed to remove file with sudo.".to_string(),
            ));
        }

        println!("File removed: {}", binary_file.display());
    }

    let data_file = data_root.join(format!("{}-{}.json", owner, repo));
    if data_file.exists() {
        remove_file(&data_file)?;
        println!("File removed: {}", data_file.display());
    }

    let cache_dir = cache_root.join(format!("{}-{}", owner, repo));
    if cache_dir.exists() {
        remove_dir_all(&cache_dir)?;
        println!("Directory removed: {}", cache_dir.display());
    }

    if config {
        let config_dir = config_root.join(format!("{}-{}", owner, repo));
        remove_dir_all(&config_dir)?;
        println!("Directory removed: {}", config_dir.display());
    }

    println!("Done!");
    Ok(())
}
