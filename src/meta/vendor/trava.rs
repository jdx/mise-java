use crate::github;
use crate::github::GitHubRelease;

use super::JavaMetaData;
use super::Vendor;
use super::normalize_architecture;
use super::normalize_os;
use super::normalize_version;
use eyre::Result;
use log::debug;
use log::warn;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use std::collections::HashSet;
use xx::regex;

#[derive(Clone, Copy, Debug)]
pub struct Trava {}

struct FileNameMeta {
    arch: String,
    os: String,
    ext: String,
}

impl Vendor for Trava {
    fn get_name(&self) -> String {
        "trava".to_string()
    }

    fn fetch_metadata(&self, meta_data: &mut HashSet<JavaMetaData>) -> Result<()> {
        for version in &["8", "11"] {
            debug!("[trava] fetching releases for version: {version}");
            let repo = format!("TravaOpenJDK/trava-jdk-{version}-dcevm");
            let releases = github::list_releases(repo.as_str())?;
            let data = releases
                .into_par_iter()
                .flat_map(|release| {
                    map_release(version, &release).unwrap_or_else(|err| {
                        warn!("[trava] failed to map release: {}", err);
                        vec![]
                    })
                })
                .collect::<Vec<JavaMetaData>>();
            meta_data.extend(data);
        }
        Ok(())
    }
}

fn map_release(version: &str, release: &GitHubRelease) -> Result<Vec<JavaMetaData>> {
    let mut meta_data = vec![];
    let assets = release.assets.iter().filter(|asset| include(asset));
    for asset in assets {
        let filename = asset.name.clone();
        let filename_meta = meta_from_name(version, &filename)?;
        let url = asset.browser_download_url.clone();
        let version = version_from_tag(version, &release.tag_name)?;
        meta_data.push(JavaMetaData {
            architecture: normalize_architecture(&filename_meta.arch),
            features: None,
            filename,
            file_type: filename_meta.ext.clone(),
            image_type: "jdk".to_string(),
            java_version: normalize_version(&version),
            jvm_impl: "hotspot".to_string(),
            os: normalize_os(&filename_meta.os),
            release_type: "ga".to_string(),
            url,
            vendor: "trava".to_string(),
            version: normalize_version(&version),
            ..Default::default()
        });
    }
    Ok(meta_data)
}

fn include(asset: &github::GitHubAsset) -> bool {
    asset.content_type.starts_with("application") && !asset.name.contains("_source") && !asset.name.ends_with(".jar")
}

fn version_from_tag(version: &str, tag: &str) -> Result<String> {
    match version {
        "8" => version_from_tag_8(tag),
        "11" => version_from_tag_11(tag),
        _ => Err(eyre::eyre!("unknown version: {}", version)),
    }
}

fn version_from_tag_8(tag: &str) -> Result<String> {
    let capture = regex!(r"^dcevm8u([0-9]+)b([0-9])+$")
        .captures(tag)
        .ok_or_else(|| eyre::eyre!("regular expression failed for tag: {}", tag))?;
    let major = capture.get(1).unwrap().as_str();
    let build = capture.get(2).unwrap().as_str();
    Ok(format!("8.0.{major}+{build}"))
}

fn version_from_tag_11(tag: &str) -> Result<String> {
    let capture = regex!(r"^dcevm-(11\.[0-9.+]+)$")
        .captures(tag)
        .ok_or_else(|| eyre::eyre!("regular expression failed for tag: {}", tag))?;
    let major = capture.get(1).unwrap().as_str();
    Ok(major.to_string())
}

fn meta_from_name(version: &str, name: &str) -> Result<FileNameMeta> {
    match version {
        "8" => meta_from_name_8(name),
        "11" => meta_from_name_11(name),
        _ => Err(eyre::eyre!("unknown version: {}", version)),
    }
}

fn meta_from_name_8(name: &str) -> Result<FileNameMeta> {
    debug!("[trava] parsing name: {}", name);
    let capture = regex!(r"^java8-openjdk-dcevm-(linux|osx|windows)\.(.*)$")
        .captures(name)
        .ok_or_else(|| eyre::eyre!("regular expression failed for name: {}", name))?;

    let arch = "x86_64".to_string();
    let os = capture.get(1).unwrap().as_str().to_string();
    let ext = capture.get(2).unwrap().as_str().to_string();

    Ok(FileNameMeta { arch, os, ext })
}

fn meta_from_name_11(name: &str) -> Result<FileNameMeta> {
    debug!("[trava] parsing name: {}", name);
    let capture = regex!(r"^(?:java11-openjdk|Openjdk11u)-dcevm-(linux|osx|mac|windows)-?(amd64|arm64|x64)?\.(.*)$")
        .captures(name)
        .ok_or_else(|| eyre::eyre!("regular expression failed for name: {}", name))?;

    let os = capture.get(1).unwrap().as_str().to_string();
    let arch = capture.get(2).map_or("x86_64", |m| m.as_str()).to_string();
    let ext = capture.get(3).unwrap().as_str().to_string();

    Ok(FileNameMeta { arch, os, ext })
}
