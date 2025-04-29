use std::collections::{HashMap, HashSet};

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
pub struct Liberica {}

#[derive(Debug, PartialEq)]
struct FileNameMeta {
    arch: String,
    ext: String,
    feature: String,
    image_type: String,
    os: String,
    version: String,
}

impl Vendor for Liberica {
    fn get_name(&self) -> String {
        "liberica".to_string()
    }

    fn fetch_data(&self, jvm_data: &mut HashSet<JvmData>) -> eyre::Result<()> {
        let releases = github::list_releases("bell-sw/Liberica")?;
        let data = releases
            .into_par_iter()
            .flat_map(|release| {
                map_release(&release).unwrap_or_else(|err| {
                    warn!("[liberica] error parsing release: {}", err);
                    vec![]
                })
            })
            .collect::<Vec<JvmData>>();
        jvm_data.extend(data);
        Ok(())
    }
}

fn map_release(release: &GitHubRelease) -> Result<Vec<JvmData>> {
    let sha1sums = get_sha1sums(release)?;
    let assets = release
        .assets
        .iter()
        .filter(|asset| include(asset))
        .collect::<Vec<&github::GitHubAsset>>();

    let jvm_data = assets
        .into_par_iter()
        .filter_map(|asset| match map_asset(release, asset, &sha1sums) {
            Ok(meta) => Some(meta),
            Err(e) => {
                warn!("[liberica] {}", e);
                None
            }
        })
        .collect::<Vec<_>>();

    Ok(jvm_data)
}

fn include(asset: &github::GitHubAsset) -> bool {
    !asset.name.ends_with(".bom")
        && !asset.name.ends_with(".json")
        && !asset.name.ends_with(".txt")
        && !asset.name.ends_with("-src.tar.gz")
        && !asset.name.ends_with("-src-full.tar.gz")
        && !asset.name.ends_with("-src-crac.tar.gz")
        && !asset.name.ends_with("-src-leyden.tar.gz")
        && !asset.name.contains("-full-nosign")
}

fn map_asset(release: &GitHubRelease, asset: &GitHubAsset, sha1sums: &HashMap<String, String>) -> Result<JvmData> {
    let filename = asset.name.clone();
    let filename_meta = meta_from_name(&filename)?;
    let features = normalize_features(&filename_meta.feature);
    let sha1 = match sha1sums.get(&filename) {
        Some(sha1) => Some(format!("sha1:{}", sha1.clone())),
        None => {
            warn!("[liberica] unable to find SHA1 for {filename}");
            None
        }
    };
    let url = asset.browser_download_url.clone();
    Ok(JvmData {
        architecture: normalize_architecture(&filename_meta.arch),
        checksum: sha1.clone(),
        features,
        filename,
        file_type: filename_meta.ext.clone(),
        image_type: filename_meta.image_type.clone(),
        java_version: normalize_version(&filename_meta.version),
        jvm_impl: "hotspot".to_string(),
        os: normalize_os(&filename_meta.os),
        release_type: get_release_type(&filename_meta.version, release.prerelease),
        url,
        vendor: "liberica".to_string(),
        version: normalize_version(&filename_meta.version),
        ..Default::default()
    })
}

fn get_sha1sums(release: &GitHubRelease) -> Result<HashMap<String, String>> {
    let sha1sum_asset = release.assets.iter().find(|asset| asset.name == "sha1sum.txt");
    let sha1sums = match sha1sum_asset {
        Some(asset) => HTTP
            .get_text(&asset.browser_download_url)?
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    Some((parts[1].to_string(), parts[0].to_string()))
                } else {
                    warn!("[liberica] malformed SHA1 line: {}", line);
                    None
                }
            })
            .collect(),
        None => {
            warn!("[liberica] unable to find SHA1 for release: {}", release.tag_name);
            HashMap::new()
        }
    };
    Ok(sha1sums)
}

fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[liberica] parsing name: {}", name);
    let capture = regex!(
        r"^bellsoft-(jre|jdk)(.+)-(?:ea-)?(linux|windows|macos|solaris)-(amd64|i386|i586|aarch64|arm64|ppc64le|arm32-vfp-hflt|x64|sparcv9|riscv64)-?(fx|lite|full|musl|musl-lite|crac|musl-crac|leyden|musl-leyden|lite-leyden|musl-lite-leyden)?\.(apk|deb|rpm|msi|dmg|pkg|tar\.gz|zip)$"
    )
    .captures(name)
    .ok_or_else(|| eyre::eyre!("regular expression did not match name: {}", name))?;

    let image_type = capture.get(1).map_or("jdk", |m| m.as_str()).to_string();
    let version = capture.get(2).unwrap().as_str().to_string();
    let os = capture.get(3).unwrap().as_str().to_string();
    let arch = capture.get(4).unwrap().as_str().to_string();
    let feature = capture.get(5).map_or("", |m| m.as_str()).to_string();
    let ext = capture.get(6).unwrap().as_str().to_string();

    Ok(FileNameMeta {
        arch,
        ext,
        feature,
        image_type,
        os,
        version,
    })
}

fn get_release_type(version: &str, is_prerelease: bool) -> String {
    if is_prerelease || version.contains("ea") {
        "ea".to_string()
    } else {
        "ga".to_string()
    }
}

fn normalize_features(input: &str) -> Option<Vec<String>> {
    let mut features = Vec::new();
    match input {
        "full" => {
            features.push("libericafx".to_string());
            features.push("minimal-vm".to_string());
            features.push("javafx".to_string());
        }
        "fx" => {
            features.push("javafx".to_string());
        }
        _ => {
            features.extend(
                input
                    .split('-')
                    .map(|f| f.to_string())
                    .filter(|f| !f.is_empty())
                    .collect::<Vec<String>>(),
            );
        }
    }
    match features.is_empty() {
        true => None,
        false => {
            features.sort();
            Some(features)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_features() {
        for (actual, expected) in [
            ("fx", Some(vec!["javafx".to_string()])),
            ("musl-leyden", Some(vec!["leyden".to_string(), "musl".to_string()])),
            (
                "musl-lite-leyden",
                Some(vec!["leyden".to_string(), "lite".to_string(), "musl".to_string()]),
            ),
            ("musl-crac", Some(vec!["crac".to_string(), "musl".to_string()])),
            ("musl-lite", Some(vec!["lite".to_string(), "musl".to_string()])),
            ("musl", Some(vec!["musl".to_string()])),
            (
                "full",
                Some(vec![
                    "javafx".to_string(),
                    "libericafx".to_string(),
                    "minimal-vm".to_string(),
                ]),
            ),
        ] {
            assert_eq!(normalize_features(actual), expected);
        }
    }

    #[test]
    fn test_meta_from_name() {
        for (actual, expected) in [
            (
                "bellsoft-jdk11.0.11+9-linux-aarch64-musl-lite.tar.gz",
                FileNameMeta {
                    arch: "aarch64".to_string(),
                    ext: "tar.gz".to_string(),
                    feature: "musl-lite".to_string(),
                    image_type: "jdk".to_string(),
                    os: "linux".to_string(),
                    version: "11.0.11+9".to_string(),
                },
            ),
            (
                "bellsoft-jre22.0.1+10-macos-aarch64.dmg",
                FileNameMeta {
                    arch: "aarch64".to_string(),
                    ext: "dmg".to_string(),
                    feature: "".to_string(),
                    image_type: "jre".to_string(),
                    os: "macos".to_string(),
                    version: "22.0.1+10".to_string(),
                },
            ),
            (
                "bellsoft-jre11.0.25+11-windows-amd64-full.zip",
                FileNameMeta {
                    arch: "amd64".to_string(),
                    ext: "zip".to_string(),
                    feature: "full".to_string(),
                    image_type: "jre".to_string(),
                    os: "windows".to_string(),
                    version: "11.0.25+11".to_string(),
                },
            ),
        ] {
            assert_eq!(meta_from_name(actual).unwrap(), expected);
        }
    }
}
