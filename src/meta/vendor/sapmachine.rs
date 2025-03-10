use std::collections::HashSet;

use crate::{
    github::{self, GitHubAsset, GitHubRelease},
    http::HTTP,
    meta::JavaMetaData,
};
use eyre::Result;
use log::{debug, warn};
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use xx::regex;

use super::{Vendor, normalize_architecture, normalize_os, normalize_version};

pub struct SAPMachine {}

struct FileNameMeta {
    arch: String,
    ext: String,
    features: String,
    image_type: String,
    os: String,
    version: String,
}

impl Vendor for SAPMachine {
    fn get_name(&self) -> String {
        "sapmachine".to_string()
    }

    fn fetch_metadata(&self, meta_data: &mut HashSet<JavaMetaData>) -> eyre::Result<()> {
        let releases = github::list_releases("SAP/SapMachine")?;
        let data: Vec<JavaMetaData> = releases
            .into_par_iter()
            .flat_map(|release| {
                map_release(&release).unwrap_or_else(|err| {
                    warn!("[sapmachine] failed to map release: {}", err);
                    vec![]
                })
            })
            .collect();
        meta_data.extend(data);
        Ok(())
    }
}

fn map_release(release: &GitHubRelease) -> Result<Vec<JavaMetaData>> {
    let mut meta_data = vec![];
    let assets = release.assets.iter().filter(|asset| include(asset));
    for asset in assets {
        let sha256_url = match &asset.name {
            name if name.ends_with(".tar.gz") => format!(
                "{}.sha256.txt",
                asset.browser_download_url.replace(".tar.gz", "")
            ),
            name if name.ends_with(".zip") => format!(
                "{}.sha256.txt",
                asset.browser_download_url.replace(".zip", "")
            ),
            _ => format!("{}.sha256.txt", asset.browser_download_url),
        };
        let sha256sum = match HTTP.get_text(&sha256_url) {
            Ok(sha256) => Some(sha256.split(" ").next().unwrap().to_string()),
            Err(_) => {
                warn!("unable to find SHA256 for asset: {}", asset.name);
                None
            }
        };
        let filename = asset.name.clone();
        let filename_meta = match asset.name.ends_with(".rpm") {
            true => meta_from_name_rpm(&filename)?,
            false => meta_from_name(&filename)?,
        };
        let features = match filename_meta.features.is_empty() {
            true => None,
            false => Some(vec![filename_meta.features.clone()]),
        };
        let url = asset.browser_download_url.clone();
        let version = normalize_version(&filename_meta.version);
        meta_data.push(JavaMetaData {
            architecture: normalize_architecture(&filename_meta.arch),
            features,
            filename,
            file_type: filename_meta.ext.clone(),
            image_type: filename_meta.image_type.clone(),
            java_version: version.clone(),
            jvm_impl: "hotspot".to_string(),
            os: normalize_os(&filename_meta.os),
            release_type: match release.prerelease {
                true => "ea".to_string(),
                false => "ga".to_string(),
            },
            sha256: sha256sum,
            sha256_url: Some(sha256_url),
            url,
            vendor: "sapmachine".to_string(),
            version: version.clone(),
            ..Default::default()
        })
    }
    Ok(meta_data)
}

fn include(asset: &GitHubAsset) -> bool {
    asset.content_type.starts_with("application")
        && !asset.name.contains("symbols")
        && !asset.name.ends_with(".sha256.txt")
}

fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[sapmachine] parsing name: {}", name);
    let capture = regex!(r"^sapmachine-(jdk|jre)-([0-9].+)_(aix|linux|macos|osx|windows)-(x64|aarch64|ppc64le|ppc64|x64)-?(.*)_bin\.(.+)$")
        .captures(name)
        .ok_or_else(|| eyre::eyre!("regular expression did not match name: {}", name))?;

    let image_type = capture.get(1).unwrap().as_str().to_string();
    let version = capture.get(2).unwrap().as_str().to_string();
    let os = capture.get(3).unwrap().as_str().to_string();
    let arch = capture.get(4).unwrap().as_str().to_string();
    let features = capture.get(5).map_or("", |m| m.as_str()).to_string();
    let ext = capture.get(6).unwrap().as_str().to_string();

    Ok(FileNameMeta {
        arch,
        ext,
        image_type,
        features,
        os,
        version,
    })
}

fn meta_from_name_rpm(name: &str) -> Result<FileNameMeta> {
    debug!("[sapmachine] parsing name: {}", name);
    let capture = regex!(r"^sapmachine-(jdk|jre)-([0-9].+)\.(aarch64|ppc64le|x86_64)\.rpm$")
        .captures(name)
        .ok_or_else(|| eyre::eyre!("regular expression did not match name: {}", name))?;

    let image_type = capture.get(1).unwrap().as_str().to_string();
    let version = capture.get(2).unwrap().as_str().to_string();
    let os = "linux".to_string();
    let arch = capture.get(3).unwrap().as_str().to_string();
    let features = "".to_string();
    let ext = "rpm".to_string();

    Ok(FileNameMeta {
        arch,
        ext,
        image_type,
        features,
        os,
        version,
    })
}
