use std::collections::HashMap;

use crate::{
    github::{self, GitHubRelease},
    http::HTTP,
    meta::JavaMetaData,
};
use eyre::Result;
use log::{debug, warn};
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use xx::regex;

use super::{normalize_architecture, normalize_os, normalize_version, Vendor};

#[derive(Clone, Copy, Debug)]
pub struct Liberica {}

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

    fn fetch_metadata(&self, meta_data: &mut Vec<crate::meta::JavaMetaData>) -> eyre::Result<()> {
        let releases = github::list_releases("bell-sw/Liberica")?;
        let data = releases
            .into_par_iter()
            .flat_map(|release| {
                map_release(&release).unwrap_or_else(|err| {
                    warn!("[liberica] error parsing release: {:?}", err);
                    vec![]
                })
            })
            .collect::<Vec<JavaMetaData>>();
        meta_data.extend(data);
        Ok(())
    }
}

fn map_release(release: &GitHubRelease) -> Result<Vec<JavaMetaData>> {
    let sha1sums = get_sha1sums(release)?;
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
            features,
            filename,
            file_type: filename_meta.ext.clone(),
            image_type: filename_meta.image_type.clone(),
            java_version: normalize_version(&filename_meta.version),
            jvm_impl: "hotspot".to_string(),
            os: normalize_os(&filename_meta.os),
            release_type: get_release_type(&filename_meta.version, release.prerelease),
            sha1,
            url,
            vendor: "liberica".to_string(),
            version: normalize_version(&filename_meta.version),
            ..Default::default()
        });
    }
    Ok(meta_data)
}

fn include(asset: &github::GitHubAsset) -> bool {
    !(asset.name.ends_with(".bom")
        || asset.name.ends_with(".json")
        || asset.name.ends_with(".txt")
        || asset.name.ends_with("-src.tar.gz")
        || asset.name.ends_with("-src-full.tar.gz")
        || asset.name.ends_with("-src-crac.tar.gz")
        || asset.name.ends_with("-src-leyden.tar.gz")
        || asset.name.contains("-full-nosign"))
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
    if is_prerelease {
        "ea".to_string()
    } else if version.contains("ea") {
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
        false => Some(features),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_features() {
        assert_eq!(normalize_features("fx"), Some(vec!["javafx".to_string()]));
        assert_eq!(
            normalize_features("musl-leyden"),
            Some(vec!["musl".to_string(), "leyden".to_string()])
        );
        assert_eq!(
            normalize_features("musl-lite-leyden"),
            Some(vec![
                "musl".to_string(),
                "lite".to_string(),
                "leyden".to_string()
            ])
        );
        assert_eq!(
            normalize_features("musl-crac"),
            Some(vec!["musl".to_string(), "crac".to_string()])
        );
        assert_eq!(
            normalize_features("musl-lite"),
            Some(vec!["musl".to_string(), "lite".to_string()])
        );
        assert_eq!(normalize_features("musl"), Some(vec!["musl".to_string()]));
        assert_eq!(
            normalize_features("full"),
            Some(vec![
                "libericafx".to_string(),
                "minimal-vm".to_string(),
                "javafx".to_string()
            ])
        );
    }
}
