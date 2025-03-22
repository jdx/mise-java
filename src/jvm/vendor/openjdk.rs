use std::collections::HashSet;

use eyre::Result;
use log::{debug, error, warn};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use xx::regex;

use crate::{http::HTTP, jvm::JvmData};

use super::{AnchorElement, Vendor, anchors_from_html, normalize_architecture, normalize_os, normalize_version};

#[derive(Clone, Copy, Debug)]
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

    fn fetch_data(&self, jvm_data: &mut HashSet<JvmData>) -> eyre::Result<()> {
        let anchors: Vec<AnchorElement> = vec!["archive", "21", "22", "23", "24", "25", "leyden", "loom", "valhalla"]
            .into_par_iter()
            .flat_map(|version| {
                let url = format!("http://jdk.java.net/{version}/");
                let releases_html = match HTTP.get_text(url) {
                    Ok(releases_html) => releases_html,
                    Err(e) => {
                        error!("[openjdk] error fetching releases: {}", e);
                        "".to_string()
                    }
                };
                anchors_from_html(&releases_html, "a:is([href$='.tar.gz'], [href$='.zip'])")
            })
            .collect();

        let data = anchors
            .into_par_iter()
            .filter_map(|anchor| match map_release(&anchor) {
                Ok(release) => Some(release),
                Err(e) => {
                    warn!("[openjdk] {}", e);
                    None
                }
            })
            .collect::<Vec<JvmData>>();
        jvm_data.extend(data);
        Ok(())
    }
}

fn map_release(a: &AnchorElement) -> Result<JvmData> {
    let name = a
        .href
        .split("/")
        .last()
        .ok_or_else(|| eyre::eyre!("no name found"))?
        .to_string();
    let filename_meta = meta_from_name(&name)?;
    let arch = &filename_meta.arch;
    let features = if arch.contains("x64-musl") {
        Some(vec!["musl".to_string()])
    } else {
        None
    };
    let sha256_url = format!("{}.sha256", &a.href);
    let sha256 = match HTTP.get_text(&sha256_url) {
        Ok(sha) => sha.split_whitespace().next().map(|s| format!("sha256:{}", s)),
        Err(_) => {
            warn!("[openjdk] unable to find SHA256 for {name}");
            None
        }
    };

    Ok(JvmData {
        architecture: normalize_architecture(arch),
        checksum: sha256.clone(),
        checksum_url: Some(sha256_url),
        features,
        filename: name.clone(),
        file_type: filename_meta.ext,
        image_type: "jdk".to_string(),
        java_version: normalize_version(&filename_meta.version),
        jvm_impl: "hotspot".to_string(),
        os: normalize_os(&filename_meta.os),
        release_type: normalize_release_type(&filename_meta.version),
        url: a.href.clone(),
        version: normalize_version(&filename_meta.version),
        vendor: "openjdk".to_string(),
        ..Default::default()
    })
}

fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[oracle] parsing name: {}", name);
    let capture =
        regex!(r"^openjdk-([0-9]{1,}[^_]*)_(linux|osx|macos|windows)-(aarch64|x64-musl|x64)_bin\.(tar\.gz|zip)$")
            .captures(name)
            .ok_or_else(|| eyre::eyre!("regular expression did not match for {}", name))?;

    let version = capture.get(1).unwrap().as_str().to_string();
    let os = capture.get(2).unwrap().as_str().to_string();
    let arch = capture.get(3).unwrap().as_str().to_string();
    let ext = capture.get(4).unwrap().as_str().to_string();

    Ok(FileNameMeta { arch, ext, os, version })
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
