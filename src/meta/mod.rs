use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::hash::Hash;
use std::{collections::HashMap, hash::Hasher};

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
    pub md5_url: Option<String>,
    pub os: String,
    pub release_type: String,
    pub sha1: Option<String>,
    pub sha1_url: Option<String>,
    pub sha256: Option<String>,
    pub sha256_url: Option<String>,
    pub sha512: Option<String>,
    pub sha512_url: Option<String>,
    pub size: Option<i32>,
    pub url: String,
    pub vendor: String,
    pub version: String,
}

impl Hash for JavaMetaData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.url.hash(state);
    }
}

impl PartialEq for JavaMetaData {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}

impl Eq for JavaMetaData {}

impl JavaMetaData {
    pub fn map(item: &JavaMetaData, properties: &Option<Vec<String>>) -> Map<String, Value> {
        let props: HashMap<String, Value> =
            serde_json::from_value(serde_json::to_value(item).unwrap()).unwrap();
        let mut map = Map::new();
        for prop in &props {
            match properties {
                Some(properties) => {
                    if properties.contains(prop.0) {
                        map.insert(prop.0.clone(), json!(prop.1.clone()));
                    }
                }
                None => {
                    map.insert(prop.0.clone(), json!(prop.1.clone()));
                }
            }
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_metadata() -> JavaMetaData {
        JavaMetaData {
            architecture: "x86_64".to_string(),
            features: Some(vec!["feature1".to_string(), "feature2".to_string()]),
            file_type: "tar.gz".to_string(),
            filename: "openjdk.tar.gz".to_string(),
            image_type: "jdk".to_string(),
            java_version: "11".to_string(),
            jvm_impl: "hotspot".to_string(),
            md5: Some("md5hash".to_string()),
            md5_url: Some("http://example.com/md5".to_string()),
            os: "linux".to_string(),
            release_type: "ga".to_string(),
            sha1: Some("sha1hash".to_string()),
            sha1_url: Some("http://example.com/sha1".to_string()),
            sha256: Some("sha256hash".to_string()),
            sha256_url: Some("http://example.com/sha256".to_string()),
            sha512: Some("sha512hash".to_string()),
            sha512_url: Some("http://example.com/sha512".to_string()),
            size: Some(12345678),
            url: "http://example.com/download".to_string(),
            vendor: "AdoptOpenJDK".to_string(),
            version: "11.0.2".to_string(),
        }
    }

    #[test]
    fn test_map_with_all_properties() {
        let metadata = get_metadata();

        let properties = Some(vec![
            "architecture".to_string(),
            "features".to_string(),
            "file_type".to_string(),
            "filename".to_string(),
            "image_type".to_string(),
            "java_version".to_string(),
            "jvm_impl".to_string(),
            "md5".to_string(),
            "md5_url".to_string(),
            "os".to_string(),
            "release_type".to_string(),
            "sha1".to_string(),
            "sha1_url".to_string(),
            "sha256".to_string(),
            "sha256_url".to_string(),
            "sha512".to_string(),
            "sha512_url".to_string(),
            "size".to_string(),
            "url".to_string(),
            "vendor".to_string(),
            "version".to_string(),
        ]);

        let map = JavaMetaData::map(&metadata, &properties);

        assert_eq!(map.get("architecture").unwrap(), "x86_64");
        assert_eq!(
            map.get("features").unwrap(),
            &json!(vec!["feature1", "feature2"])
        );
        assert_eq!(map.get("file_type").unwrap(), "tar.gz");
        assert_eq!(map.get("filename").unwrap(), "openjdk.tar.gz");
        assert_eq!(map.get("image_type").unwrap(), "jdk");
        assert_eq!(map.get("java_version").unwrap(), "11");
        assert_eq!(map.get("jvm_impl").unwrap(), "hotspot");
        assert_eq!(map.get("md5").unwrap(), "md5hash");
        assert_eq!(map.get("md5_url").unwrap(), "http://example.com/md5");
        assert_eq!(map.get("os").unwrap(), "linux");
        assert_eq!(map.get("release_type").unwrap(), "ga");
        assert_eq!(map.get("sha1").unwrap(), "sha1hash");
        assert_eq!(map.get("sha1_url").unwrap(), "http://example.com/sha1");
        assert_eq!(map.get("sha256").unwrap(), "sha256hash");
        assert_eq!(map.get("sha256_url").unwrap(), "http://example.com/sha256");
        assert_eq!(map.get("sha512").unwrap(), "sha512hash");
        assert_eq!(map.get("sha512_url").unwrap(), "http://example.com/sha512");
        assert_eq!(map.get("size").unwrap(), 12345678);
        assert_eq!(map.get("url").unwrap(), "http://example.com/download");
        assert_eq!(map.get("vendor").unwrap(), "AdoptOpenJDK");
        assert_eq!(map.get("version").unwrap(), "11.0.2");
    }

    #[test]
    fn test_map_with_some_properties() {
        let metadata = get_metadata();
        let properties = Some(vec![
            "architecture".to_string(),
            "file_type".to_string(),
            "os".to_string(),
            "url".to_string(),
            "version".to_string(),
        ]);

        let map = JavaMetaData::map(&metadata, &properties);

        assert_eq!(map.get("architecture").unwrap(), "x86_64");
        assert_eq!(map.get("file_type").unwrap(), "tar.gz");
        assert!(map.get("features").is_none());
        assert!(map.get("filename").is_none());
        assert!(map.get("image_type").is_none());
        assert!(map.get("java_version").is_none());
        assert!(map.get("jvm_impl").is_none());
        assert!(map.get("md5").is_none());
        assert!(map.get("md5_url").is_none());
        assert_eq!(map.get("os").unwrap(), "linux");
        assert!(map.get("release_type").is_none());
        assert!(map.get("sha1").is_none());
        assert!(map.get("sha1_url").is_none());
        assert!(map.get("sha256").is_none());
        assert!(map.get("sha256_url").is_none());
        assert!(map.get("sha512").is_none());
        assert!(map.get("sha512_url").is_none());
        assert!(map.get("size").is_none());
        assert_eq!(map.get("url").unwrap(), "http://example.com/download");
        assert!(map.get("vendor").is_none());
        assert_eq!(map.get("version").unwrap(), "11.0.2");
    }
}
