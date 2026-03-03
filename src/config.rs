use std::{
    fs::{read_to_string, write},
    path::Path,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PackageConfig {
    pub binaries_path: Vec<String>,
}

pub fn save_config(
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

pub fn load_config(
    owner: &str,
    repo: &str,
    config_root: &Path,
) -> Result<PackageConfig, Box<dyn std::error::Error>> {
    let config_dir = config_root.join(format!("{}-{}", owner, repo));
    let json_file = config_dir.join(format!("{}-{}.json", owner, repo));
    let json_string = read_to_string(json_file)?;

    Ok(serde_json::from_str::<PackageConfig>(json_string.as_str())?)
}
