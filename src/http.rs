#![allow(dead_code)]
use std::time::Duration;

use eyre::Result;
use log::{debug, warn};
use once_cell::sync::Lazy;
use reqwest::blocking::{ClientBuilder, RequestBuilder, Response};
use reqwest::header::HeaderMap;
use reqwest::{IntoUrl, Url};

use crate::cli::version;
use crate::env;

pub static HTTP: Lazy<Client> = Lazy::new(|| Client::new(Duration::from_secs(30)).unwrap());

#[derive(Debug)]
pub struct Client {
    reqwest: reqwest::blocking::Client,
}

impl Client {
    fn new(timeout: Duration) -> Result<Self> {
        Ok(Self {
            reqwest: Self::_new().timeout(timeout).build()?,
        })
    }

    fn _new() -> ClientBuilder {
        reqwest::blocking::ClientBuilder::new()
            .user_agent(format!("{}/{}", &*env::BINARY_NAME, &*version::VERSION))
            .gzip(true)
            .zstd(true)
    }

    pub fn get<U: IntoUrl>(&self, url: U) -> Result<Response> {
        let url = url.into_url()?;
        let mut req = self.reqwest.get(url.clone());
        req = with_github_auth(&url.clone(), req);
        let resp = req.send()?;
        debug!("GET {url} {}", resp.status());
        display_github_rate_limit(&resp);
        resp.error_for_status_ref()?;
        Ok(resp)
    }

    pub fn get_json<T, U: IntoUrl>(&self, url: U) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.get_json_with_headers(url).map(|(json, _)| json)
    }

    pub fn get_json_with_headers<T, U: IntoUrl>(&self, url: U) -> Result<(T, HeaderMap)>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = url.into_url()?;
        let mut req = self.reqwest.get(url.clone());
        req = with_github_auth(&url, req);
        let resp = req.send()?;
        let headers = resp.headers().clone();
        debug!("GET {url} {}", resp.status());
        display_github_rate_limit(&resp);
        resp.error_for_status_ref()?;
        Ok::<(T, HeaderMap), eyre::Error>((resp.json()?, headers))
    }

    pub fn get_text<U: IntoUrl>(&self, url: U) -> Result<String> {
        let url = url.into_url()?;
        let req = self.reqwest.get(url.clone());
        let resp = req.send()?;
        debug!("GET {url} {}", resp.status());
        resp.error_for_status_ref()?;
        Ok(resp.text()?)
    }
}

fn with_github_auth(url: &Url, mut req: RequestBuilder) -> RequestBuilder {
    if url.host_str() == Some("api.github.com") {
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            req = req.header("authorization", format!("token {}", token));
            req = req.header("x-github-api-version", "2022-11-28");
        }
    }
    req
}

fn display_github_rate_limit(resp: &Response) {
    let status = resp.status().as_u16();
    if status == 403 || status == 429 {
        if resp
            .headers()
            .get("x-ratelimit-remaining")
            .is_none_or(|r| r != "0")
        {
            return;
        }
        if let Some(reset) = resp.headers().get("x-ratelimit-reset") {
            let reset = reset.to_str().map(|r| r.to_string()).unwrap_or_default();
            if let Some(reset) = chrono::DateTime::from_timestamp(reset.parse().unwrap(), 0) {
                warn!(
                    "GitHub rate limit exceeded. Resets at {}",
                    reset.naive_local().to_string()
                );
            }
        }
    }
}
