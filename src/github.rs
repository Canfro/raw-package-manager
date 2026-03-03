use std::str::FromStr;

use bytes::Bytes;
use reqwest::{Client, Url};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Release {
    pub tag_name: String,
    pub tarball_url: String,
}

pub struct FetchedRelease {
    pub tag_name: String,
    pub tarball_bytes: Bytes,
}

pub async fn fetch_latest_release(
    owner: &str,
    repo: &str,
) -> Result<FetchedRelease, Box<dyn std::error::Error>> {
    let release_url = Url::from_str(
        format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            owner, repo
        )
        .as_str(),
    )?;

    let client = Client::builder()
        .user_agent("github-repository-manager")
        .build()?;

    let release = client
        .get(release_url)
        .header("accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json::<Release>()
        .await?;

    let tarball = client
        .get(release.tarball_url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    Ok(FetchedRelease {
        tag_name: release.tag_name,
        tarball_bytes: tarball,
    })
}
