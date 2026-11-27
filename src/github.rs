use std::thread::sleep;
use std::time::Duration;

use log::{error, warn};
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
pub struct GitHubAsset {
    pub browser_download_url: String,
    pub content_type: String,
    pub name: String,
    pub size: u64,
}

pub fn list_releases(repo: &str) -> Result<Vec<GitHubRelease>> {
    let url = format!("https://api.github.com/repos/{repo}/releases?per_page=100");

    let (mut releases, mut headers) = get_page(&url)?;

    while let Some(next) = next_page(&headers) {
        let (more, h) = match get_page(&next) {
            Ok(result) => result,
            Err(err) => {
                // GitHub API returns 422 if more than 1000 releases are requested
                error!("failed to fetch release page: {err}");
                break;
            }
        };
        releases.extend(more);
        headers = h;
    }
    releases.retain(|r| !r.draft);

    Ok(releases)
}

const MAX_ATTEMPTS: u32 = 3;

/// Fetches one page of releases, retrying failures that are usually transient on
/// large release listings: 5xx (e.g. 504 Gateway Timeout), timeouts and
/// truncated response bodies.
fn get_page(url: &str) -> Result<(Vec<GitHubRelease>, HeaderMap)> {
    let mut attempt = 1;
    loop {
        match HTTP.get_json_with_headers::<Vec<GitHubRelease>, _>(url) {
            Ok(result) => return Ok(result),
            Err(err) if attempt < MAX_ATTEMPTS && is_transient(&err) => {
                warn!("failed to fetch {url} (attempt {attempt}/{MAX_ATTEMPTS}), retrying: {err}");
                sleep(Duration::from_secs(5 * u64::from(attempt)));
                attempt += 1;
            }
            Err(err) => return Err(err),
        }
    }
}

fn is_transient(err: &eyre::Report) -> bool {
    err.downcast_ref::<reqwest::Error>().is_some_and(|e| {
        e.is_timeout()
            || e.is_decode()
            || e.is_body()
            || e.is_connect()
            || e.status().is_some_and(|s| s.is_server_error())
    })
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
