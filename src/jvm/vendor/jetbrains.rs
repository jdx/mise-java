use std::collections::HashSet;

use crate::{
    github::{self, GitHubRelease},
    http::HTTP,
    jvm::JvmData,
};
use eyre::Result;
use log::{debug, error, warn};
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use scraper::{ElementRef, Html, Selector};
use xx::regex;

use super::{Vendor, md_to_html, normalize_architecture, normalize_os, normalize_version};

#[derive(Clone, Copy, Debug)]
pub struct Jetbrains {}

#[derive(Debug, PartialEq)]
struct FileNameMeta {
    arch: String,
    ext: String,
    image_type: String,
    os: String,
    version: String,
}

impl Vendor for Jetbrains {
    fn get_name(&self) -> String {
        "jetbrains".to_string()
    }

    fn fetch_data(&self, jvm_data: &mut HashSet<JvmData>) -> eyre::Result<()> {
        let releases = github::list_releases("JetBrains/JetBrainsRuntime")?;
        let data = releases
            .into_par_iter()
            .flat_map(|release| {
                let mut data = vec![];
                let version = release.tag_name.as_str();
                let html = match release.body {
                    Some(ref body) => md_to_html(body.as_str()),
                    None => {
                        warn!("[jetbrains] no body found for release: {version}");
                        return data;
                    }
                };
                let fragment = Html::parse_fragment(&html);
                let a_selector =
                    Selector::parse("table a:is([href$='.pkg'], [href$='.tar.gz'], [href$='.zip'])").unwrap();

                for a in fragment.select(&a_selector) {
                    match map_release(&release, &a) {
                        Ok(release) => data.push(release),
                        Err(e) => {
                            error!("[jetbrains] {}", e);
                        }
                    }
                }
                data
            })
            .collect::<Vec<JvmData>>();
        jvm_data.extend(data);
        Ok(())
    }
}

fn map_release(release: &GitHubRelease, a: &ElementRef<'_>) -> Result<JvmData> {
    let href = a.value().attr("href").ok_or_else(|| eyre::eyre!("no href found"))?;
    let name = href
        .split("/")
        .last()
        .ok_or_else(|| eyre::eyre!("no name found"))?
        .to_string();
    let filename_meta = meta_from_name(&name)?;
    let sha512_url = format!("{}.checksum", &href);
    let sha512 = match HTTP.get_text(&sha512_url) {
        Ok(sha512) => match sha512.split_whitespace().next() {
            Some(s) => match s.len() {
                64 => Some(format!("sha256:{s}")),
                _ => Some(format!("sha512:{s}")),
            },
            None => {
                warn!("[jetbrains] unable to parse SHA512 for {name}");
                None
            }
        },
        Err(_) => {
            warn!("[jetbrains] unable to find SHA256/SHA512 for {name}");
            None
        }
    };
    Ok(JvmData {
        architecture: normalize_architecture(&filename_meta.arch),
        checksum: sha512,
        checksum_url: Some(sha512_url),
        features: normalize_features(&name),
        filename: name.to_string(),
        file_type: filename_meta.ext,
        image_type: filename_meta.image_type,
        java_version: normalize_version(&filename_meta.version),
        jvm_impl: "hotspot".to_string(),
        os: normalize_os(&filename_meta.os),
        release_type: match release.prerelease {
            true => "ea".to_string(),
            false => "ga".to_string(),
        },
        url: href.to_string(),
        version: normalize_version(&filename_meta.version),
        vendor: "jetbrains".to_string(),
        ..Default::default()
    })
}

fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[jetbrains] parsing name: {}", name);
    let capture = regex!(r"^jbr(sdk)?(?:_\w+)?-([0-9][0-9\+._]{1,})-(linux-musl|linux|osx|macos|windows)-(aarch64|x64|x86)(?:-\w+)?-(b[0-9\+.]{1,})(?:_\w+)?\.(tar\.gz|zip|pkg)$")
        .captures(name)
        .ok_or_else(|| eyre::eyre!("regular expression did not match for {}", name))?;

    let image_type = capture
        .get(1)
        .map_or("jre", |m| match m.as_str() {
            "sdk" => "jdk",
            _ => "jre",
        })
        .to_string();
    let os = capture.get(3).unwrap().as_str().to_string();
    let arch = capture.get(4).unwrap().as_str().to_string();
    let version = format!(
        "{}-{}",
        capture.get(2).unwrap().as_str(),
        capture.get(5).unwrap().as_str()
    );
    let ext = capture.get(6).unwrap().as_str().to_string();

    Ok(FileNameMeta {
        arch,
        ext,
        image_type,
        os,
        version,
    })
}

fn normalize_features(name: &str) -> Option<Vec<String>> {
    let mut features = vec![];
    let name = name.to_lowercase();
    if name.contains("_diz") {
        features.push("debug".to_string());
    }
    if name.contains("jcef") {
        features.push("jcef".to_string());
    }
    if name.contains("-fastdebug") {
        features.push("fastdebug".to_string());
    }
    if name.contains("_fd") {
        features.push("jcef".to_string());
        features.push("fastdebug".to_string());
    }
    if name.contains("_ft") {
        features.push("freetype".to_string());
    }
    if name.contains("musl") {
        features.push("musl".to_string());
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
        for (actual, expected) in [
            ("_diz", Some(vec!["debug".to_string()])),
            ("_fd", Some(vec!["jcef".to_string(), "fastdebug".to_string()])),
            ("_ft", Some(vec!["freetype".to_string()])),
            ("_musl", Some(vec!["musl".to_string()])),
            ("_jcef", Some(vec!["jcef".to_string()])),
            ("-fastdebug", Some(vec!["fastdebug".to_string()])),
        ] {
            assert_eq!(normalize_features(actual), expected);
        }
    }

    #[test]
    fn test_meta_from_name() {
        for (actual, expected) in [
            (
                "jbr_fd-17.0.4.1-linux-aarch64-b629.2.tar.gz",
                FileNameMeta {
                    arch: "aarch64".to_string(),
                    ext: "tar.gz".to_string(),
                    image_type: "jre".to_string(),
                    os: "linux".to_string(),
                    version: "17.0.4.1-b629.2".to_string(),
                },
            ),
            (
                "jbrsdk-21.0.5-osx-aarch64-b792.48_diz.tar.gz",
                FileNameMeta {
                    arch: "aarch64".to_string(),
                    ext: "tar.gz".to_string(),
                    image_type: "jdk".to_string(),
                    os: "osx".to_string(),
                    version: "21.0.5-b792.48".to_string(),
                },
            ),
            (
                "jbrsdk-21.0.6-windows-x64-b895.97.zip",
                FileNameMeta {
                    arch: "x64".to_string(),
                    ext: "zip".to_string(),
                    image_type: "jdk".to_string(),
                    os: "windows".to_string(),
                    version: "21.0.6-b895.97".to_string(),
                },
            ),
        ] {
            assert_eq!(meta_from_name(actual).unwrap(), expected);
        }
    }
}
