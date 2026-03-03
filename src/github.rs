use std::str::FromStr;

use bytes::Bytes;
use reqwest::{Client, Url};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Release {
    pub tag_name: String,
    pub tarball_url: String,
}

pub async fn fetch_latest_release(
    owner: &str,
    repo: &str,
) -> Result<(String, Bytes), Box<dyn std::error::Error>> {
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

    Ok((release.tag_name, tarball))
}
