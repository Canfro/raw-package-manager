use std::{
    fs::{read_to_string, write},
    path::Path,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PackageState {
    pub owner: String,
    pub repo: String,
    pub installed_version: String,
}

pub fn save_state(package_state: PackageState, state_root: &Path) -> Result<(), std::io::Error> {
    let json_file = state_root.join(format!(
        "{}-{}.json",
        package_state.owner, package_state.repo
    ));
    let json_string = serde_json::to_string_pretty(&package_state)?;

    write(json_file, json_string)?;

    Ok(())
}
pub fn load_state(
    owner: &str,
    repo: &str,
    state_root: &Path,
) -> Result<PackageState, Box<dyn std::error::Error>> {
    let json_file = state_root.join(format!("{}-{}.json", owner, repo));
    let json_string = read_to_string(json_file)?;

    Ok(serde_json::from_str::<PackageState>(json_string.as_str())?)
}
