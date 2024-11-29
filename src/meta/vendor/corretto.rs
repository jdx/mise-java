use crate::{
    debug,
    github::{self, GitHubRelease},
    meta::JavaMetaData,
};
use comrak::{markdown_to_html, ComrakOptions};
use eyre::Result;
use indoc::formatdoc;
use log::{error, info};
use scraper::{Html, Selector};
use xx::regex;

use super::{normalize_architecture, normalize_os, normalize_version, Vendor};

pub struct Corretto {}

impl Vendor for Corretto {
    fn get_name(&self) -> String {
        "corretto".to_string()
    }

    fn fetch(&self) -> Result<Vec<JavaMetaData>> {
        debug!("[corretto] fetching available releases");

        let mut meta_data = Vec::new();

        for version in &["8", "11", "jdk", "17", "18", "19", "20", "21", "22", "23"] {
            let repo = format!("corretto/corretto-{}", version);
            let releases = github::list_releases(repo.as_str())?;

            for release in &releases {
                meta_data.extend(map_release(&release));
            }
        }

        info!("[corretto] fetched {} entries", meta_data.len());
        Ok(meta_data)
    }
}

fn map_release(release: &GitHubRelease) -> Vec<JavaMetaData> {
    let mut metadata = Vec::new();
    let version = release.tag_name.clone();

    let html = body_to_html(release.body.as_str());
    let fragment = Html::parse_fragment(&html);
    let table_row_selector = Selector::parse("table tr").unwrap();
    for table_row in fragment.select(&table_row_selector).skip(1) {
        let mut metadata_entry = JavaMetaData {
            features: Some(vec![]),
            java_version: normalize_version(version.as_str()),
            jvm_impl: "hotspot".to_string(),
            release_type: "ga".to_string(),
            vendor: "corretto".to_string(),
            version: normalize_version(version.as_str()),
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
                    let a = fragment.select(&a_selector).next();
                    match a {
                        Some(a) => {
                            let name = a.text().collect::<String>();
                            let url = a.value().attr("href").unwrap();

                            metadata_entry.filename = name.clone();
                            metadata_entry.url = url.to_string();
                            let (os, arch, ext) = match meta_from_name(name.clone().as_str()) {
                                Ok((os, arch, ext)) => (os, arch, ext),
                                Err(e) => {
                                    error!("Failed to parse name: {:?}", e);
                                    continue;
                                }
                            };

                            if os == "alpine-linux" {
                                metadata_entry.features = Some(vec!["musl".to_string()]);
                            }
                            metadata_entry.os = normalize_os(os.as_str());
                            metadata_entry.architecture = normalize_architecture(&arch);
                            metadata_entry.file_type = ext;
                        }
                        None => (),
                    }
                }
                // Checksum
                3 => {
                    let code_selector = Selector::parse("code").unwrap();
                    let mut code_iter = fragment.select(&code_selector).into_iter();
                    if let Some(code) = code_iter.next() {
                        let md5 = code.text().collect::<String>();
                        metadata_entry.md5 = Some(md5);
                    }
                    if let Some(code) = code_iter.next() {
                        let sha256 = code.text().collect::<String>();
                        metadata_entry.sha256 = sha256;
                    };
                }
                _ => (),
            }
        }
        metadata.push(metadata_entry);
    }

    metadata
}

fn body_to_html(body: &str) -> String {
    let markdown_input = formatdoc! {r#"
      {markdown}
      "#,
      markdown = body.replace("\\r\\n", "\n"),
    };

    let mut options = ComrakOptions::default();
    options.extension.table = true;

    markdown_to_html(&markdown_input, &options)
}

fn meta_from_name(name: &str) -> Result<(String, String, String)> {
    let capture = regex!(r".*?-corretto(-devel|-jdk)?[\-_]([\w\d._]+(-\d)?)-?(alpine-linux|linux|macosx|windows)?[._\-](amd64|arm64|armv7|aarch64|x64|i386|x86|x86_64)(-(jdk|jre|musl-headless))?\.(.*)")
        .captures(name)
        .ok_or_else(|| eyre::eyre!("Regular expression failed for name: {}", name))?;

    let mut os = capture.get(4).unwrap().as_str().to_string();
    let arch = capture.get(5).unwrap().as_str().to_string();
    let ext = capture.get(8).unwrap().as_str().to_string();
    match ext.as_str() {
        "rpm" => {
            os = "linux".to_string();
        }
        "deb" => {
            os = "linux".to_string();
        }
        _ => (),
    };

    Ok((os, arch, ext))
}
