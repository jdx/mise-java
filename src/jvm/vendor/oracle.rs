use std::collections::HashSet;

use crate::{
    http::HTTP,
    jvm::{JvmData, vendor::anchors_from_doc},
};
use eyre::Result;
use log::{debug, error, warn};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use scraper::{Html, Selector};
use xx::regex;

use super::{AnchorElement, Vendor, normalize_architecture, normalize_os, normalize_version};

#[derive(Clone, Copy, Debug)]
pub struct Oracle {}

#[derive(Debug, PartialEq)]
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

    fn fetch_data(&self, jvm_data: &mut HashSet<JvmData>) -> Result<()> {
        let anchors: Vec<AnchorElement> = build_urls()
            .into_par_iter()
            .flat_map(|url| {
                let releases_html = match HTTP.get_text(&url) {
                    Ok(releases_html) => releases_html,
                    Err(e) => {
                        error!("[oracle] error fetching releases: {e}");
                        "".to_string()
                    }
                };
                let document = Html::parse_document(&releases_html);
                let latest_versions = extract_latest_versions(&document);
                anchors_from_doc(&document, "a:is([href$='.dep'], [href$='.dmg'], [href$='.exe'], [href$='.msi'], [href$='.rpm'], [href$='.tar.gz'], [href$='.zip'])")
                  .into_iter()
                  .map(|mut anchor|  {
                    replace_with_latest_version(&mut anchor, &latest_versions);
                    anchor
                }).collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let data = anchors
            .into_par_iter()
            .filter(|a| !a.href.contains("graalvm-"))
            .flat_map(|anchor| match map_release(&anchor) {
                Ok(release) => vec![release],
                Err(e) => {
                    warn!("[oracle] {e}");
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
    let url = a.href.clone().replace("/latest/", "/archive/");
    let filename_meta = meta_from_name(&name)?;
    let sha256_url = format!("{}.sha256", &url);
    let sha256 = match HTTP.get_text(&sha256_url) {
        Ok(sha256) => sha256.split_whitespace().next().map(|s| format!("sha256:{s}")),
        Err(_) => {
            warn!("[oracle] unable to find SHA256 for {name}");
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
        url: url.clone(),
        version: normalize_version(&filename_meta.version),
        vendor: "oracle".to_string(),
        ..Default::default()
    })
}

fn extract_latest_versions(document: &Html) -> Vec<String> {
    let mut versions = HashSet::new();
    let h_id = Selector::parse("h3[id^='java']").unwrap();
    document.select(&h_id).for_each(|h| {
        let text = h.text().collect::<String>();
        let re = regex!(r"(?<version>\d+\.\d+\.\d+)");
        for cap in re.captures_iter(text.as_str()) {
            if let Some(version) = cap.name("version") {
                versions.insert(version.as_str().to_string());
            }
        }
    });
    versions.into_iter().collect()
}

fn replace_with_latest_version(anchor: &mut AnchorElement, latest_versions: &[String]) {
    if anchor.href.contains("/latest/") {
        anchor.name = latest_versions
            .iter()
            .find(|v| {
                let major = v.split('.').next().unwrap_or("");
                anchor.name.contains(major)
            })
            .map(|v| {
                let major = v.split('.').next().unwrap_or("");
                anchor.name.replace(&format!("jdk-{major}_"), &format!("jdk-{}_", &v))
            })
            .unwrap_or_else(|| anchor.name.clone());
    }
}

fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[oracle] parsing name: {name}");
    let capture =
        regex!(r"^jdk-([0-9+.]{2,})_(linux|macos|windows)-(x64|aarch64)_bin\.(dep|dmg|exe|msi|rpm|tar\.gz|zip)$")
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
    for version in 17..=25 {
        urls.push(format!(
            "https://www.oracle.com/java/technologies/javase/jdk{version}-archive-downloads.html"
        ));
    }
    urls
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_replace_with_latest_version() {
        for (actual, expected) in [
            (
                "https://download.oracle.com/java/24/latest/jdk-24_linux-aarch64_bin.tar.gz",
                "https://download.oracle.com/java/24/latest/jdk-24.0.1_linux-aarch64_bin.tar.gz",
            ),
            (
                "https://download.oracle.com/java/21/latest/jdk-21_linux-aarch64_bin.tar.gz",
                "https://download.oracle.com/java/21/latest/jdk-21.0.7_linux-aarch64_bin.tar.gz",
            ),
        ] {
            let mut anchor = AnchorElement {
                href: actual.to_string(),
                name: actual.to_string(),
            };
            let latest_versions = vec!["21.0.7".to_string(), "24.0.1".to_string()];
            replace_with_latest_version(&mut anchor, &latest_versions);
            assert_eq!(anchor.name, expected);
        }
    }

    #[test]
    fn test_extract_latest_versions() {
        let html = r#"
            <html>
                <body>
                    <h3 id="java24">Java SE Development Kit 24.0.1 downloads</h3>
                    <h3 id="java21">Java SE Development Kit 21.0.7 downloads</h3>
                </body>
            </html>
        "#;
        let document = Html::parse_document(html);
        let versions = extract_latest_versions(&document);
        assert_ne!(versions.len(), 0);
        assert!(versions.contains(&"21.0.7".to_string()));
        assert!(versions.contains(&"24.0.1".to_string()));
    }

    #[test]
    fn test_meta_from_name() {
        for (actual, expected) in [
            (
                "jdk-17.0.7_linux-aarch64_bin.tar.gz",
                FileNameMeta {
                    arch: "aarch64".to_string(),
                    ext: "tar.gz".to_string(),
                    os: "linux".to_string(),
                    version: "17.0.7".to_string(),
                },
            ),
            (
                "jdk-21_macos-aarch64_bin.tar.gz",
                FileNameMeta {
                    arch: "aarch64".to_string(),
                    ext: "tar.gz".to_string(),
                    os: "macos".to_string(),
                    version: "21".to_string(),
                },
            ),
            (
                "jdk-23_windows-x64_bin.zip",
                FileNameMeta {
                    arch: "x64".to_string(),
                    ext: "zip".to_string(),
                    os: "windows".to_string(),
                    version: "23".to_string(),
                },
            ),
        ] {
            assert_eq!(meta_from_name(actual).unwrap(), expected);
        }

        for invalid_name in [
            "graalvm-jdk-21_linux-aarch64_bin.tar.gz", // Unexpected graalvm prefix
            "jdk-21.0.4_linux_bin.tar.gz",             // Missing architecture
            "jdk-21.0.4_linux-aarch64.tar.gz",         // Missing '_bin' in name
            "jdk-21.0.4_linux-aarch64_bin.unknown",    // Unsupported extension
            "jdk-21.0.4_unknown-aarch64_bin.tar.gz",   // Unsupported OS
            "jdk-21.0.4_linux-unknown_bin.tar.gz",     // Unsupported architecture
        ] {
            assert!(
                meta_from_name(invalid_name).is_err(),
                "Expected an error for invalid file name: {invalid_name}",
            );
        }
    }
}
