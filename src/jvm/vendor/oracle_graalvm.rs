use std::collections::HashSet;

use eyre::Result;
use log::{debug, warn};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::Deserialize;
use xx::regex;

use crate::http::HTTP;

use super::{Vendor, normalize_architecture, normalize_os, normalize_version};
use crate::jvm::JvmData;

const INDEX_URL: &str = "https://www.oracle.com/a/tech/docs/graalvm-downloads.json";
const BASE_URL: &str = "https://www.oracle.com";

#[derive(Clone, Copy, Debug)]
pub struct OracleGraalVM {}

#[derive(Debug, Deserialize)]
struct ReleaseRef {
    #[serde(rename = "JSON File")]
    json_file: String,
}

#[derive(Debug, Deserialize)]
struct EntryData {
    #[serde(default)]
    #[serde(rename = "Title")]
    title: String,
    #[serde(default)]
    #[serde(rename = "Releases")]
    releases: std::collections::HashMap<String, ReleaseRef>,
}

#[derive(Debug, Deserialize)]
struct ReleaseData {
    #[serde(default)]
    #[serde(rename = "Release")]
    release: String,
    #[serde(default)]
    #[serde(rename = "Packages")]
    packages: std::collections::HashMap<String, PackageData>,
}

#[derive(Debug, Deserialize)]
struct PackageData {
    #[serde(default)]
    #[serde(rename = "Files")]
    files: std::collections::HashMap<String, FileData>,
}

#[derive(Debug, Deserialize)]
struct FileData {
    #[serde(rename = "File")]
    file: String,
    #[serde(default)]
    #[serde(rename = "Hash")]
    hash: Vec<String>,
}

#[derive(Debug, PartialEq)]
struct FileNameMeta {
    arch: String,
    ext: String,
    java_version: String,
    os: String,
}

impl Vendor for OracleGraalVM {
    fn get_name(&self) -> String {
        "oracle-graalvm".to_string()
    }

    fn fetch_data(&self, jvm_data: &mut HashSet<JvmData>) -> Result<()> {
        // 1. Fetch the index JSON
        let index_text = HTTP.get_text(INDEX_URL)?;
        let index: std::collections::HashMap<String, EntryData> = serde_json::from_str(&index_text)?;

        // 2. Collect all version-specific JSON file paths, skipping Enterprise entries
        let json_files: Vec<String> = index
            .values()
            .filter(|e| !e.title.contains("Enterprise"))
            .flat_map(|entry| entry.releases.values())
            .map(|r| format!("{BASE_URL}{}", r.json_file))
            .collect();

        debug!("[oracle-graalvm] found {} release JSON files", json_files.len());

        // 3. Fetch each version JSON in parallel and parse file data
        let data = json_files
            .into_par_iter()
            .flat_map(|url| {
                let release_json = match HTTP.get_text(&url) {
                    Ok(text) => text,
                    Err(e) => {
                        warn!("[oracle-graalvm] error fetching {url}: {e}");
                        return vec![];
                    }
                };
                let release: ReleaseData = match serde_json::from_str(&release_json) {
                    Ok(r) => r,
                    Err(e) => {
                        warn!("[oracle-graalvm] error parsing {url}: {e}");
                        return vec![];
                    }
                };
                map_release(&release)
            })
            .collect::<Vec<_>>();

        jvm_data.extend(data);
        Ok(())
    }
}

fn map_release(release: &ReleaseData) -> Vec<JvmData> {
    if release.release.is_empty() {
        debug!("[oracle-graalvm] skipping release with empty release field");
        return vec![];
    }

    let mut results = Vec::new();

    for (pkg_name, pkg) in &release.packages {
        // Only process Core packages (the actual JDK distributions)
        if pkg_name != "Core" {
            continue;
        }

        for (file_key, file_data) in &pkg.files {
            match map_file(file_key, file_data) {
                Ok(Some(jvm)) => results.push(jvm),
                Ok(None) => {}
                Err(e) => {
                    warn!("[oracle-graalvm] skipping {file_key}: {e}");
                }
            }
        }
    }

    results
}

fn map_file(file_key: &str, file_data: &FileData) -> Result<Option<JvmData>> {
    let url = &file_data.file;
    let filename = url
        .rsplit('/')
        .next()
        .ok_or_else(|| eyre::eyre!("no filename in URL: {url}"))?;

    // Parse filename to extract os, arch, and java version
    let meta = meta_from_name(filename)?;

    // Skip innovation releases (treated as early access)
    if regex!(r"graalvm-jdk-\d+i").is_match(filename) {
        debug!("[oracle-graalvm] skipping innovation release: {filename}");
        return Ok(None);
    }

    // Skip URLs that require authentication
    if url.contains("/otn/") {
        debug!("[oracle-graalvm] skipping authenticated URL: {url}");
        return Ok(None);
    }

    // Get SHA256 checksum
    let checksum_url = format!("{url}.sha256");
    let checksum = if file_data.hash.len() >= 2 && file_data.hash[0] == "SHA256" {
        Some(format!("sha256:{}", file_data.hash[1]))
    } else {
        warn!("[oracle-graalvm] no SHA256 hash for {file_key}");
        None
    };

    let file_type = meta.ext.clone();
    let arch = normalize_architecture(&meta.arch);
    let os = normalize_os(&meta.os);
    let java_version = normalize_version(&meta.java_version);

    debug!(
        "[oracle-graalvm] {}: version={java_version} os={os} arch={arch}",
        filename
    );

    Ok(Some(JvmData {
        architecture: arch,
        checksum,
        checksum_url: Some(checksum_url),
        features: None,
        filename: filename.to_string(),
        file_type,
        image_type: "jdk".to_string(),
        java_version: java_version.clone(),
        jvm_impl: "graalvm".to_string(),
        os,
        release_type: "ga".to_string(),
        url: url.to_string(),
        version: java_version,
        vendor: "oracle-graalvm".to_string(),
        ..Default::default()
    }))
}

/// Parse filename to extract os, arch, and java version.
///
/// Matches: `graalvm-jdk-{version}_{os}-{arch}_bin.{ext}`
fn meta_from_name(name: &str) -> Result<FileNameMeta> {
    debug!("[oracle-graalvm] parsing name: {name}");

    // Try modern Oracle GraalVM format first
    if let Some(caps) =
        regex!(r"^graalvm-jdk-([^_]+)_(linux|macos|windows)-(x64|aarch64)_bin\.(tar\.gz|zip)$").captures(name)
    {
        let version_raw = caps.get(1).unwrap().as_str();
        let os = caps.get(2).unwrap().as_str().to_string();
        let arch = caps.get(3).unwrap().as_str().to_string();
        let ext = caps.get(4).unwrap().as_str().to_string();

        // Extract the java version; for formats like "25i2-25.0.4" take
        // the part after the last dash (the actual Java version)
        let java_version = if let Some((_, after)) = version_raw.rsplit_once('-') {
            after.to_string()
        } else {
            version_raw.to_string()
        };

        return Ok(FileNameMeta {
            arch,
            ext,
            java_version,
            os,
        });
    }

    Err(eyre::eyre!("regular expression did not match for {}", name))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_meta_from_name_modern() {
        for (actual, expected) in [
            (
                "graalvm-jdk-21.0.12_linux-x64_bin.tar.gz",
                FileNameMeta {
                    arch: "x64".to_string(),
                    ext: "tar.gz".to_string(),
                    java_version: "21.0.12".to_string(),
                    os: "linux".to_string(),
                },
            ),
            (
                "graalvm-jdk-17.0.20_linux-aarch64_bin.tar.gz",
                FileNameMeta {
                    arch: "aarch64".to_string(),
                    ext: "tar.gz".to_string(),
                    java_version: "17.0.20".to_string(),
                    os: "linux".to_string(),
                },
            ),
            (
                "graalvm-jdk-25.0.4_linux-x64_bin.tar.gz",
                FileNameMeta {
                    arch: "x64".to_string(),
                    ext: "tar.gz".to_string(),
                    java_version: "25.0.4".to_string(),
                    os: "linux".to_string(),
                },
            ),
            (
                "graalvm-jdk-25.0.4_macos-aarch64_bin.tar.gz",
                FileNameMeta {
                    arch: "aarch64".to_string(),
                    ext: "tar.gz".to_string(),
                    java_version: "25.0.4".to_string(),
                    os: "macos".to_string(),
                },
            ),
            (
                "graalvm-jdk-25.0.4_windows-x64_bin.zip",
                FileNameMeta {
                    arch: "x64".to_string(),
                    ext: "zip".to_string(),
                    java_version: "25.0.4".to_string(),
                    os: "windows".to_string(),
                },
            ),
        ] {
            assert_eq!(meta_from_name(actual).unwrap(), expected, "Failed for: {actual}");
        }
    }

    #[test]
    fn test_meta_from_name_invalid() {
        for invalid_name in [
            "jdk-21_linux-aarch64_bin.tar.gz",               // Missing graalvm prefix
            "graalvm-jdk-21.0.4_linux_bin.tar.gz",           // Missing architecture
            "graalvm-jdk-21.0.4_linux-aarch64.tar.gz",       // Missing '_bin' in name
            "graalvm-jdk-21.0.4_unknown-aarch64_bin.tar.gz", // Unsupported OS
            "graalvm-jdk-21.0.4_linux-unknown_bin.tar.gz",   // Unsupported architecture
        ] {
            assert!(
                meta_from_name(invalid_name).is_err(),
                "Expected an error for invalid file name: {invalid_name}",
            );
        }
    }
}
