use std::collections::HashSet;

use eyre::Result;
use indoc::formatdoc;
use log::debug;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use serde::{Deserialize, Serialize};

use crate::{http::HTTP, jvm::JvmData};

use super::{Vendor, get_extension, normalize_architecture, normalize_os, normalize_version};

#[derive(Clone, Copy, Debug)]
pub struct Temurin {}

impl Vendor for Temurin {
    fn get_name(&self) -> String {
        "temurin".to_string()
    }

    fn fetch_data(&self, jvm_data: &mut HashSet<JvmData>) -> Result<()> {
        // get available releases
        // https://api.adoptium.net/v3/info/available_releases
        let api_releases_url = "https://api.adoptium.net/v3/info/available_releases";
        debug!("[temurin] fetching releases [{}]", api_releases_url);
        let releases = HTTP.get_json::<AvailableReleases, _>(api_releases_url)?;

        // get meta data for a specific release
        // https://api.adoptium.net/v3/assets/feature_releases/${release}/ga?page=${page}&page_size=20&project=jdk&sort_order=ASC&vendor=adoptium
        let data = releases
            .available_releases
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
                        &vendor=eclipse",
                        page = page, page_size = page_size, release = release,
                    };
                    debug!("[temurin] fetching release [{}] page [{}]", release, page);
                    match HTTP.get_json::<Vec<Release>, _>(api_url) {
                        Ok(resp) => {
                            resp.iter().for_each(|release| {
                                let release_data: Vec<JvmData> = map_release(release)
                                    .into_iter()
                                    .filter(|m| !["sbom"].contains(&m.image_type.as_str()))
                                    .collect::<Vec<JvmData>>();
                                data.extend(release_data)
                            });
                            page += 1;
                        }
                        Err(_) => break,
                    }
                }
                data
            })
            .collect::<Vec<JvmData>>();
        jvm_data.extend(data);
        Ok(())
    }
}

fn normalize_features(binary: Binary) -> Option<Vec<String>> {
    let mut features = Vec::new();
    if binary.heap_size == "large" {
        features.push("large_heap".to_string());
    }
    if binary.os == "alpine-linux" || binary.c_lib.as_deref() == Some("musl") {
        features.push("musl".to_string());
    }
    if features.is_empty() { None } else { Some(features) }
}

fn map_release(release: &Release) -> Vec<JvmData> {
    let mut jvm_data = Vec::new();
    for binary in &release.binaries {
        let package = binary.package.clone();
        let package_checksum = package.as_ref().and_then(|p| p.checksum.clone());
        let package_checksum_link = package.as_ref().and_then(|p| p.checksum_link.clone());
        let package_link = package.as_ref().map(|p| p.link.clone());
        let package_name = package.as_ref().map(|p| p.name.clone());
        let package_extension = package_name.as_ref().map(|p| get_extension(p));

        let java_jvm_data = JvmData {
            architecture: normalize_architecture(binary.architecture.as_str()),
            checksum: package_checksum.and_then(|c| format!("sha256:{}", c).into()),
            checksum_url: package_checksum_link,
            image_type: binary.image_type.clone(),
            features: normalize_features(binary.clone()),
            file_type: package_extension.unwrap_or_default().to_string(),
            filename: package_name.unwrap_or_default().to_string(),
            java_version: release.version_data.openjdk_version.clone().to_string(),
            jvm_impl: binary.jvm_impl.clone(),
            os: normalize_os(binary.os.as_str()),
            size: Some(package.as_ref().map(|p| p.size as i32).unwrap_or(0)),
            release_type: release.release_type.clone().to_string(),
            url: package_link.unwrap_or_default().to_string(),
            vendor: "temurin".to_string(),
            version: normalize_version(release.version_data.semver.clone().as_str()),
        };
        jvm_data.push(java_jvm_data);
    }
    jvm_data
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
    c_lib: Option<String>,
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

#[cfg(test)]
mod tests {
    use crate::jvm::vendor::temurin::{Binary, normalize_features};

    #[test]
    fn test_normalize_features() {
        for (values, expected) in [
            (
                (None, "large".to_string(), "linux".to_string()),
                Some(vec!["large_heap".to_string()]),
            ),
            ((None, "normal".to_string(), "linux".to_string()), None),
            (
                (None, "normal".to_string(), "alpine-linux".to_string()),
                Some(vec!["musl".to_string()]),
            ),
            (
                (Some("musl".to_string()), "normal".to_string(), "linux".to_string()),
                Some(vec!["musl".to_string()]),
            ),
        ] {
            let binary = Binary {
                architecture: "x64".to_string(),
                c_lib: values.0,
                heap_size: values.1,
                image_type: "jdk".to_string(),
                installer: None,
                jvm_impl: "temurin".to_string(),
                os: values.2,
                package: None,
            };
            let actual = normalize_features(binary.clone());
            assert_eq!(expected, actual);
        }
    }
}
