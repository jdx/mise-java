use eyre::Result;
use indoc::formatdoc;
use itertools::Itertools;
use log::{debug, warn};
use serde::{Deserialize, Serialize};

use crate::{http::HTTP, meta::JavaMetaData};
use xx::regex;

use super::{normalize_architecture, normalize_os, normalize_version, Vendor};

#[derive(Clone, Copy, Debug)]
pub struct Zulu {}

impl Vendor for Zulu {
    fn get_name(&self) -> String {
        "zulu".to_string()
    }

    fn fetch_metadata(&self, meta_data: &mut Vec<JavaMetaData>) -> Result<()> {
        let mut page = 1;
        let page_size = 1000;
        let mut all_packages: Vec<Package> = Vec::new();
        loop {
            let api_url = formatdoc! {"https://api.azul.com/metadata/v1/zulu/packages
              ?availability_types=ca
              &release_status=both
              &page_size={page_size}
              &include_fields=arch,archive_type,java_package_features,java_package_type,lib_c_type,os,release_status,sha256_hash,size
              &page={page}",
              page = page, page_size = page_size,
            };
            debug!("[zulu] fetching packages at {}", api_url);
            match HTTP.get_json::<Vec<Package>>(api_url.as_str()) {
                Ok(packages) => {
                    all_packages.extend(packages);
                    page += 1;
                }
                Err(_) => break,
            }
        }
        meta_data.extend(map_packages(all_packages)?);
        Ok(())
    }
}

fn map_packages(packages: Vec<Package>) -> Result<Vec<JavaMetaData>> {
    let mut meta_data: Vec<JavaMetaData> = Vec::new();
    for package in packages {
        let arch = match arch_from_name(&package.name) {
            Ok(arch) => arch,
            Err(_) => {
                warn!("[zulu] failed to parse architecture for: {}", &package.name);
                &package.arch
            }
        };
        let architecture = normalize_architecture(&arch);
        let release_type = &package.release_status;
        let features = normalize_features(&package);
        let os = normalize_os(&package.os);
        let java_version = package.java_version.iter().map(|n| n.to_string()).join(".");
        let version = normalize_version(
            package
                .distro_version
                .iter()
                .map(|n| n.to_string())
                .join(".")
                .as_str(),
        );

        let meta = JavaMetaData {
            architecture,
            file_type: package.archive_type,
            features: Some(features),
            filename: package.name,
            image_type: package.java_package_type,
            java_version,
            jvm_impl: "hotspot".to_string(),
            os,
            release_type: release_type.to_string(),
            sha256: Some(package.sha256_hash),
            size: Some(package.size),
            url: package.download_url,
            vendor: "zulu".to_string(),
            version,
            ..Default::default()
        };
        meta_data.push(meta);
    }
    Ok(meta_data)
}

fn arch_from_name(name: &str) -> Result<&str> {
    debug!("[zulu] parsing name: {}", name);
    let capture = regex!(r"^.*[._-](aarch32hf|aarch32sf|aarch64|amd64|arm64|musl_aarch64|i386|i686|musl_x64|ppc32hf|ppc32spe|ppc64|sparcv9|x64|x86_64|x86lx64)\..*$")
        .captures(name)
        .ok_or_else(|| eyre::eyre!("regular expression failed for name: {}", name))?;

    let arch = capture.get(1).unwrap().as_str();
    Ok(arch)
}

fn normalize_features(package: &Package) -> Vec<String> {
    let mut features = Vec::new();
    match package.javafx_bundled {
        Some(true) => features.push("javafx".to_string()),
        _ => {}
    }
    match package.crac_supported {
        Some(true) => features.push("crac".to_string()),
        _ => {}
    }
    match &package.lib_c_type {
        Some(lib_c_type) => match lib_c_type.as_str() {
            "musl" => features.push("musl".to_string()),
            _ => {}
        },
        None => {}
    }
    features
}

#[derive(Debug, Deserialize, Serialize)]
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
