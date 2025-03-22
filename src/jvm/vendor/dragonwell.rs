use std::collections::HashSet;

use eyre::Result;
use log::{debug, warn};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use xx::regex;

use crate::{
    github::{self, GitHubAsset, GitHubRelease},
    http::HTTP,
    jvm::JvmData,
};

use super::{Vendor, normalize_architecture, normalize_os, normalize_version};

#[derive(Clone, Copy, Debug)]
pub struct Dragonwell {}

#[derive(Default)]
struct FileNameMeta {
    arch: String,
    ext: String,
    java_version: String,
    os: String,
    release_type: Option<String>,
    version: String,
}

impl Vendor for Dragonwell {
    fn get_name(&self) -> String {
        "dragonwell".to_string()
    }

    fn fetch_data(&self, jvm_data: &mut HashSet<JvmData>) -> eyre::Result<()> {
        for version in &["8", "11", "17", "21"] {
            debug!("[dragonwell] fetching releases for version: {version}");
            let repo = format!("dragonwell-project/dragonwell{}", version);
            let releases = github::list_releases(repo.as_str())?;
            let data = releases
                .into_par_iter()
                .flat_map(|release| {
                    map_release(&release).unwrap_or_else(|err| {
                        warn!("[dragonwell] failed to map release: {}", err);
                        vec![]
                    })
                })
                .collect::<Vec<JvmData>>();
            jvm_data.extend(data);
        }
        Ok(())
    }
}

fn map_release(release: &GitHubRelease) -> Result<Vec<JvmData>> {
    let assets = release
        .assets
        .iter()
        .filter(|asset| include(asset))
        .collect::<Vec<&GitHubAsset>>();

    let jvm_data = assets
        .into_par_iter()
        .filter_map(|asset| match map_asset(asset) {
            Ok(meta) => Some(meta),
            Err(err) => {
                warn!("[dragonwell] {}", err);
                None
            }
        })
        .collect::<Vec<_>>();

    Ok(jvm_data)
}

fn include(asset: &GitHubAsset) -> bool {
    asset.content_type.starts_with("application")
        && !asset.name.contains("_source")
        && !asset.name.ends_with(".jar")
        && !asset.name.ends_with(".json")
        && !asset.name.ends_with(".sig")
}

fn map_asset(asset: &GitHubAsset) -> Result<JvmData> {
    let sha256_url = format!("{}.sha256.txt", asset.browser_download_url);
    let sha256 = match HTTP.get_text(&sha256_url) {
        Ok(sha256) => match sha256.split_whitespace().next() {
            Some(sha256) => Some(format!("sha256:{}", sha256)),
            None => {
                warn!("[dragonwell] unable to parse SHA256 for {}", asset.name);
                None
            }
        },
        Err(_) => {
            warn!("[dragonwell] unable to find SHA256 for {}", asset.name);
            None
        }
    };
    let filename = asset.name.clone();
    let filename_meta = meta_from_name(&filename)?;
    let url = asset.browser_download_url.clone();
    let version = normalize_version(&filename_meta.version);
    Ok(JvmData {
        architecture: normalize_architecture(&filename_meta.arch),
        checksum: sha256,
        checksum_url: Some(sha256_url),
        features: if filename.contains("_alpine") {
            Some(vec!["musl".to_string()])
        } else {
            None
        },
        filename,
        file_type: filename_meta.ext.clone(),
        image_type: "jdk".to_string(),
        java_version: filename_meta.java_version.clone(),
        jvm_impl: "hotspot".to_string(),
        os: normalize_os(&filename_meta.os),
        release_type: normalize_release_type(&filename_meta.release_type.map_or("ga".to_string(), |s| s)),
        url,
        vendor: "dragonwell".to_string(),
        version,
        ..Default::default()
    })
}

fn normalize_release_type(release_type: &str) -> String {
    match release_type {
        _ if release_type.eq_ignore_ascii_case("ea")
            || release_type.contains("Experimental")
            || release_type.contains("preview")
            || release_type == "FP1" =>
        {
            "ea".to_string()
        }
        _ => "ga".to_string(),
    }
}

fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[dragonwell] parsing name: {}", name);
    if let Some(caps) = regex!(r"^Alibaba_Dragonwell_(?:Standard|Extended)[–_]([0-9\+.]{1,}[^_]*)_(aarch64|riscv64|x64)(?:_alpine)?[-_](Linux|linux|Windows|windows)\.(.*)$").captures(name) {
      Ok(FileNameMeta {
        java_version: caps.get(1).unwrap().as_str().to_string(),
        version: caps.get(1).unwrap().as_str().to_string(),
        arch: caps.get(2).unwrap().as_str().to_string(),
        os: caps.get(3).unwrap().as_str().to_string(),
        ext: caps.get(4).unwrap().as_str().to_string(),
        release_type: None,
      })
    } else if let Some(caps) = regex!(r"^Alibaba_Dragonwell_([0-9\+.]{1,}[^_]*)(?:_alpine)?_(aarch64|x64|x86)(?:_alpine)?[_-](Linux|linux|Windows|windows)\.(.*)$").captures(name) {
      Ok(FileNameMeta {
        java_version: caps.get(1).unwrap().as_str().to_string(),
        version: caps.get(1).unwrap().as_str().to_string(),
        arch: caps.get(2).unwrap().as_str().to_string(),
        os: caps.get(3).unwrap().as_str().to_string(),
        ext: caps.get(4).unwrap().as_str().to_string(),
        release_type: None,
      })
    } else if name.starts_with("Alibaba_Dragonwell") {
      let caps = regex!(r"^Alibaba_Dragonwell_([0-9\+.]{1,}[^_-]*)(?:_alpine)?[_-](?:(GA|Experimental|GA_Experimental|FP1)_)?(Linux|linux|Windows|windows)_(aarch64|x64)\.(.*)$")
        .captures(name)
        .ok_or_else(|| eyre::eyre!("regular expression failed for name: {}", name))?;
      Ok(FileNameMeta {
        java_version: caps.get(1).unwrap().as_str().to_string(),
        version: caps.get(1).unwrap().as_str().to_string(),
        release_type: caps.get(2).map(|m| m.as_str().to_string()),
        os: caps.get(3).unwrap().as_str().to_string(),
        arch: caps.get(4).unwrap().as_str().to_string(),
        ext: caps.get(5).unwrap().as_str().to_string(),
      })
    } else {
        let caps = regex!(r"^OpenJDK(?:[0-9\+].{1,})_(x64|aarch64)_(linux|windows)_dragonwell_dragonwell-([0-9.]+)(?:_jdk)?[-_]([0-9._]+)-?(ga|.*)\.(tar\.gz|zip)$")
            .captures(name)
            .ok_or_else(|| eyre::eyre!("regular expression failed for name: {}", name))?;
        Ok(FileNameMeta {
            arch: caps.get(1).unwrap().as_str().to_string(),
            os: caps.get(2).unwrap().as_str().to_string(),
            version: caps.get(3).unwrap().as_str().to_string(),
            java_version: caps.get(4).unwrap().as_str().to_string(),
            release_type: Some(caps.get(5).unwrap().as_str().to_string()),
            ext: caps.get(6).unwrap().as_str().to_string(),
        })
    }
}
