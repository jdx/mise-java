use std::collections::HashSet;

use eyre::Result;
use log::{debug, warn};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use xx::regex;

use crate::{
    github::{self, GitHubAsset, GitHubRelease},
    http::HTTP,
    meta::JavaMetaData,
};

use super::{normalize_architecture, normalize_os, normalize_version, Vendor};

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

    fn fetch_metadata(&self, meta_data: &mut HashSet<JavaMetaData>) -> eyre::Result<()> {
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
                .collect::<Vec<JavaMetaData>>();
            meta_data.extend(data);
        }
        Ok(())
    }
}

fn map_release(release: &GitHubRelease) -> Result<Vec<JavaMetaData>> {
    let mut meta_data = vec![];
    let assets = release.assets.iter().filter(|asset| include(asset));
    for asset in assets {
        let sha256_url = format!("{}.sha256.txt", asset.browser_download_url);
        let sha256sum = match HTTP.get_text(&sha256_url) {
            Ok(sha256) => Some(sha256),
            Err(_) => {
                warn!("unable to find SHA256 for asset: {}", asset.name);
                None
            }
        };
        let filename = asset.name.clone();
        let filename_meta = meta_from_name(&filename)?;
        let url = asset.browser_download_url.clone();
        let version = normalize_version(&filename_meta.version);
        meta_data.push(JavaMetaData {
            architecture: normalize_architecture(&filename_meta.arch),
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
            release_type: normalize_release_type(&filename_meta.release_type.map_or(
                "ga".to_string(),
                |s| {
                    if s.contains("preview") {
                        "ea".to_string()
                    } else {
                        s
                    }
                },
            )),
            sha256: sha256sum,
            sha256_url: Some(sha256_url),
            url,
            vendor: "dragonwell".to_string(),
            version,
            ..Default::default()
        });
    }
    Ok(meta_data)
}

fn include(asset: &GitHubAsset) -> bool {
    asset.content_type.starts_with("application")
        && !asset.name.contains("_source")
        && !asset.name.ends_with(".jar")
        && !asset.name.ends_with(".json")
        && !asset.name.ends_with(".sig")
}

fn normalize_release_type(release_type: &str) -> String {
    match release_type {
        _ if release_type.eq_ignore_ascii_case("ea")
            || release_type.contains("Experimental")
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
      let caps = regex!(r"^Alibaba_Dragonwell_([0-9\+.]{1,}[^_]*)(?:_alpine)?[_-](?:(GA|Experimental|GA_Experimental|FP1)_)?(Linux|linux|Windows|windows)_(aarch64|x64)\.(.*)$")
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
