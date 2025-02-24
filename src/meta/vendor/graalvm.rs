use super::{normalize_architecture, normalize_os, normalize_version, Vendor};
use crate::{
    github::{self, GitHubAsset, GitHubRelease},
    http::HTTP,
    meta::JavaMetaData,
};
use eyre::Result;
use log::{debug, error, warn};
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use xx::regex;

#[derive(Clone, Copy, Debug)]
pub struct GraalVM {}

struct FileNameMeta {
    arch: String,
    ext: String,
    java_version: String,
    os: String,
    version: String,
}

impl Vendor for GraalVM {
    fn get_name(&self) -> String {
        "graalvm".to_string()
    }

    fn fetch_metadata(&self, meta_data: &mut Vec<JavaMetaData>) -> Result<()> {
        let releases = github::list_releases("graalvm/graalvm-ce-builds")?;
        let data = releases
            .into_par_iter()
            .flat_map(|release| {
                map_release(&release).unwrap_or_else(|err| {
                    warn!("[graalvm] error parsing release: {:?}", err);
                    vec![]
                })
            })
            .collect::<Vec<JavaMetaData>>();
        meta_data.extend(data);
        Ok(())
    }
}

fn map_release(release: &GitHubRelease) -> Result<Vec<JavaMetaData>> {
    let mut meta_data = vec![];
    let assets = release.assets.iter().filter(|asset| include(asset));
    for asset in assets {
        let release = if asset.name.starts_with("graalvm-ce") {
            map_ce(asset)
        } else if asset.name.starts_with("graalvm-community") {
            map_community(asset)
        } else {
            continue;
        };

        match release {
            Ok(release) => meta_data.push(release),
            Err(e) => error!("[graalvm] error parsing release: {:?}", e),
        }
    }
    Ok(meta_data)
}

fn map_ce(asset: &GitHubAsset) -> Result<JavaMetaData> {
    // TODO centralize and handle fetch error with None url return value
    //      only fetch if enabled or unknown (some vendors require 1000s of requests)
    //      fetch_checksum(url: &str) -> Result<(Option<String>, Option<String>)>
    let sha256_url = format!("{}.sha256", asset.browser_download_url);
    let sha256sum = match HTTP.get_text(sha256_url) {
        Ok(sha256) => Some(sha256),
        Err(_) => {
            warn!("unable to find SHA256 for asset: {}", asset.name);
            None
        }
    };
    let filename = asset.name.clone();
    let filename_meta = meta_from_name_ce(&filename)?;
    let url = asset.browser_download_url.clone();
    let version = normalize_version(&filename_meta.version);
    Ok(JavaMetaData {
        architecture: normalize_architecture(&filename_meta.arch),
        filename,
        file_type: filename_meta.ext.clone(),
        image_type: "jdk".to_string(),
        java_version: filename_meta.java_version.clone(),
        jvm_impl: "graalvm".to_string(),
        os: normalize_os(&filename_meta.os),
        release_type: "ga".to_string(),
        sha256: sha256sum,
        url,
        vendor: "graalvm".to_string(),
        version: format!("{}+java{}", version, filename_meta.java_version.clone()),
        ..Default::default()
    })
}

fn map_community(asset: &GitHubAsset) -> Result<JavaMetaData> {
    let sha256_url = format!("{}.sha256", asset.browser_download_url);
    let sha256sum = match HTTP.get_text(&sha256_url) {
        Ok(sha256) => Some(sha256),
        Err(_) => {
            warn!("unable to find SHA256 for asset: {}", asset.name);
            None
        }
    };
    let filename = asset.name.clone();
    let filename_meta = meta_from_name_community(&filename)?;
    let url = asset.browser_download_url.clone();
    let version = normalize_version(&filename_meta.version);
    Ok(JavaMetaData {
        architecture: normalize_architecture(&filename_meta.arch),
        filename,
        file_type: filename_meta.ext.clone(),
        image_type: "jdk".to_string(),
        java_version: version.clone(),
        jvm_impl: "graalvm".to_string(),
        os: normalize_os(&filename_meta.os),
        release_type: "ga".to_string(),
        sha256: sha256sum,
        sha256_url: Some(sha256_url),
        url,
        vendor: "graalvm-community".to_string(),
        version,
        ..Default::default()
    })
}

fn include(asset: &GitHubAsset) -> bool {
    (asset.name.starts_with("graalvm-ce") || asset.name.starts_with("graalvm-community"))
        && (asset.name.ends_with(".tar.gz") || asset.name.ends_with(".zip"))
}

fn meta_from_name_ce(name: &str) -> Result<FileNameMeta> {
    debug!("[graalvm] parsing name: {}", name);
    let capture = regex!(r"^graalvm-ce-(?:complete-)?java([0-9]{1,2})-(linux|darwin|windows)-(aarch64|amd64)-([0-9+.]{2,})\.(zip|tar\.gz)$")
        .captures(name)
        .ok_or_else(|| eyre::eyre!("regular expression did not match name: {}", name))?;

    let java_version = capture.get(1).unwrap().as_str().to_string();
    let os = capture.get(2).unwrap().as_str().to_string();
    let arch = capture.get(3).unwrap().as_str().to_string();
    let version = capture.get(4).unwrap().as_str().to_string();
    let ext = capture.get(5).unwrap().as_str().to_string();

    Ok(FileNameMeta {
        arch,
        ext,
        java_version,
        os,
        version,
    })
}

fn meta_from_name_community(name: &str) -> Result<FileNameMeta> {
    debug!("[graalvm] parsing name: {}", name);
    let capture = regex!(r"^graalvm-community-jdk-([0-9]{1,2}\.[0-9]{1}\.[0-9]{1,3})_(linux|macos|windows)-(aarch64|x64)_bin\.(zip|tar\.gz)$")
      .captures(name)
      .ok_or_else(|| eyre::eyre!("regular expression did not match name: {}", name))?;

    let java_version = capture.get(1).unwrap().as_str().to_string();
    let os = capture.get(2).unwrap().as_str().to_string();
    let arch = capture.get(3).unwrap().as_str().to_string();
    let ext = capture.get(4).unwrap().as_str().to_string();

    Ok(FileNameMeta {
        arch,
        ext,
        java_version: java_version.clone(),
        os,
        version: java_version.clone(),
    })
}
