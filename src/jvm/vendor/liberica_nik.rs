use std::collections::HashSet;

use crate::{http::HTTP, jvm::JvmData};
use eyre::Result;
use indoc::formatdoc;
use log::{debug, warn};
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use serde::{Deserialize, Serialize};
use xx::regex;

use super::{Vendor, normalize_architecture, normalize_os, normalize_version};

#[derive(Clone, Copy, Debug)]
pub struct LibericaNIK {}

#[derive(Debug, PartialEq)]
struct FileNameMeta {
    arch: String,
    ext: String,
    java_version: String,
    os: String,
    version: String,
}

impl Vendor for LibericaNIK {
    fn get_name(&self) -> String {
        "liberica-nik".to_string()
    }

    fn fetch_data(&self, jvm_data: &mut HashSet<JvmData>) -> eyre::Result<()> {
        let api_url = formatdoc! {"https://api.bell-sw.com/v1/nik/releases
            ?fields=architecture,downloadUrl,GA,os,bundleType,filename,packageType,size,sha1,version"
        };
        debug!("[liberica-nik] fetching releases from {api_url}");

        let releases = HTTP.get_json::<Vec<Release>, _>(api_url)?;
        let data = releases
            .into_par_iter()
            // filter out source releases
            .filter(|release| !&release.filename.contains("-src"))
            // filter out full and standard releases, keep only core releases for now
            .filter(|release| release.filename.contains("-core-"))
            .flat_map(|release| match map_release(&release) {
                Ok(meta) => vec![meta],
                Err(err) => {
                    warn!("[liberica-nik] error parsing release: {err}");
                    vec![]
                }
            })
            .collect::<Vec<JvmData>>();
        jvm_data.extend(data);
        Ok(())
    }
}

fn map_release(release: &Release) -> Result<JvmData> {
    let filename_meta = meta_from_name(&release.filename)?;
    let architecture = normalize_architecture(&filename_meta.arch);
    let release_type = if release.ga { "ga" } else { "ea" };
    let features = normalize_features(release);
    let os = normalize_os(&release.os);
    let java_version = normalize_version(&filename_meta.java_version);
    let version = normalize_version(&filename_meta.version);

    let meta = JvmData {
        architecture,
        checksum: Some(format!("sha1:{}", release.sha1)),
        file_type: release.package_type.clone(),
        features,
        filename: release.filename.clone(),
        image_type: "jdk".to_string(),
        java_version,
        jvm_impl: "graalvm".to_string(),
        os,
        release_type: release_type.to_string(),
        size: Some(release.size as i32),
        url: release.download_url.clone(),
        vendor: "liberica-nik".to_string(),
        version,
        ..Default::default()
    };
    Ok(meta)
}

fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[liberica-nik] parsing name: {name}");
    let capture = regex!(
        r"^bellsoft-liberica-vm(?:-core|-full)?-openjdk(?P<java>.*?)-(?P<version>.*?)(-ea)?-(?P<os>.*?)-(?<arch>.*?)-?(?:musl)?.(?P<ext>apk|deb|dmg|msi|pkg|rpm|tar\.gz|zip)$"
    )
    .captures(name)
    .ok_or_else(|| eyre::eyre!("regular expression did not match name: {name}"))?;

    let java_version = capture.name("java").unwrap().as_str().to_string();
    let version = capture.name("version").unwrap().as_str().to_string();
    let os = capture.name("os").unwrap().as_str().to_string();
    let arch = capture.name("arch").unwrap().as_str().to_string();
    let ext = capture.name("ext").unwrap().as_str().to_string();

    Ok(FileNameMeta {
        arch,
        ext,
        java_version,
        os,
        version,
    })
}

fn normalize_features(release: &Release) -> Option<Vec<String>> {
    let mut features = Vec::new();
    if release.filename.contains("-musl") {
        features.push("musl".to_string());
    }
    if features.is_empty() { None } else { Some(features) }
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct Release {
    architecture: String,
    #[serde(rename = "bundleType")]
    bundle_type: String,
    #[serde(rename = "downloadUrl")]
    download_url: String,
    filename: String,
    #[serde(rename = "GA")]
    ga: bool,
    os: String,
    #[serde(rename = "packageType")]
    package_type: String,
    sha1: String,
    size: u64,
    version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_from_name() {
        for (actual, expected) in [
            (
                "bellsoft-liberica-vm-openjdk17.0.6+10-22.3.1+1-macos-aarch64.zip",
                FileNameMeta {
                    arch: "aarch64".to_string(),
                    ext: "zip".to_string(),
                    java_version: "17.0.6+10".to_string(),
                    os: "macos".to_string(),
                    version: "22.3.1+1".to_string(),
                },
            ),
            (
                "bellsoft-liberica-vm-openjdk24.0.1+11-24.2.1+3-linux-amd64.tar.gz",
                FileNameMeta {
                    arch: "amd64".to_string(),
                    ext: "tar.gz".to_string(),
                    java_version: "24.0.1+11".to_string(),
                    os: "linux".to_string(),
                    version: "24.2.1+3".to_string(),
                },
            ),
            (
                "bellsoft-liberica-vm-core-openjdk11-21.3.1-linux-aarch64-musl.tar.gz",
                FileNameMeta {
                    arch: "aarch64".to_string(),
                    ext: "tar.gz".to_string(),
                    java_version: "11".to_string(),
                    os: "linux".to_string(),
                    version: "21.3.1".to_string(),
                },
            ),
            (
                "bellsoft-liberica-vm-openjdk17.0.5+8-22.3.0+1-ea-macos-amd64.zip",
                FileNameMeta {
                    arch: "amd64".to_string(),
                    ext: "zip".to_string(),
                    java_version: "17.0.5+8".to_string(),
                    os: "macos".to_string(),
                    version: "22.3.0+1".to_string(),
                },
            ),
        ] {
            assert_eq!(meta_from_name(actual).unwrap(), expected);
        }
    }
}
