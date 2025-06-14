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
pub struct RedHat {}

impl Vendor for RedHat {
    fn get_name(&self) -> String {
        "redhat".to_string()
    }

    fn fetch_data(&self, jvm_data: &mut HashSet<JvmData>) -> Result<()> {
        // get available releases
        let api_releases_url = "https://marketplace-api.adoptium.net/v1/info/available_releases/redhat";
        debug!("[redhat] fetching releases [{}]", api_releases_url);
        let releases = HTTP.get_json::<AvailableReleases, _>(api_releases_url)?;

        // get meta data for a specific release
        let data = releases
            .available_releases
            .into_par_iter()
            .flat_map(|release| {
                let mut page = 0;
                let page_size = 1000;
                let mut data = Vec::new();

                loop {
                    let api_url = formatdoc! {"https://marketplace-api.adoptium.net/v1/assets/feature_releases/redhat/{release}
                        ?page={page}
                        &page_size={page_size}
                        &sort_order=ASC",
                        page = page, page_size = page_size, release = release,
                    };
                    debug!("[redhat] fetching release [{}] page [{}]", release, page);
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
                        Err(e) => {
                            debug!("[redhat] error fetching page for release [{}] {}", release, e);
                            break;
                        },
                    }
                }
                data
            })
            .collect::<Vec<JvmData>>();
        jvm_data.extend(data);
        Ok(())
    }
}

fn map_release(release: &Release) -> Vec<JvmData> {
    let mut jvm_data = Vec::new();
    for binary in &release.binaries {
        let mut artifacts = get_installer_artifacts(binary);
        artifacts.push(get_package_artifact(binary));

        let version = release
            .release_name
            .trim_start_matches("jdk-")
            .trim_start_matches("jdk");

        for artifact in artifacts {
            let java_jvm_data = JvmData {
                architecture: normalize_architecture(binary.architecture.as_str()),
                checksum: artifact.checksum.and_then(|c| format!("sha256:{}", c).into()),
                checksum_url: artifact.checksum_link,
                image_type: binary.image_type.clone(),
                features: None,
                file_type: artifact.extension.to_string(),
                filename: artifact.name.to_string(),
                java_version: release
                    .openjdk_version_data
                    .openjdk_version
                    .trim_start_matches("jdk")
                    .to_string(),
                jvm_impl: binary.jvm_impl.clone(),
                os: normalize_os(binary.os.as_str()),
                release_type: "ga".to_string(),
                url: artifact.link.to_string(),
                vendor: "redhat".to_string(),
                version: normalize_version(version),
                size: None,
            };
            jvm_data.push(java_jvm_data);
        }
    }
    jvm_data
}

fn get_package_artifact(binary: &Binary) -> BinaryArtifact {
    let checksum = binary.package.as_ref().and_then(|p| p.sha265sum.clone());
    let checksum_link = binary.package.as_ref().and_then(|p| p.sha265sum_link.clone());
    let link = binary.package.as_ref().map(|p| p.link.clone()).unwrap_or_default();
    let name = binary.package.as_ref().map(|p| p.name.clone()).unwrap_or_default();
    let extension = get_extension(&name);

    BinaryArtifact {
        checksum,
        checksum_link,
        link,
        name,
        extension,
    }
}

fn get_installer_artifacts(binary: &Binary) -> Vec<BinaryArtifact> {
    let mut artifacts = Vec::new();
    if let Some(installers) = &binary.installer {
        for installer in installers {
            let checksum = installer.sha265sum.clone();
            let checksum_link = installer.sha265sum_link.clone();
            let link = installer.link.clone();
            let name = installer.name.clone();
            let extension = get_extension(&name);

            artifacts.push(BinaryArtifact {
                checksum,
                checksum_link,
                link,
                name,
                extension,
            });
        }
    }
    artifacts
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AvailableReleases {
    available_lts_releases: Vec<u8>,
    available_releases: Vec<u8>,
    most_recent_feature_version: u8,
    most_recent_lts: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Release {
    binaries: Vec<Binary>,
    release_name: String,
    last_updated_timestamp: String,
    openjdk_version_data: VersionData,
    vendor: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct VersionData {
    openjdk_version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Binary {
    architecture: String,
    image_type: String,
    jvm_impl: String,
    os: String,
    package: Option<Package>,
    installer: Option<Vec<Installer>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Installer {
    sha265sum: Option<String>,
    sha265sum_link: Option<String>,
    link: String,
    name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Package {
    sha265sum: Option<String>,
    sha265sum_link: Option<String>,
    link: String,
    name: String,
}

struct BinaryArtifact {
    checksum: Option<String>,
    checksum_link: Option<String>,
    link: String,
    name: String,
    extension: String,
}
