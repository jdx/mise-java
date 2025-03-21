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
        for version in &["8", "11", "jdk", "17", "18", "19", "20", "21", "22", "23", "24"] {
            debug!("[corretto] fetching releases for version: {version}");
            let repo = format!("corretto/corretto-{}", version);
            let releases = github::list_releases(repo.as_str())?;
            let data = releases
                .into_par_iter()
                .flat_map(|release| {
                    map_release(&release).unwrap_or_else(|err| {
                        warn!("[corretto] failed to map release: {}", err);
                        vec![]
                    })
                })
                .collect::<Vec<JvmData>>();
            jvm_data.extend(data);
        }
        Ok(())
    }
}

fn map_release(release: &GitHubRelease) -> Result<Vec<JvmData>> {
    let mut meta_data = Vec::new();
    let version = release.tag_name.clone();
    let html = match release.body {
        Some(ref body) => md_to_html(body.as_str()),
        None => {
            warn!("[corretto] no body found for release: {version}");
            return Ok(meta_data);
        }
    };

    let fragment = Html::parse_fragment(&html);
    let table_row_selector = Selector::parse("table tr").unwrap();
    for table_row in fragment.select(&table_row_selector).skip(1) {
        let mut jvm_data = JvmData {
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
                // Type
                1 => {
                    jvm_data.image_type = text.to_lowercase();
                }
                // Download Link
                2 => {
                    let a_selector = Selector::parse("a").unwrap();
                    let anchor = fragment.select(&a_selector).next();
                    if let Some(a) = anchor {
                        let name = a.text().collect::<String>();
                        let url = a.value().attr("href").unwrap();
                        match meta_from_name(name.clone().as_str()) {
                            Ok(filename_meta) => {
                                if filename_meta.os == "alpine-linux" {
                                    jvm_data.features = Some(vec!["musl".to_string()]);
                                }
                                jvm_data.architecture = normalize_architecture(&filename_meta.arch);
                                jvm_data.filename = name.clone();
                                jvm_data.file_type = filename_meta.ext;
                                jvm_data.java_version = normalize_version(&filename_meta.version);
                                jvm_data.os = normalize_os(&filename_meta.os);
                                jvm_data.url = url.to_string();
                                jvm_data.version = normalize_version(&filename_meta.version);
                            }
                            Err(e) => {
                                error!("[corretto] {}", e);
                            }
                        }
                    }
                }
                // Checksum
                3 => {
                    let code_selector = Selector::parse("code").unwrap();
                    let mut code_iter = fragment.select(&code_selector);
                    if let Some(code) = code_iter.next() {
                        let md5 = code.text().collect::<String>();
                        jvm_data.checksum = Some(format!("md5:{}", md5));
                    }
                    if let Some(code) = code_iter.next() {
                        let sha256 = code.text().collect::<String>();
                        jvm_data.checksum = Some(format!("sha256:{}", sha256));
                    };
                }
                _ => (),
            }
        }
        meta_data.push(jvm_data);
    }

    Ok(meta_data)
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
