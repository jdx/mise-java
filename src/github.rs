use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use xx::regex;

use crate::http::HTTP;
use eyre::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub assets: Vec<GitHubAsset>,
    pub body: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    pub tag_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubTag {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubAsset {
    pub browser_download_url: String,
    pub content_type: String,
    pub name: String,
    pub size: u64,
}

pub fn list_releases(repo: &str) -> Result<Vec<GitHubRelease>> {
    let url = format!("https://api.github.com/repos/{repo}/releases?per_page=100");

    let (mut releases, mut headers) = HTTP.get_json_with_headers::<Vec<GitHubRelease>, _>(url)?;

    while let Some(next) = next_page(&headers) {
        let (more, h) = HTTP.get_json_with_headers::<Vec<GitHubRelease>, _>(&next)?;
        releases.extend(more);
        headers = h;
    }
    releases.retain(|r| !r.draft && !r.prerelease);

    Ok(releases)
}

fn next_page(headers: &HeaderMap) -> Option<String> {
    let link = headers
        .get("link")
        .map(|l| l.to_str().unwrap_or_default().to_string())
        .unwrap_or_default();
    regex!(r#"<([^>]+)>; rel="next""#)
        .captures(&link)
        .map(|c| c.get(1).unwrap().as_str().to_string())
}
