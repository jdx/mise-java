use eyre::Result;
use std::collections::HashSet;
use xx::regex;

use log::{debug, warn};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{
    github::{self, GitHubAsset, GitHubRelease},
    http::HTTP,
    jvm::JvmData,
};

use super::{Vendor, normalize_architecture, normalize_os, normalize_version};

#[derive(Clone, Copy, Debug)]
pub struct Kona {}

struct FileNameMeta {
    arch: String,
    ext: String,
    features: String,
    os: String,
    version: String,
}

impl Vendor for Kona {
    fn get_name(&self) -> String {
        "kona".to_string()
    }

    fn fetch_data(&self, meta_data: &mut HashSet<JvmData>) -> eyre::Result<()> {
        for version in &["8", "11", "17", "21"] {
            debug!("[kona] fetching releases for version: {version}");
            let repo = format!("Tencent/TencentKona-{version}");
            let releases = github::list_releases(&repo)?;
            let data = releases
                .into_par_iter()
                .flat_map(|release| {
                    map_release(&release).unwrap_or_else(|err| {
                        warn!("[kona] failed to map release: {}", err);
                        vec![]
                    })
                })
                .collect::<Vec<JvmData>>();
            meta_data.extend(data);
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

    let meta_data = assets
        .into_par_iter()
        .filter_map(|asset| match map_asset(asset) {
            Ok(meta) => Some(meta),
            Err(e) => {
                warn!("[kona] {}", e);
                None
            }
        })
        .collect::<Vec<JvmData>>();

    Ok(meta_data)
}

fn include(asset: &GitHubAsset) -> bool {
    asset.content_type.starts_with("application")
        && !asset.name.contains("_source")
        && !asset.name.contains("-internal")
        && !asset.name.contains("_jre_")
        && !asset.name.ends_with(".md5")
}

fn map_asset(asset: &GitHubAsset) -> Result<JvmData> {
    let md5_url = format!("{}.md5", asset.browser_download_url);
    let md5 = match &asset.name {
        //FIXME: TencentKona-17.0.4.b1_jdk_windows-x86_64_signed.zip is not a valid checksum
        filename if filename.eq_ignore_ascii_case("TencentKona-17.0.4.b1_jdk_windows-x86_64_signed.zip") => None,
        _ => get_md5(asset, &md5_url),
    };
    let filename = asset.name.clone();
    let filename_meta = meta_from_name(&filename)?;
    let features = match filename_meta.features.trim().is_empty() {
        true => None,
        false => {
            let mut feat: Vec<String> = filename_meta
                .features
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            if filename_meta.version.contains("musl") {
                feat.push("musl".to_string());
            }
            Some(feat)
        }
    };
    let url = asset.browser_download_url.clone();
    let version = normalize_version(&filename_meta.version);
    Ok(JvmData {
        architecture: normalize_architecture(&filename_meta.arch),
        checksum: md5,
        checksum_url: Some(md5_url),
        features,
        filename,
        file_type: filename_meta.ext.clone(),
        image_type: "jdk".to_string(),
        java_version: version.clone(),
        jvm_impl: "hotspot".to_string(),
        os: normalize_os(&filename_meta.os),
        release_type: "ga".to_string(),
        url,
        vendor: "kona".to_string(),
        version,
        ..Default::default()
    })
}

fn get_md5(asset: &GitHubAsset, md5_url: &str) -> Option<String> {
    match HTTP.get_text(md5_url) {
        Ok(body) => match body.to_lowercase().starts_with("md5") {
            true => {
                let chunks = body.split('=').map(|s| s.to_string()).collect::<Vec<_>>();
                chunks.get(1).map(|md5| format!("md5:{}", md5.trim()))
            }
            false => {
                let chunks = body.split_whitespace().map(|s| s.to_string()).collect::<Vec<_>>();
                chunks.first().map(|md5| format!("md5:{}", md5.trim()))
            }
        },
        Err(_) => {
            warn!("[kona] unable to find MD5 for asset: {}", asset.name);
            None
        }
    }
}

fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[kona] parsing name: {}", name);
    let capture = regex!(r"^TencentKona-?([0-9b.]{1,})(?:[_-](ea))?[-_]jdk_(?:(fiber|vector-api)_)?(linux[-_]musl|linux|macosx|windows)-(aarch64|x86_64)(?:_8u\d+)?(?:_(notarized|signed))?\.(tar\.gz|zip)$")
        .captures(name)
        .ok_or_else(|| eyre::eyre!("regular expression did not match name: {}", name))?;

    let version = capture.get(1).unwrap().as_str().to_string();
    let features_1 = capture.get(3).map_or("", |m| m.as_str());
    let os = capture.get(4).unwrap().as_str().to_string();
    let arch = capture.get(5).unwrap().as_str().to_string();
    let features_2 = capture.get(6).map_or("", |m| m.as_str());
    let ext = capture.get(7).unwrap().as_str().to_string();

    Ok(FileNameMeta {
        arch,
        ext,
        features: format!("{} {}", features_1, features_2),
        os,
        version,
    })
}
