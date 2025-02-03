use eyre::Result;
use log::{debug, error};
use scraper::{ElementRef, Html, Selector};
use xx::regex;

use crate::{http::HTTP, meta::JavaMetaData};

use super::{normalize_architecture, normalize_os, normalize_version, Vendor};

pub struct OpenJDK {}

struct FileNameMeta {
    arch: String,
    ext: String,
    os: String,
    version: String,
}

impl Vendor for OpenJDK {
    fn get_name(&self) -> String {
        "openjdk".to_string()
    }

    fn fetch_metadata(&self, meta_data: &mut Vec<crate::meta::JavaMetaData>) -> eyre::Result<()> {
        for version in vec![
            "archive", "21", "22", "23", "24", "leyden", "loom", "valhalla",
        ] {
            let url = format!("http://jdk.java.net/{version}/");
            let releases_html = HTTP.get_text(url)?;
            let document = Html::parse_document(&releases_html);

            let a_selector = Selector::parse("a:is([href$='.tar.gz'], [href$='.zip'])").unwrap();
            for a in document.select(&a_selector) {
                let release = match map_release(&a) {
                    Ok(release) => release,
                    Err(e) => {
                        error!("[openjdk] error parsing release: {:?}", e);
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
    let name = href
        .split("/")
        .last()
        .ok_or_else(|| eyre::eyre!("no name found"))?
        .to_string();
    let filename_meta = meta_from_name(&name)?;
    let arch = &filename_meta.arch;
    let features = if arch.contains("x64-musl") {
        Some(vec!["musl".to_string()])
    } else {
        Some(vec![])
    };
    let sha256_url = format!("{}.sha256", &href);
    let sha256 = match HTTP.get_text(&sha256_url) {
        Ok(sha) => sha.split_whitespace().next().map(|s| s.to_string()),
        Err(e) => {
            error!("error fetching sha256sum for {name}: {e}");
            None
        }
    };

    Ok(JavaMetaData {
        architecture: normalize_architecture(&arch),
        features,
        filename: name.to_string(),
        file_type: filename_meta.ext,
        image_type: "jdk".to_string(),
        java_version: normalize_version(&filename_meta.version),
        jvm_impl: "hotspot".to_string(),
        os: normalize_os(&filename_meta.os),
        release_type: normalize_release_type(&filename_meta.version),
        sha256,
        sha256_url: Some(sha256_url),
        url: href.to_string(),
        version: normalize_version(&filename_meta.version),
        vendor: "openjdk".to_string(),
        ..Default::default()
    })
}

fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[oracle] parsing name: {}", name);
    let capture = regex!(r"^openjdk-([0-9]{1,}[^_]*)_(linux|osx|macos|windows)-(aarch64|x64-musl|x64)_bin\.(tar\.gz|zip)$")
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

fn normalize_release_type(version: &str) -> String {
    if version.contains("-ea")
        || version.contains("-leyden")
        || version.contains("-loom")
        || version.contains("-valhalla")
    {
        "ea".to_string()
    } else {
        "ga".to_string()
    }
}
