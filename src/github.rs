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
    with_retry(
        || HTTP.get_json_with_headers::<Vec<GitHubRelease>, _>(url),
        is_transient,
        sleep,
        |attempt, err| warn!("failed to fetch {url} (attempt {attempt}/{MAX_ATTEMPTS}), retrying: {err}"),
    )
}

fn with_retry<T>(
    mut fetch: impl FnMut() -> Result<T>,
    is_transient: impl Fn(&eyre::Report) -> bool,
    mut sleep: impl FnMut(Duration),
    mut on_retry: impl FnMut(u32, &eyre::Report),
) -> Result<T> {
    let mut attempt = 1;
    loop {
        match fetch() {
            Ok(result) => return Ok(result),
            Err(err) if attempt < MAX_ATTEMPTS && is_transient(&err) => {
                on_retry(attempt, &err);
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

#[cfg(test)]
mod tests {
    use super::*;
    use eyre::eyre;

    fn transient(err: &eyre::Report) -> bool {
        err.to_string() == "transient"
    }

    #[test]
    fn retries_transient_error_then_succeeds() {
        let mut calls = 0;
        let mut sleeps = vec![];
        let result = with_retry(
            || {
                calls += 1;
                if calls < 3 { Err(eyre!("transient")) } else { Ok(calls) }
            },
            transient,
            |d| sleeps.push(d),
            |_, _| {},
        );
        assert_eq!(result.unwrap(), 3);
        assert_eq!(sleeps, vec![Duration::from_secs(5), Duration::from_secs(10)]);
    }

    #[test]
    fn gives_up_after_max_attempts() {
        let mut calls = 0;
        let result: Result<()> = with_retry(
            || {
                calls += 1;
                Err(eyre!("transient"))
            },
            transient,
            |_| {},
            |_, _| {},
        );
        assert!(result.is_err());
        assert_eq!(calls, MAX_ATTEMPTS);
    }

    #[test]
    fn does_not_retry_permanent_error() {
        let mut calls = 0;
        let result: Result<()> = with_retry(
            || {
                calls += 1;
                Err(eyre!("422 Unprocessable Entity"))
            },
            transient,
            |_| {},
            |_, _| {},
        );
        assert!(result.is_err());
        assert_eq!(calls, 1);
    }
}
