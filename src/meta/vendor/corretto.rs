use std::collections::HashSet;

use crate::{
    github::{self, GitHubRelease},
    meta::JavaMetaData,
};
use eyre::Result;
use log::{debug, warn};
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

    fn fetch_metadata(&self, meta_data: &mut HashSet<JavaMetaData>) -> Result<()> {
        for version in &["8", "11", "jdk", "17", "18", "19", "20", "21", "22", "23"] {
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
                .collect::<Vec<JavaMetaData>>();
            meta_data.extend(data);
        }
        Ok(())
    }
}

fn map_release(release: &GitHubRelease) -> Result<Vec<JavaMetaData>> {
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
        let mut metadata_entry = JavaMetaData {
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
                    metadata_entry.image_type = text.to_lowercase();
                }
                // Download Link
                2 => {
                    let a_selector = Selector::parse("a").unwrap();
                    let anchor = fragment.select(&a_selector).next();
                    if let Some(a) = anchor {
                        let name = a.text().collect::<String>();
                        let url = a.value().attr("href").unwrap();
                        if let Ok(filename_meta) = meta_from_name(name.clone().as_str()) {
                            if filename_meta.os == "alpine-linux" {
                                metadata_entry.features = Some(vec!["musl".to_string()]);
                            }
                            metadata_entry.architecture =
                                normalize_architecture(&filename_meta.arch);
                            metadata_entry.filename = name.clone();
                            metadata_entry.file_type = filename_meta.ext;
                            metadata_entry.java_version = normalize_version(&filename_meta.version);
                            metadata_entry.os = normalize_os(&filename_meta.os);
                            metadata_entry.url = url.to_string();
                            metadata_entry.version = normalize_version(&filename_meta.version);
                        }
                    }
                }
                // Checksum
                3 => {
                    let code_selector = Selector::parse("code").unwrap();
                    let mut code_iter = fragment.select(&code_selector);
                    if let Some(code) = code_iter.next() {
                        let md5 = code.text().collect::<String>();
                        metadata_entry.checksum = Some(format!("md5:{}", md5));
                    }
                    if let Some(code) = code_iter.next() {
                        let sha256 = code.text().collect::<String>();
                        metadata_entry.checksum = Some(format!("sha256:{}", sha256));
                    };
                }
                _ => (),
            }
        }
        meta_data.push(metadata_entry);
    }

    Ok(meta_data)
}

fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[corretto] parsing name: {}", name);
    let capture = regex!(r".*?-corretto(-devel|-jdk)?[\-_]([\w\d._]+(-\d)?)-?(alpine-linux|linux|macosx|windows)?[._\-](amd64|arm64|armv7|aarch64|x64|i386|x86|x86_64)(-(jdk|jre|musl-headless))?\.(.*)")
        .captures(name)
        .ok_or_else(|| eyre::eyre!("regular expression failed for name: {}", name))?;

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

    Ok(FileNameMeta {
        arch,
        os,
        ext,
        version,
    })
}
