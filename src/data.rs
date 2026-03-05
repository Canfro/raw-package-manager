use std::{
    fs::{read_to_string, write},
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::error::GrmError;

#[derive(Serialize, Deserialize, Debug)]
pub struct PackageData {
    pub owner: String,
    pub repo: String,
    pub installed_version: String,
}

pub fn save_data(package_data: PackageData, data_root: &Path) -> Result<(), GrmError> {
    let json_file = data_root.join(format!("{}-{}.json", package_data.owner, package_data.repo));
    let json_string = serde_json::to_string_pretty(&package_data)?;

    write(json_file, json_string)?;

    Ok(())
}

pub fn load_data(owner: &str, repo: &str, data_root: &Path) -> Result<PackageData, GrmError> {
    let json_file = data_root.join(format!("{}-{}.json", owner, repo));
    let json_string = read_to_string(json_file)?;

    Ok(serde_json::from_str::<PackageData>(json_string.as_str())?)
}
