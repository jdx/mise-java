use crate::{http::HTTP, jvm::JvmData};
use eyre::Result;
use log::{debug, error, warn};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::collections::HashSet;
use xx::regex;

use super::{AnchorElement, Vendor, anchors_from_html, normalize_architecture, normalize_os, normalize_version};

#[derive(Clone, Copy, Debug)]
pub struct OracleGraalVM {}

#[derive(Debug, PartialEq)]
struct FileNameMeta {
    arch: String,
    ext: String,
    os: String,
    version: String,
}

impl Vendor for OracleGraalVM {
    fn get_name(&self) -> String {
        "oracle-graalvm".to_string()
    }

    fn fetch_data(&self, jvm_data: &mut HashSet<JvmData>) -> eyre::Result<()> {
        let anchors = build_urls()
      .into_par_iter()
      .flat_map(|url| {
          let releases_html = match HTTP.get_text(&url) {
              Ok(releases_html) => releases_html,
              Err(e) => {
                  error!("[oracle-graalvm] error fetching releases: {}", e);
                  "".to_string()
              }
          };
          anchors_from_html(&releases_html, "a:is([href$='.dep'],[href$='.dmg'], [href$='.exe'], [href$='.msi'], [href$='.rpm'], [href$='.tar.gz'], [href$='.zip'])")
      })
      .collect::<Vec<_>>();
        let data = anchors
            .into_par_iter()
            .filter(|a| a.href.contains("graalvm-"))
            .flat_map(|anchor| match map_release(&anchor) {
                Ok(release) => vec![release],
                Err(e) => {
                    warn!("[oracle-graalvm] {}", e);
                    vec![]
                }
            })
            .collect::<Vec<_>>();
        jvm_data.extend(data);
        Ok(())
    }
}

fn map_release(a: &AnchorElement) -> Result<JvmData> {
    let name = a
        .name
        .split("/")
        .last()
        .ok_or_else(|| eyre::eyre!("no name found"))?
        .to_string();
    let filename_meta = meta_from_name(&name)?;
    let sha256_url = format!("{}.sha256", &a.href);
    let sha256 = match HTTP.get_text(&sha256_url) {
        Ok(sha256) => sha256.split_whitespace().next().map(|s| format!("sha256:{}", s)),
        Err(_) => {
            warn!("[oracle-graalvm] unable to find SHA256 for {name}");
            None
        }
    };

    Ok(JvmData {
        architecture: normalize_architecture(&filename_meta.arch),
        checksum: sha256.clone(),
        checksum_url: Some(sha256_url),
        features: None,
        filename: name.to_string(),
        file_type: filename_meta.ext,
        image_type: "jdk".to_string(),
        java_version: normalize_version(&filename_meta.version),
        jvm_impl: "hotspot".to_string(),
        os: normalize_os(&filename_meta.os),
        release_type: "ga".to_string(),
        url: a.href.clone(),
        version: normalize_version(&filename_meta.version),
        vendor: "oracle-graalvm".to_string(),
        ..Default::default()
    })
}

fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[oracle-graalvm] parsing name: {}", name);
    let capture = regex!(
        r"^graalvm-jdk-([0-9+.]{2,})_(linux|macos|windows)-(x64|aarch64)_bin\.(tar\.gz|zip|msi|dmg|exe|deb|rpm)$"
    )
    .captures(name)
    .ok_or_else(|| eyre::eyre!("regular expression did not match for {}", name))?;

    let version = capture.get(1).unwrap().as_str().to_string();
    let os = capture.get(2).unwrap().as_str().to_string();
    let arch = capture.get(3).unwrap().as_str().to_string();
    let ext = capture.get(4).unwrap().as_str().to_string();

    Ok(FileNameMeta { arch, ext, os, version })
}

fn build_urls() -> Vec<String> {
    let mut urls = vec!["https://www.oracle.com/java/technologies/downloads/".to_string()];
    for version in [17, 20, 21, 22, 23] {
        urls.push(format!(
            "https://www.oracle.com/java/technologies/javase/graalvm-jdk{version}-archive-downloads.html"
        ));
    }
    urls
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_meta_from_name() {
        for (actual, expected) in [
            (
                "graalvm-jdk-21.0.4_linux-aarch64_bin.tar.gz",
                FileNameMeta {
                    arch: "aarch64".to_string(),
                    ext: "tar.gz".to_string(),
                    os: "linux".to_string(),
                    version: "21.0.4".to_string(),
                },
            ),
            (
                "graalvm-jdk-17.0.8_macos-aarch64_bin.tar.gz",
                FileNameMeta {
                    arch: "aarch64".to_string(),
                    ext: "tar.gz".to_string(),
                    os: "macos".to_string(),
                    version: "17.0.8".to_string(),
                },
            ),
            (
                "graalvm-jdk-22.0.1_windows-x64_bin.zip",
                FileNameMeta {
                    arch: "x64".to_string(),
                    ext: "zip".to_string(),
                    os: "windows".to_string(),
                    version: "22.0.1".to_string(),
                },
            ),
        ] {
            assert_eq!(meta_from_name(actual).unwrap(), expected);
        }

        for invalid_name in [
            "jdk-21_linux-aarch64_bin.tar.gz",               // Missing graalvm prefix
            "graalvm-jdk-21.0.4_linux_bin.tar.gz",           // Missing architecture
            "graalvm-jdk-21.0.4_linux-aarch64.tar.gz",       // Missing '_bin' in name
            "graalvm-jdk-21.0.4_linux-aarch64_bin.unknown",  // Unsupported extension
            "graalvm-jdk-21.0.4_unknown-aarch64_bin.tar.gz", // Unsupported OS
            "graalvm-jdk-21.0.4_linux-unknown_bin.tar.gz",   // Unsupported architecture
        ] {
            assert!(
                meta_from_name(invalid_name).is_err(),
                "Expected an error for invalid file name: {}",
                invalid_name
            );
        }
    }
}
