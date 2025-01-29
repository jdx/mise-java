#![allow(dead_code)]
use std::path::Path;
use std::time::Duration;

use eyre::Result;
use log::{debug, warn};
use once_cell::sync::Lazy;
use reqwest::header::HeaderMap;
use reqwest::{ClientBuilder, IntoUrl, RequestBuilder, Response, Url};
use tokio::io::AsyncWriteExt;

use crate::cli::version;
use crate::env;
use crate::tokio::RUNTIME;

pub static HTTP: Lazy<Client> = Lazy::new(|| Client::new(Duration::from_secs(30)).unwrap());

#[derive(Debug)]
pub struct Client {
    reqwest: reqwest::Client,
}

impl Client {
    fn new(timeout: Duration) -> Result<Self> {
        Ok(Self {
            reqwest: Self::_new()
                .read_timeout(timeout)
                .connect_timeout(timeout)
                .build()?,
        })
    }

    fn _new() -> ClientBuilder {
        ClientBuilder::new()
            .user_agent(format!("{}/{}", &*env::BINARY_NAME, &*version::VERSION))
            .gzip(true)
            .zstd(true)
    }

    pub async fn get_async<U: IntoUrl>(&self, url: U) -> Result<Response> {
        let url = url.into_url()?;
        let mut req = self.reqwest.get(url.clone());
        req = with_github_auth(&url.clone(), req);
        let resp = req.send().await?;
        debug!("GET {url} {}", resp.status());
        display_github_rate_limit(&resp);
        resp.error_for_status_ref()?;
        Ok(resp)
    }

    pub fn get<U: IntoUrl>(&self, url: U) -> Result<Response> {
        RUNTIME.block_on(self.get_async(url))
    }

    pub async fn get_json_async<T, U: IntoUrl>(&self, url: U) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        // let url = url.into_url()?;
        // let req = self.reqwest.get(url.clone());
        // let resp = req.send().await?;
        // debug!("GET {url} {}", resp.status());
        // resp.error_for_status_ref()?;
        // Ok(resp.json::<T>().await?)
        self.get_json_with_headers_async(url)
            .await
            .map(|(json, _)| json)
    }

    pub async fn get_json_with_headers_async<T, U: IntoUrl>(&self, url: U) -> Result<(T, HeaderMap)>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = url.into_url()?;
        let mut req = self.reqwest.get(url.clone());
        req = with_github_auth(&url, req);
        let resp = req.send().await?;
        let headers = resp.headers().clone();
        debug!("GET {url} {}", resp.status());
        display_github_rate_limit(&resp);
        resp.error_for_status_ref()?;
        Ok::<(T, HeaderMap), eyre::Error>((resp.json().await?, headers))
    }

    pub fn get_json<T>(&self, url: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        RUNTIME.block_on(self.get_json_async(url))
    }

    pub fn get_json_with_headers<T, U: IntoUrl>(&self, url: U) -> Result<(T, HeaderMap)>
    where
        T: serde::de::DeserializeOwned,
    {
        RUNTIME.block_on(self.get_json_with_headers_async(url))
    }

    pub async fn get_text_async<U: IntoUrl>(&self, url: U) -> Result<String> {
        let url = url.into_url()?;
        let req = self.reqwest.get(url.clone());
        let resp = req.send().await?;
        debug!("GET {url} {}", resp.status());
        resp.error_for_status_ref()?;
        Ok(resp.text().await?)
    }

    pub fn get_text<U: IntoUrl>(&self, url: U) -> Result<String> {
        RUNTIME.block_on(self.get_text_async(url))
    }

    pub async fn download_file_async<U: IntoUrl, T: AsRef<Path>>(
        &self,
        url: U,
        path: T,
    ) -> Result<()> {
        let mut response = self.get_async(url).await?;
        let mut file = tokio::fs::File::create(path).await?;
        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await?;
        }
        Ok(())
    }

    pub fn download_file<T: AsRef<Path>, U: IntoUrl>(&self, url: U, path: T) -> Result<()> {
        RUNTIME.block_on(self.download_file_async(url, path))
    }
}

fn with_github_auth(url: &Url, mut req: RequestBuilder) -> RequestBuilder {
    if url.host_str() == Some("api.github.com") {
        if let Some(token) = std::env::var("GITHUB_TOKEN").ok() {
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
