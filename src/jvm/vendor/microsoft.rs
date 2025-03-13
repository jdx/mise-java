use std::collections::HashSet;

use crate::{http::HTTP, jvm::JvmData};
use eyre::Result;
use log::warn;
use log::{debug, error};

use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use xx::regex;

use super::AnchorElement;
use super::anchors_from_html;
use super::{Vendor, normalize_architecture, normalize_os, normalize_version};

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

    fn fetch_data(&self, meta_data: &mut HashSet<JvmData>) -> Result<()> {
        let urls = vec![
            "https://docs.microsoft.com/en-us/java/openjdk/download",
            "https://learn.microsoft.com/en-us/java/openjdk/older-releases",
        ];

        // ElementRef is not Send, so we can't use rayon, so we have to turn it into a usable struct
        let anchors: Vec<AnchorElement> = urls
            .into_iter()
            .flat_map(|url| {
                let releases_html = match HTTP.get_text(url) {
                    Ok(releases_html) => releases_html,
                    Err(e) => {
                        error!("[microsoft] error fetching releases: {}", e);
                        "".to_string()
                    }
                };
                anchors_from_html(
                    &releases_html,
                    "a:is([href$='.tar.gz'], [href$='.zip'], [href$='.msi'],[href$='.dmg'],[href$='.pkg'])",
                )
            })
            .collect();

        let data = anchors
            .into_par_iter()
            .filter(|anchor| !anchor.name.contains("-debugsymbols-") && !anchor.name.contains("-sources-"))
            .flat_map(|anchor| match map_release(&anchor) {
                Ok(release) => vec![release],
                Err(e) => {
                    warn!("[microsoft] {}", e);
                    vec![]
                }
            })
            .collect::<Vec<JvmData>>();
        meta_data.extend(data);
        Ok(())
    }
}

fn map_release(a: &AnchorElement) -> Result<JvmData> {
    let filename_meta = meta_from_name(&a.name)?;
    let sha256_url = format!("{}.sha256sum.txt", &a.href);
    let sha256 = match HTTP.get_text(&sha256_url) {
        Ok(sha) => sha.split_whitespace().next().map(|s| format!("sha256:{}", s)),
        Err(e) => {
            warn!("[microsoft] unable to find SHA256 for {}: {}", a.name, e);
            None
        }
    };

    Ok(JvmData {
        architecture: normalize_architecture(&filename_meta.arch),
        checksum: sha256.clone(),
        checksum_url: Some(sha256_url),
        features: if filename_meta.os == "alpine" {
            Some(vec!["musl".to_string()])
        } else {
            None
        },
        filename: a.name.clone(),
        file_type: filename_meta.ext,
        image_type: "jdk".to_string(),
        java_version: normalize_version(&filename_meta.version),
        jvm_impl: "hotspot".to_string(),
        os: normalize_os(&filename_meta.os),
        release_type: "ga".to_string(),
        url: a.href.clone(),
        version: normalize_version(&filename_meta.version),
        vendor: "microsoft".to_string(),
        ..Default::default()
    })
}

fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[microsoft] parsing name: {}", name);
    let capture = regex!(r"^microsoft-jdk-([0-9+.]{3,})-?.*-(alpine|linux|macos|macOS|windows)-(x64|aarch64)\.(.*)$")
        .captures(name)
        .ok_or_else(|| eyre::eyre!("regular expression did not match for {}", name))?;

    let version = capture.get(1).unwrap().as_str().to_string();
    let os = capture.get(2).unwrap().as_str().to_string();
    let arch = capture.get(3).unwrap().as_str().to_string();
    let ext = capture.get(4).unwrap().as_str().to_string();

    Ok(FileNameMeta { arch, ext, os, version })
}
