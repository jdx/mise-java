use eyre::Result;
use indoc::formatdoc;
use log::debug;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use serde::{Deserialize, Serialize};

use crate::{http::HTTP, meta::JavaMetaData};

use super::{get_extension, normalize_architecture, normalize_os, normalize_version, Vendor};

#[derive(Clone, Copy, Debug)]
pub struct Temurin {}

impl Vendor for Temurin {
    fn get_name(&self) -> String {
        "temurin".to_string()
    }

    fn fetch_metadata(&self, meta_data: &mut Vec<JavaMetaData>) -> Result<()> {
        // get available releases
        // https://api.adoptium.net/v3/info/available_releases
        let api_releases_url = "https://api.adoptium.net/v3/info/available_releases";
        debug!("[temurin] fetching releases [{}]", api_releases_url);
        let releases = HTTP.get_json::<AvailableReleases, _>(api_releases_url)?;

        // get meta data for a specific release
        // https://api.adoptium.net/v3/assets/feature_releases/${release}/ga?page=${page}&page_size=20&project=jdk&sort_order=ASC&vendor=adoptium
        let data = releases.available_releases
            .into_par_iter()
            .flat_map(|release| {
                let mut page = 0;
                let page_size = 1000;
                let mut data = Vec::new();

                loop {
                    let api_url = formatdoc! {"https://api.adoptium.net/v3/assets/feature_releases/{release}/ga
                        ?page={page}
                        &page_size={page_size}
                        &project=jdk
                        &sort_order=ASC
                        &vendor=adoptium",
                        page = page, page_size = page_size, release = release,
                    };
                    debug!("[temurin] fetching release [{}] page [{}]", release, page);
                    match HTTP.get_json::<Vec<Release>, _>(api_url) {
                        Ok(resp) => {
                          resp.iter().for_each(|release| {
                                let release_data: Vec<JavaMetaData> = map_release(release)
                                    .into_iter()
                                    .filter(|m| !["sbom"].contains(&m.image_type.as_str()))
                                    .collect::<Vec<JavaMetaData>>();
                                data.extend(release_data)
                          });
                          page += 1;
                        }
                        Err(_) => break,
                    }
                }
                data
            })
            .collect::<Vec<JavaMetaData>>();
        meta_data.extend(data);
        Ok(())
    }
}

fn normalize_features(features: &str) -> Option<Vec<String>> {
    match features {
        "large" => Some(vec!["large_heap".to_string()]),
        _ => None,
    }
}

fn map_release(release: &Release) -> Vec<JavaMetaData> {
    let mut meta_data = Vec::new();
    for binary in &release.binaries {
        let package = binary.package.clone();
        let package_checksum = package.as_ref().and_then(|p| p.checksum.clone());
        let package_checksum_link = package.as_ref().and_then(|p| p.checksum_link.clone());
        let package_link = package.as_ref().map(|p| p.link.clone());
        let package_name = package.as_ref().map(|p| p.name.clone());
        let package_extension = package_name.as_ref().map(|p| get_extension(p));

        let java_meta_data = JavaMetaData {
            architecture: normalize_architecture(binary.architecture.as_str()),
            image_type: binary.image_type.clone(),
            features: normalize_features(binary.heap_size.clone().as_str()),
            file_type: package_extension.unwrap_or_default().to_string(),
            filename: package_name.unwrap_or_default().to_string(),
            java_version: release.version_data.openjdk_version.clone().to_string(),
            jvm_impl: binary.jvm_impl.clone(),
            os: normalize_os(binary.os.as_str()),
            sha256: package_checksum,
            sha256_url: package_checksum_link,
            size: Some(package.as_ref().map(|p| p.size as i32).unwrap_or(0)),
            release_type: release.release_type.clone().to_string(),
            url: package_link.unwrap_or_default().to_string(),
            vendor: "temurin".to_string(),
            version: normalize_version(release.version_data.semver.clone().as_str()),
            ..Default::default()
        };
        meta_data.push(java_meta_data);
    }
    meta_data
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AvailableReleases {
    available_lts_releases: Vec<u8>,
    available_releases: Vec<u8>,
    most_recent_feature_release: u8,
    most_recent_feature_version: u8,
    most_recent_lts: u8,
    tip_version: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Release {
    binaries: Vec<Binary>,
    release_name: String,
    release_type: String,
    updated_at: String,
    version_data: VersionData,
    vendor: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct VersionData {
    openjdk_version: String,
    semver: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Binary {
    architecture: String,
    heap_size: String,
    image_type: String,
    installer: Option<Installer>,
    jvm_impl: String,
    os: String,
    package: Option<Package>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Installer {
    checksum: Option<String>,
    checksum_link: Option<String>,
    link: String,
    name: String,
    size: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Package {
    checksum: Option<String>,
    checksum_link: Option<String>,
    link: String,
    name: String,
    size: u64,
}
