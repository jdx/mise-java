use std::collections::HashMap;

use crate::{
    github::{self, GitHubRelease},
    http::HTTP,
    meta::JavaMetaData,
};
use eyre::Result;
use log::{debug, warn};
use xx::regex;

use super::{normalize_architecture, normalize_os, normalize_version, Vendor};

pub struct Liberica {}

struct FileNameMeta {
    image_type: String,
    release_type: String,
    os: String,
    arch: String,
    feature: String,
    ext: String,
}

impl Vendor for Liberica {
    fn get_name(&self) -> String {
        "liberica".to_string()
    }

    fn fetch_metadata(&self, meta_data: &mut Vec<crate::meta::JavaMetaData>) -> eyre::Result<()> {
        let releases = github::list_releases("bell-sw/Liberica")?;
        for release in &releases {
            if release.prerelease {
                continue;
            }
            meta_data.extend(map_release(release)?);
        }
        Ok(())
    }
}

fn map_release(release: &GitHubRelease) -> Result<Vec<JavaMetaData>> {
    let sha1sums = get_sha1sums(release)?;
    let version = release.tag_name.clone();
    let mut meta_data = vec![];
    let assets = release.assets.iter().filter(|asset| include(asset));
    for asset in assets {
        let filename = asset.name.clone();
        let filename_meta = meta_from_name(&filename)?;
        let features = normalize_features(&filename_meta.feature);
        let sha1 = match sha1sums.get(&filename) {
            Some(sha1) => Some(sha1.clone()),
            None => {
                warn!("unable to find SHA1 for asset: {filename}");
                None
            }
        };
        let url = asset.browser_download_url.clone();
        meta_data.push(JavaMetaData {
            architecture: normalize_architecture(&filename_meta.arch),
            features: Some(features),
            filename,
            file_type: filename_meta.ext.clone(),
            image_type: filename_meta.image_type.clone(),
            java_version: normalize_version(&version),
            jvm_impl: "hotspot".to_string(),
            os: normalize_os(&filename_meta.os),
            release_type: filename_meta.release_type.clone(),
            sha1,
            url,
            vendor: "liberica".to_string(),
            version: normalize_version(&version),
            ..Default::default()
        });
    }
    Ok(meta_data)
}

fn include(asset: &github::GitHubAsset) -> bool {
    asset.name.ends_with(".bom")
        || asset.name.ends_with(".json")
        || asset.name.ends_with(".txt")
        || asset.name.ends_with("-src.tar.gz")
        || asset.name.ends_with("-src-full.tar.gz")
        || asset.name.ends_with("-src-crac.tar.gz")
        || asset.name.ends_with("-src-leyden.tar.gz")
        || asset.name.contains("-full-nosign")
}

fn get_sha1sums(release: &GitHubRelease) -> Result<HashMap<String, String>> {
    let sha1sum_asset = release
        .assets
        .iter()
        .find(|asset| asset.name == "sha1sum.txt");
    let sha1sums = match sha1sum_asset {
        Some(asset) => HTTP
            .get_text(&asset.browser_download_url)?
            .lines()
            .map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                (parts[1].to_string(), parts[0].to_string())
            })
            .collect(),
        None => {
            warn!(
                "unable to find sha1sum.txt for release: {}",
                release.tag_name
            );
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
    let release_type = capture.get(2).map_or("ga", |m| m.as_str()).to_string();
    let os = capture.get(3).unwrap().as_str().to_string();
    let arch = capture.get(4).unwrap().as_str().to_string();
    let feature = capture.get(5).map_or("", |m| m.as_str()).to_string();
    let ext = capture.get(6).unwrap().as_str().to_string();

    Ok(FileNameMeta {
        image_type,
        release_type,
        os,
        arch,
        feature,
        ext,
    })
}

fn normalize_features(input: &str) -> Vec<String> {
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
    features
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_features() {
        assert_eq!(normalize_features("fx"), vec!["javafx"]);
        assert_eq!(normalize_features("musl-leyden"), vec!["musl", "leyden"]);
        assert_eq!(
            normalize_features("musl-lite-leyden"),
            vec!["musl", "lite", "leyden"]
        );
        assert_eq!(normalize_features("musl-crac"), vec!["musl", "crac"]);
        assert_eq!(normalize_features("musl-lite"), vec!["musl", "lite"]);
        assert_eq!(normalize_features("musl"), vec!["musl"]);
        assert_eq!(
            normalize_features("full"),
            vec!["libericafx", "minimal-vm", "javafx"]
        );
    }
}
