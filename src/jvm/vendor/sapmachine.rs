use std::collections::HashSet;

use crate::{
    github::{self, GitHubAsset, GitHubRelease},
    http::HTTP,
    jvm::JvmData,
};
use eyre::Result;
use log::{debug, warn};
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use xx::regex;

use super::{Vendor, normalize_architecture, normalize_os, normalize_version};

#[derive(Clone, Copy, Debug)]
pub struct SAPMachine {}

#[derive(Debug, Default, PartialEq)]
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

    fn fetch_data(&self, jvm_data: &mut HashSet<JvmData>) -> eyre::Result<()> {
        let releases = github::list_releases("SAP/SapMachine")?;
        let data: Vec<JvmData> = releases
            .into_par_iter()
            .flat_map(|release| {
                map_release(&release).unwrap_or_else(|err| {
                    warn!("[sapmachine] failed to map release: {}", err);
                    vec![]
                })
            })
            .collect();
        jvm_data.extend(data);
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
        .filter_map(|asset| match map_asset(release, asset) {
            Ok(meta) => Some(meta),
            Err(err) => {
                warn!("[sapmachine] {}", err);
                None
            }
        })
        .collect::<Vec<_>>();

    Ok(jvm_data)
}

fn map_asset(release: &GitHubRelease, asset: &GitHubAsset) -> Result<JvmData> {
    let sha256_url = get_sha256_url(asset);
    let sha256 = match sha256_url {
        Some(ref url) => match HTTP.get_text(url.clone()) {
            Ok(sha256) => match sha256.split_whitespace().next() {
                Some(sha256) => Some(format!("sha256:{}", sha256.trim())),
                None => {
                    warn!("[sapmachine] unable to find SHA256 for {}", asset.name);
                    None
                }
            },
            Err(_) => {
                warn!("[sapmachine] unable to find SHA256 for {}", asset.name);
                None
            }
        },
        None => None,
    };
    let filename = asset.name.clone();
    let filename_meta = meta_from_name(&filename)?;
    let features = match filename_meta.features.is_empty() {
        true => None,
        false => Some(vec![filename_meta.features.clone()]),
    };
    let url = asset.browser_download_url.clone();
    let version = normalize_version(&filename_meta.version);
    Ok(JvmData {
        architecture: normalize_architecture(&filename_meta.arch),
        checksum: sha256,
        checksum_url: sha256_url,
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
        url,
        vendor: "sapmachine".to_string(),
        version: version.clone(),
        ..Default::default()
    })
}

fn get_sha256_url(asset: &GitHubAsset) -> Option<String> {
    match &asset.name {
        name if name.ends_with(".tar.gz") => Some(format!(
            "{}.sha256.txt",
            asset.browser_download_url.replace(".tar.gz", "")
        )),
        name if name.ends_with(".zip") => {
            Some(format!("{}.sha256.txt", asset.browser_download_url.replace(".zip", "")))
        }
        // rpm packages do not come with sha256 checksums
        name if name.ends_with(".rpm") => None,
        // skip dmg/msi for now; the checksum is inconsistent
        // either in .dmg.sha256.txt or sha256.dmg.txt or missing randomly
        name if name.ends_with(".dmg") || name.ends_with(".msi") => None,
        _ => Some(format!("{}.sha256.txt", asset.browser_download_url)),
    }
}

fn include(asset: &GitHubAsset) -> bool {
    asset.content_type.starts_with("application")
        && !asset.name.contains("symbols")
        && !asset.name.ends_with(".sha256.txt")
}

fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[sapmachine] parsing name: {}", name);
    match name {
        name if name.ends_with(".rpm") => meta_from_name_rpm(name),
        _ => meta_from_name_other(name),
    }
}

fn meta_from_name_other(name: &str) -> Result<FileNameMeta> {
    let capture = regex!(r"^sapmachine-(jdk|jre)-([0-9].+)_(aix|linux|macos|osx|windows)-(x64|aarch64|ppc64le|ppc64|x64)-?(.*)_bin\.(.+)$")
        .captures(name)
        .ok_or_else(|| eyre::eyre!("regular expression did not match for {}", name))?;

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
    let capture = regex!(r"^sapmachine-(jdk|jre)-([0-9].+)\.(aarch64|ppc64le|x86_64)\.rpm$")
        .captures(name)
        .ok_or_else(|| eyre::eyre!("regular expression did not match for {}", name))?;

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_meta_from_name() {
        for (actual, expected) in [
            (
                "sapmachine-jdk-23_linux-aarch64_bin.tar.gz",
                FileNameMeta {
                    arch: "aarch64".to_string(),
                    ext: "tar.gz".to_string(),
                    features: "".to_string(),
                    image_type: "jdk".to_string(),
                    os: "linux".to_string(),
                    version: "23".to_string(),
                },
            ),
            (
                "sapmachine-jre-18.0.1.1_macos-aarch64_bin.tar.gz",
                FileNameMeta {
                    arch: "aarch64".to_string(),
                    ext: "tar.gz".to_string(),
                    features: "".to_string(),
                    image_type: "jre".to_string(),
                    os: "macos".to_string(),
                    version: "18.0.1.1".to_string(),
                },
            ),
            (
                "sapmachine-jdk-21.0.4_windows-x64_bin.zip",
                FileNameMeta {
                    arch: "x64".to_string(),
                    ext: "zip".to_string(),
                    features: "".to_string(),
                    image_type: "jdk".to_string(),
                    os: "windows".to_string(),
                    version: "21.0.4".to_string(),
                },
            ),
        ] {
            assert_eq!(meta_from_name(actual).unwrap(), expected);
        }
    }

    #[test]
    fn test_meta_from_name_rpm() {
        for (actual, expected) in [
            (
                "sapmachine-jdk-17.0.14-1.aarch64.rpm",
                FileNameMeta {
                    arch: "aarch64".to_string(),
                    ext: "rpm".to_string(),
                    features: "".to_string(),
                    image_type: "jdk".to_string(),
                    os: "linux".to_string(),
                    version: "17.0.14-1".to_string(),
                },
            ),
            (
                "sapmachine-jdk-23-1.x86_64.rpm",
                FileNameMeta {
                    arch: "x86_64".to_string(),
                    ext: "rpm".to_string(),
                    features: "".to_string(),
                    image_type: "jdk".to_string(),
                    os: "linux".to_string(),
                    version: "23-1".to_string(),
                },
            ),
        ] {
            assert_eq!(meta_from_name_rpm(actual).unwrap(), expected);
        }
    }
}
