use crate::{http::HTTP, meta::JavaMetaData};
use eyre::Result;
use log::{debug, error};
use scraper::{ElementRef, Html, Selector};
use xx::regex;

use super::{normalize_architecture, normalize_os, normalize_version, Vendor};

#[derive(Clone, Copy, Debug)]
pub struct Microsoft {}

struct FileNameMeta {
    arch: String,
    ext: String,
    os: String,
    version: String,
}

impl Vendor for Microsoft {
    fn get_name(&self) -> String {
        "microsoft".to_string()
    }

    fn fetch_metadata(&self, meta_data: &mut Vec<JavaMetaData>) -> Result<()> {
        let urls = vec![
            "https://docs.microsoft.com/en-us/java/openjdk/download",
            "https://learn.microsoft.com/en-us/java/openjdk/older-releases",
        ];

        for url in urls {
            let releases_html = HTTP.get_text(url)?;
            let document = Html::parse_document(&releases_html);

            let a_selector = Selector::parse("a:is([href$='.tar.gz'], [href$='.zip'], [href$='.msi'],[href$='.dmg'],[href$='.pkg'])").unwrap();
            for a in document.select(&a_selector) {
                // skip debug symbols releases
                let name = a.text().collect::<String>();
                if name.contains("-debugsymbols-") || name.contains("-sources-") {
                    continue;
                }
                let release = match map_release(&a) {
                    Ok(release) => release,
                    Err(e) => {
                        // older release might contain unsupported versions
                        if url.contains("older-releases") {
                            debug!("[microsoft] error parsing release: {}", name);
                        } else {
                            error!("[microsoft] error parsing release: {:?}", e);
                        }
                        continue;
                    }
                };
                meta_data.push(release);
            }
        }

        Ok(())
    }
}

fn map_release(a: &ElementRef<'_>) -> Result<JavaMetaData> {
    let href = a
        .value()
        .attr("href")
        .ok_or_else(|| eyre::eyre!("no href found"))?;
    let name = a.text().collect::<String>();
    let filename_meta = meta_from_name(&name)?;
    let sha256_url = format!("{}.sha256sum.txt", &href);
    let sha256 = match HTTP.get_text(&sha256_url) {
        Ok(sha) => sha.split_whitespace().next().map(|s| s.to_string()),
        Err(e) => {
            error!("error fetching sha256sum for {name}: {e}");
            None
        }
    };

    Ok(JavaMetaData {
        architecture: normalize_architecture(&filename_meta.arch),
        features: if filename_meta.os == "alpine" {
            Some(vec!["musl".to_string()])
        } else {
            Some(vec![])
        },
        filename: name.to_string(),
        file_type: filename_meta.ext,
        image_type: "jdk".to_string(),
        java_version: normalize_version(&filename_meta.version),
        jvm_impl: "hotspot".to_string(),
        os: normalize_os(&filename_meta.os),
        release_type: "ga".to_string(),
        sha256,
        sha256_url: Some(sha256_url),
        url: href.to_string(),
        version: normalize_version(&filename_meta.version),
        vendor: "microsoft".to_string(),
        ..Default::default()
    })
}

fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[microsoft] parsing name: {}", name);
    let capture = regex!(
        r"^microsoft-jdk-([0-9+.]{3,})-?.*-(alpine|linux|macos|macOS|windows)-(x64|aarch64)\.(.*)$"
    )
    .captures(name)
    .ok_or_else(|| eyre::eyre!("regular expression did not match name: {}", name))?;

    let version = capture.get(1).unwrap().as_str().to_string();
    let os = capture.get(2).unwrap().as_str().to_string();
    let arch = capture.get(3).unwrap().as_str().to_string();
    let ext = capture.get(4).unwrap().as_str().to_string();

    Ok(FileNameMeta {
        arch,
        ext,
        os,
        version,
    })
}
