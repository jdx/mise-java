use std::collections::HashSet;

use crate::{
    github::{self, GitHubRelease},
    jvm::JvmData,
};
use eyre::Result;
use log::{debug, error, warn};
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use scraper::{Html, Selector};
use xx::regex;

use super::{Vendor, md_to_html, normalize_architecture, normalize_os, normalize_version};

#[derive(Clone, Copy, Debug)]
pub struct Corretto {}

struct FileNameMeta {
    arch: String,
    os: String,
    ext: String,
    version: String,
}

impl Vendor for Corretto {
    fn get_name(&self) -> String {
        "corretto".to_string()
    }

    fn fetch_data(&self, jvm_data: &mut HashSet<JvmData>) -> Result<()> {
        let versions = ["8", "11", "jdk", "17", "18", "19", "20", "21", "22", "23", "24"];
        for version in versions.iter() {
            debug!("[corretto] fetching releases for version: {version}");
            let repo = format!("corretto/corretto-{version}");
            let releases = github::list_releases(&repo)?;
            let data = releases
                .into_par_iter()
                .flat_map(|release| {
                    map_release(&release).unwrap_or_else(|err| {
                        warn!("[corretto] failed to map release: {}", err);
                        vec![]
                    })
                })
                .collect::<Vec<_>>();
            jvm_data.extend(data);
        }
        Ok(())
    }
}

fn map_release(release: &GitHubRelease) -> Result<Vec<JvmData>> {
    let mut jvm_data = Vec::new();
    let version = &release.tag_name;
    let html = release.body.as_deref().map(md_to_html).unwrap_or_else(|| {
        warn!("[corretto] no body found for release: {version}");
        String::new()
    });

    let fragment = Html::parse_fragment(&html);
    let table_row_selector = Selector::parse("table tr").unwrap();
    for table_row in fragment.select(&table_row_selector).skip(1) {
        let mut jvm = JvmData {
            jvm_impl: "hotspot".to_string(),
            release_type: "ga".to_string(),
            vendor: "corretto".to_string(),
            ..Default::default()
        };
        let table_data_selector = Selector::parse("td").unwrap();
        for (index, table_data) in table_row.select(&table_data_selector).enumerate() {
            let text = table_data.text().collect::<String>();
            let html = table_data.html();
            let fragment = Html::parse_fragment(&html);
            match index {
                1 => jvm.image_type = text.to_lowercase(),
                2 => process_download_link(&mut jvm, &fragment),
                3 => process_checksum(&mut jvm, &fragment),
                _ => (),
            }
        }
        jvm_data.push(jvm);
    }

    Ok(jvm_data)
}

fn process_download_link(jvm: &mut JvmData, fragment: &Html) {
    let a_selector = Selector::parse("a").unwrap();
    if let Some(a) = fragment.select(&a_selector).next() {
        let name = a.text().collect::<String>();
        let url = a.value().attr("href").unwrap_or_default();
        if let Ok(meta) = meta_from_name(&name) {
            if meta.os == "alpine-linux" {
                jvm.features = Some(vec!["musl".to_string()]);
            }
            jvm.architecture = normalize_architecture(&meta.arch);
            jvm.filename = name;
            jvm.file_type = meta.ext;
            jvm.java_version = normalize_version(&meta.version);
            jvm.os = normalize_os(&meta.os);
            jvm.url = url.to_string();
            jvm.version = normalize_version(&meta.version);
        } else {
            error!("[corretto] failed to parse metadata for {}", name);
        }
    }
}

fn process_checksum(jvm: &mut JvmData, fragment: &Html) {
    let code_selector = Selector::parse("code").unwrap();
    let mut codes = fragment
        .select(&code_selector)
        .map(|code| code.text().collect::<String>());
    if let Some(md5) = codes.next() {
        jvm.checksum = Some(format!("md5:{}", md5));
    }
    if let Some(sha256) = codes.next() {
        jvm.checksum = Some(format!("sha256:{}", sha256));
    }
}

fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[corretto] parsing name: {}", name);
    let capture = regex!(r".*?-corretto(-devel|-jdk)?[\-_]([\w\d._]+(-\d)?)-?(alpine-linux|linux|macosx|windows)?[._\-](amd64|arm64|armv7|aarch64|x64|i386|x86|x86_64)(-(jdk|jre|musl-headless))?\.(.*)")
    .captures(name)
    .ok_or_else(|| eyre::eyre!("regular expression did not match for {}", name))?;

    let arch = capture.get(5).unwrap().as_str().to_string();
    let ext = capture.get(8).unwrap().as_str().to_string();
    let os = match capture.get(4) {
        Some(os) => os.as_str().to_string(),
        None => {
            if ext == "rpm" || ext == "deb" {
                "linux".to_string()
            } else {
                "unknown".to_string()
            }
        }
    };
    let version = capture.get(2).unwrap().as_str().to_string();

    Ok(FileNameMeta { arch, os, ext, version })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archive_formats() {
        for (actual, expected) in [
            (
                meta_from_name("amazon-corretto-11.0.18.10.1-linux-x64.tar.gz").unwrap(),
                FileNameMeta {
                    arch: "x64".to_string(),
                    os: "linux".to_string(),
                    ext: "tar.gz".to_string(),
                    version: "11.0.18.10.1".to_string(),
                },
            ),
            (
                meta_from_name("amazon-corretto-11.0.19.7.1-alpine-linux-x64.tar.gz").unwrap(),
                FileNameMeta {
                    arch: "x64".to_string(),
                    os: "alpine-linux".to_string(),
                    ext: "tar.gz".to_string(),
                    version: "11.0.19.7.1".to_string(),
                },
            ),
            (
                meta_from_name("amazon-corretto-8.382.05.1-windows-x64-jdk.zip").unwrap(),
                FileNameMeta {
                    arch: "x64".to_string(),
                    os: "windows".to_string(),
                    ext: "zip".to_string(),
                    version: "8.382.05.1".to_string(),
                },
            ),
            (
                meta_from_name("amazon-corretto-17.0.7.7.1-macosx-aarch64.tar.gz").unwrap(),
                FileNameMeta {
                    arch: "aarch64".to_string(),
                    os: "macosx".to_string(),
                    ext: "tar.gz".to_string(),
                    version: "17.0.7.7.1".to_string(),
                },
            ),
            (
                meta_from_name("amazon-corretto-11.0.19.7.1-linux-x64-musl-headless.tar.gz").unwrap(),
                FileNameMeta {
                    arch: "x64".to_string(),
                    os: "linux".to_string(),
                    ext: "tar.gz".to_string(),
                    version: "11.0.19.7.1".to_string(),
                },
            ),
            (
                meta_from_name("amazon-corretto-21.0.1.9.1-linux-arm64.tar.gz").unwrap(),
                FileNameMeta {
                    arch: "arm64".to_string(),
                    os: "linux".to_string(),
                    ext: "tar.gz".to_string(),
                    version: "21.0.1.9.1".to_string(),
                },
            ),
        ] {
            assert_eq!(actual.arch, expected.arch);
            assert_eq!(actual.os, expected.os);
            assert_eq!(actual.ext, expected.ext);
            assert_eq!(actual.version, expected.version);
        }
    }

    #[test]
    fn test_package_formats() {
        for (actual, expected) in [
            (
                meta_from_name("java-11-amazon-corretto-devel-11.0.18.10.1-1.x86_64.rpm").unwrap(),
                FileNameMeta {
                    arch: "x86_64".to_string(),
                    os: "linux".to_string(),
                    ext: "rpm".to_string(),
                    version: "11.0.18.10.1-1".to_string(),
                },
            ),
            (
                meta_from_name("java-17-amazon-corretto-jdk_17.0.7-1_amd64.deb").unwrap(),
                FileNameMeta {
                    arch: "amd64".to_string(),
                    os: "linux".to_string(),
                    ext: "deb".to_string(),
                    version: "17.0.7-1".to_string(),
                },
            ),
        ] {
            assert_eq!(actual.arch, expected.arch);
            assert_eq!(actual.os, expected.os);
            assert_eq!(actual.ext, expected.ext);
            assert_eq!(actual.version, expected.version);
        }
    }
}
