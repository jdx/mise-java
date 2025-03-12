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

use super::{Vendor, normalize_architecture, normalize_os, normalize_version};

#[derive(Clone, Copy, Debug)]
pub struct Mandrel {}

struct FileNameMeta {
    arch: String,
    java_version: String,
    os: String,
    version: String,
}

impl Vendor for Mandrel {
    fn get_name(&self) -> String {
        "mandrel".to_string()
    }

    fn fetch_metadata(&self, meta_data: &mut HashSet<JavaMetaData>) -> eyre::Result<()> {
        debug!("[mandrel] fetching releases");
        let releases = github::list_releases("graalvm/mandrel")?;
        let data = releases
            .into_par_iter()
            .flat_map(|release| {
                map_release(&release).unwrap_or_else(|err| {
                    warn!("[mandrel] failed to map release: {}", err);
                    vec![]
                })
            })
            .collect::<Vec<JavaMetaData>>();
        meta_data.extend(data);

        Ok(())
    }
}

fn map_release(release: &GitHubRelease) -> Result<Vec<JavaMetaData>> {
    let assets = release
        .assets
        .iter()
        .filter(|asset| include(asset))
        .collect::<Vec<&GitHubAsset>>();

    let meta_data = assets
        .into_par_iter()
        .filter_map(|asset| match map_asset(asset) {
            Ok(meta) => Some(meta),
            Err(e) => {
                warn!("[mandrel] {}", e);
                None
            }
        })
        .collect();

    Ok(meta_data)
}

fn include(asset: &GitHubAsset) -> bool {
    asset.name.starts_with("mandrel-") && (asset.name.ends_with(".tar.gz") || asset.name.ends_with(".zip"))
}

fn map_asset(asset: &GitHubAsset) -> Result<JavaMetaData> {
    let sha256_url = format!("{}.sha256", asset.browser_download_url);
    let sha256 = match HTTP.get_text(&sha256_url) {
        Ok(sha256) => Some(format!("sha256:{}", sha256)),
        Err(_) => {
            warn!("unable to find SHA256 for asset: {}", asset.name);
            None
        }
    };
    let filename = asset.name.clone();
    let ext = match filename {
        _ if filename.ends_with(".zip") => "zip".to_string(),
        _ => "tar.gz".to_string(),
    };
    let filename_meta = meta_from_name(&filename)?;
    let url = asset.browser_download_url.clone();
    Ok(JavaMetaData {
        architecture: normalize_architecture(&filename_meta.arch),
        checksum: sha256.clone(),
        checksum_url: Some(sha256_url.clone()),
        features: None,
        filename,
        file_type: ext.clone(),
        image_type: "jdk".to_string(),
        java_version: normalize_version(&filename_meta.java_version),
        jvm_impl: "graalvm".to_string(),
        os: normalize_os(&filename_meta.os),
        release_type: normalize_release_type(&filename_meta.version),
        url,
        vendor: "mandrel".to_string(),
        version: format!(
            "{}+java{}",
            normalize_version(&filename_meta.version),
            &filename_meta.java_version
        ),
        ..Default::default()
    })
}

fn normalize_release_type(version: &str) -> String {
    if version.contains("Final") {
        "ga".to_string()
    } else {
        "ea".to_string()
    }
}

fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[mandrel] parsing name: {}", name);
    let capture =
        regex!(r"^mandrel-java([0-9]{1,2})-(linux|macos|windows)-(amd64|aarch64)-([0-9+.]{2,}.*)(\.tar\.gz|\.zip)$")
            .captures(name)
            .ok_or_else(|| eyre::eyre!("regular expression did not match name: {}", name))?;

    let java_version = capture.get(1).unwrap().as_str().to_string();
    let os = capture.get(2).unwrap().as_str().to_string();
    let arch = capture.get(3).unwrap().as_str().to_string();
    let version = capture.get(4).unwrap().as_str().to_string();

    Ok(FileNameMeta {
        arch,
        java_version,
        os,
        version,
    })
}
