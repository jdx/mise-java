use std::collections::HashSet;

use eyre::Result;
use indoc::formatdoc;
use itertools::Itertools;
use log::debug;
use serde::{Deserialize, Serialize};

use crate::{http::HTTP, jvm::JvmData};
use xx::regex;

use super::{Vendor, normalize_architecture, normalize_os, normalize_version};

#[derive(Clone, Copy, Debug)]
pub struct Zulu {}

impl Vendor for Zulu {
    fn get_name(&self) -> String {
        "zulu".to_string()
    }

    fn fetch_data(&self, jvm_data: &mut HashSet<JvmData>) -> Result<()> {
        let mut page = 1;
        let page_size = 1000;
        let mut all_packages: Vec<Package> = Vec::new();
        loop {
            let api_url = formatdoc! {"https://api.azul.com/metadata/v1/zulu/packages
              ?availability_types=ca
              &release_status=both
              &page_size={page_size}
              &include_fields=arch,archive_type,crac_supported,javafx_bundled,java_package_features,java_package_type,lib_c_type,os,release_status,sha256_hash,size
              &page={page}",
              page = page, page_size = page_size,
            };
            debug!("[zulu] fetching packages at {}", api_url);
            match HTTP.get_json::<Vec<Package>, _>(api_url) {
                Ok(packages) => {
                    all_packages.extend(packages);
                    page += 1;
                }
                Err(_) => break,
            }
        }
        jvm_data.extend(map_packages(all_packages)?);
        Ok(())
    }
}

fn map_packages(packages: Vec<Package>) -> Result<Vec<JvmData>> {
    let mut jvm_data: Vec<JvmData> = Vec::new();
    for package in packages {
        let arch = match arch_from_name(&package.name) {
            Ok(arch) => arch,
            Err(_) => {
                debug!("[zulu] failed to parse architecture for: {}", &package.name);
                &package.arch
            }
        };
        let architecture = normalize_architecture(arch);
        let release_type = &package.release_status;
        let features = normalize_features(&package);
        let os = normalize_os(&package.os);
        let java_version = package.java_version.iter().map(|n| n.to_string()).join(".");
        let version = normalize_version(package.distro_version.iter().map(|n| n.to_string()).join(".").as_str());

        let meta = JvmData {
            architecture,
            checksum: Some(format!("sha256:{}", package.sha256_hash)),
            file_type: package.archive_type,
            features,
            filename: package.name,
            image_type: package.java_package_type,
            java_version,
            jvm_impl: "hotspot".to_string(),
            os,
            release_type: release_type.to_string(),
            size: Some(package.size as i32),
            url: package.download_url,
            vendor: "zulu".to_string(),
            version,
            ..Default::default()
        };
        jvm_data.push(meta);
    }
    Ok(jvm_data)
}

fn arch_from_name(name: &str) -> Result<&str> {
    debug!("[zulu] parsing name: {}", name);
    let capture = regex!(r"^.*[._-](aarch32hf|aarch32sf|aarch64|amd64|arm64|musl_aarch64|i386|i686|musl_x64|ppc32hf|ppc32spe|ppc64|sparcv9|x64|x86_64|x86lx32|x86lx64)\..*$")
        .captures(name)
        .ok_or_else(|| eyre::eyre!("regular expression failed for name: {}", name))?;

    let arch = capture.get(1).unwrap().as_str();
    Ok(arch)
}

fn normalize_features(package: &Package) -> Option<Vec<String>> {
    let mut features = Vec::new();
    if let Some(true) = package.javafx_bundled {
        features.push("javafx".to_string());
    }
    if let Some(true) = package.crac_supported {
        features.push("crac".to_string());
    }
    if let Some(lib_c_type) = &package.lib_c_type {
        if lib_c_type == "musl" {
            features.push("musl".to_string());
        }
    }
    match features.is_empty() {
        true => None,
        false => Some(features),
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct Package {
    arch: String,
    archive_type: String,
    availability_type: String,
    crac_supported: Option<bool>,
    distro_version: Vec<u64>,
    download_url: String,
    javafx_bundled: Option<bool>,
    java_package_features: Vec<String>,
    java_package_type: String,
    java_version: Vec<u64>,
    lib_c_type: Option<String>,
    name: String,
    os: String,
    release_status: String,
    sha256_hash: String,
    size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arch_from_name() {
        for (actual, expected) in [
            ("zulu11.1.8-ca-jdk11.0.0-linux_aarch64.tar.gz", "aarch64"),
            ("zulu10.1.11-ca-jdk10.0.0-linux_i686.zip", "i686"),
            ("zulu10.1.11-ca-jdk10.0.0-macosx_x64.zip", "x64"),
            ("zulu11.39.15-ca-fx-jdk11.0.7-win_x64.zip", "x64"),
            ("zre1.7.0_65-7.6.0.2-headless-x86lx32.zip", "x86lx32"),
        ] {
            let arch = arch_from_name(actual);
            assert!(arch.is_ok());
            assert_eq!(arch_from_name(actual).unwrap(), expected);
        }

        for actual in ["zulu1.8.0_66-8.11.0.1-macosx.tar.gz", "zulu1.7.0_79-7.9.0.2-win64.msi"] {
            let arch = arch_from_name(actual);
            assert!(arch.is_err())
        }
    }

    #[test]
    fn test_normalize_features() {
        for (actual, expected) in [
            (
                Package {
                    javafx_bundled: Some(true),
                    ..Default::default()
                },
                Some(vec!["javafx".to_string()]),
            ),
            (
                Package {
                    crac_supported: Some(true),
                    ..Default::default()
                },
                Some(vec!["crac".to_string()]),
            ),
            (
                Package {
                    lib_c_type: Some("musl".to_string()),
                    ..Default::default()
                },
                Some(vec!["musl".to_string()]),
            ),
            (
                Package {
                    javafx_bundled: Some(true),
                    crac_supported: Some(true),
                    lib_c_type: Some("musl".to_string()),
                    ..Default::default()
                },
                Some(vec!["javafx".to_string(), "crac".to_string(), "musl".to_string()]),
            ),
        ] {
            assert_eq!(normalize_features(&actual), expected);
        }
    }
}
