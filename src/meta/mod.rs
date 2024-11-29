use serde::{Deserialize, Serialize};

pub mod vendor;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct JavaMetaData {
    pub architecture: String,
    pub features: Option<Vec<String>>,
    pub file_type: String,
    pub filename: String,
    pub image_type: String,
    pub java_version: String,
    pub jvm_impl: String,
    pub md5: Option<String>,
    pub md5_file: Option<String>,
    pub os: String,
    pub release_type: String,
    pub sha1: Option<String>,
    pub sha1_file: Option<String>,
    pub sha256: String,
    pub sha256_file: Option<String>,
    pub sha512: Option<String>,
    pub sha512_file: Option<String>,
    pub size: u64,
    pub url: String,
    pub vendor: String,
    pub version: String,
}
