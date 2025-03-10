use std::collections::HashSet;

use crate::{http::HTTP, meta::JavaMetaData};
use eyre::Result;
use log::{debug, error};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use xx::regex;

use super::{
    AnchorElement, Vendor, anchors_from_html, normalize_architecture, normalize_os,
    normalize_version,
};

pub struct Oracle {}

struct FileNameMeta {
    arch: String,
    ext: String,
    os: String,
    version: String,
}

impl Vendor for Oracle {
    fn get_name(&self) -> String {
        "oracle".to_string()
    }

    fn fetch_metadata(&self, meta_data: &mut HashSet<JavaMetaData>) -> Result<()> {
        let anchors = (17..=23)
            .into_par_iter()
            .flat_map(|version| {
                let url = format!("https://www.oracle.com/java/technologies/javase/jdk{version}-archive-downloads.html");
                let releases_html = match HTTP.get_text(&url) {
                    Ok(releases_html) => releases_html,
                    Err(e) => {
                        error!("[oracle] error fetching releases: {:?}", e);
                        "".to_string()
                    }
                };
                anchors_from_html(&releases_html, "a:is([href$='.dep'],[href$='.dmg'], [href$='.exe'], [href$='.msi'], [href$='.rpm'], [href$='.tar.gz'], [href$='.zip'])")
            })
            .collect::<Vec<_>>();
        let data = anchors
            .into_par_iter()
            .flat_map(|anchor| match map_release(&anchor) {
                Ok(release) => vec![release],
                Err(e) => {
                    error!("[oracle] error parsing release: {:?}", e);
                    vec![]
                }
            })
            .collect::<Vec<_>>();
        meta_data.extend(data);
        Ok(())
    }
}

fn map_release(a: &AnchorElement) -> Result<JavaMetaData> {
    let name = a
        .name
        .split("/")
        .last()
        .ok_or_else(|| eyre::eyre!("no name found"))?
        .to_string();
    let filename_meta = meta_from_name(&name)?;
    let sha256_url = format!("{}.sha256", &a.href);
    let sha256 = match HTTP.get_text(&sha256_url) {
        Ok(sha) => sha.split_whitespace().next().map(|s| s.to_string()),
        Err(e) => {
            error!("error fetching sha256sum for {name}: {e}");
            None
        }
    };

    Ok(JavaMetaData {
        architecture: normalize_architecture(&filename_meta.arch),
        features: None,
        filename: name.to_string(),
        file_type: filename_meta.ext,
        image_type: "jdk".to_string(),
        java_version: normalize_version(&filename_meta.version),
        jvm_impl: "hotspot".to_string(),
        os: normalize_os(&filename_meta.os),
        release_type: "ga".to_string(),
        sha256,
        sha256_url: Some(sha256_url),
        url: a.href.clone(),
        version: normalize_version(&filename_meta.version),
        vendor: "oracle".to_string(),
        ..Default::default()
    })
}

fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[oracle] parsing name: {}", name);
    let capture = regex!(r"^jdk-([0-9+.]{2,})_(linux|macos|windows)-(x64|aarch64)_bin\.(dep|dmg|exe|msi|rpm|tar\.gz|zip)$")
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
