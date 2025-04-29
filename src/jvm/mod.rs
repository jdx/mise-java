use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

pub mod vendor;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct JvmData {
    pub architecture: String,
    pub checksum: Option<String>,
    pub checksum_url: Option<String>,
    pub features: Option<Vec<String>>,
    pub file_type: String,
    pub filename: String,
    pub image_type: String,
    pub java_version: String,
    pub jvm_impl: String,
    pub os: String,
    pub release_type: String,
    pub size: Option<i32>,
    pub url: String,
    pub vendor: String,
    pub version: String,
}

// ensure this matches the UNIQUE constraint in the database
impl Hash for JvmData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.url.hash(state);
    }
}

// ensure this matches the UNIQUE constraint in the database
impl PartialEq for JvmData {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}

impl Eq for JvmData {}

impl JvmData {
    pub fn filter(item: &JvmData, filters: &HashMap<String, Vec<String>>) -> bool {
        if filters.is_empty() {
            return true;
        }
        for (prop, values) in filters {
            if !JvmData::matches(item, prop, values) {
                return false;
            }
        }
        true
    }

    pub fn map(item: &JvmData, include: &[String], exclude: &[String]) -> Map<String, Value> {
        let props: HashMap<String, Value> = serde_json::from_value(serde_json::to_value(item).unwrap()).unwrap();
        let mut map = Map::new();
        for prop in &props {
            if (include.is_empty() || include.contains(prop.0)) && !exclude.contains(prop.0) {
                map.insert(prop.0.clone(), json!(prop.1.clone()));
            }
        }
        map
    }

    fn matches(item: &JvmData, key: &str, values: &[String]) -> bool {
        let props: HashMap<String, Value> = serde_json::from_value(serde_json::to_value(item).unwrap()).unwrap();
        let eq = values
            .iter()
            .filter_map(|v| if !v.starts_with("!") { Some(v.to_string()) } else { None })
            .collect::<Vec<String>>();
        let neq = values
            .iter()
            .filter_map(|v| v.strip_prefix("!").map(|v| v.to_string()))
            .collect::<Vec<String>>();
        if let Some(v) = props.get(key) {
            match v {
                Value::String(s) => eq.contains(s) && !neq.contains(s),
                Value::Number(n) => n
                    .as_i64()
                    .is_some_and(|i| eq.contains(&i.to_string()) && !neq.contains(&i.to_string())),
                Value::Bool(b) => eq.contains(&b.to_string()) && !neq.contains(&b.to_string()),
                Value::Array(arr) => {
                    eq.iter().any(|v| arr.contains(&Value::String(v.to_string())))
                        && !neq.iter().any(|v| arr.contains(&Value::String(v.to_string())))
                }
                _ => true,
            }
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_jvmdata() -> JvmData {
        JvmData {
            architecture: "x86_64".to_string(),
            checksum: Some("sha256:checksum".to_string()),
            checksum_url: Some("http://example.com/checksum".to_string()),
            features: Some(vec!["feature1".to_string(), "feature2".to_string()]),
            file_type: "tar.gz".to_string(),
            filename: "openjdk.tar.gz".to_string(),
            image_type: "jdk".to_string(),
            java_version: "11".to_string(),
            jvm_impl: "hotspot".to_string(),
            os: "linux".to_string(),
            release_type: "ga".to_string(),
            size: Some(12345678),
            url: "http://example.com/download".to_string(),
            vendor: "AdoptOpenJDK".to_string(),
            version: "11.0.2".to_string(),
        }
    }

    #[test]
    fn test_filter() {
        let jvm_data = get_jvmdata();

        assert!(JvmData::filter(
            &jvm_data,
            &HashMap::from([("os".to_string(), vec!["linux".to_string()])])
        ));
        assert!(!JvmData::filter(
            &jvm_data,
            &HashMap::from([("os".to_string(), vec!["!linux".to_string()])])
        ));
        assert!(JvmData::filter(
            &jvm_data,
            &HashMap::from([
                ("os".to_string(), vec!["linux".to_string()]),
                ("architecture".to_string(), vec!["x86_64".to_string()])
            ])
        ));
        assert!(!JvmData::filter(
            &jvm_data,
            &HashMap::from([("architecture".to_string(), vec!["aarch64".to_string()])])
        ));

        assert!(JvmData::filter(
            &jvm_data,
            &HashMap::from([("features".to_string(), vec!["feature1".to_string()])])
        ));
        assert!(!JvmData::filter(
            &jvm_data,
            &HashMap::from([("features".to_string(), vec!["feature3".to_string()])])
        ));
        assert!(JvmData::filter(
            &jvm_data,
            &HashMap::from([(
                "features".to_string(),
                vec!["feature1".to_string(), "!feature3".to_string()]
            )])
        ));
        assert!(!JvmData::filter(
            &jvm_data,
            &HashMap::from([(
                "features".to_string(),
                vec!["feature1".to_string(), "!feature2".to_string()]
            )])
        ));

        let mut jvm_data_nofeature = jvm_data.clone();
        jvm_data_nofeature.features = None;
        assert!(JvmData::filter(
            &jvm_data_nofeature,
            &HashMap::from([("features".to_string(), vec!["feature1".to_string()])])
        ));
    }

    #[test]
    fn test_map_with_all_properties() {
        let jvm_data = get_jvmdata();

        let include = vec![
            "architecture".to_string(),
            "checksum".to_string(),
            "checksum_url".to_string(),
            "features".to_string(),
            "file_type".to_string(),
            "filename".to_string(),
            "image_type".to_string(),
            "java_version".to_string(),
            "jvm_impl".to_string(),
            "os".to_string(),
            "release_type".to_string(),
            "size".to_string(),
            "url".to_string(),
            "vendor".to_string(),
            "version".to_string(),
        ];

        let map = JvmData::map(&jvm_data, &include, &[]);

        assert_eq!(map.get("architecture").unwrap(), "x86_64");
        assert_eq!(map.get("checksum").unwrap(), "sha256:checksum");
        assert_eq!(map.get("checksum_url").unwrap(), "http://example.com/checksum");
        assert_eq!(map.get("features").unwrap(), &json!(vec!["feature1", "feature2"]));
        assert_eq!(map.get("file_type").unwrap(), "tar.gz");
        assert_eq!(map.get("filename").unwrap(), "openjdk.tar.gz");
        assert_eq!(map.get("image_type").unwrap(), "jdk");
        assert_eq!(map.get("java_version").unwrap(), "11");
        assert_eq!(map.get("jvm_impl").unwrap(), "hotspot");
        assert_eq!(map.get("os").unwrap(), "linux");
        assert_eq!(map.get("release_type").unwrap(), "ga");
        assert_eq!(map.get("size").unwrap(), 12345678);
        assert_eq!(map.get("url").unwrap(), "http://example.com/download");
        assert_eq!(map.get("vendor").unwrap(), "AdoptOpenJDK");
        assert_eq!(map.get("version").unwrap(), "11.0.2");
    }

    #[test]
    fn test_map_with_include() {
        let jvm_data = get_jvmdata();
        let include = vec![
            "architecture".to_string(),
            "file_type".to_string(),
            "os".to_string(),
            "url".to_string(),
            "version".to_string(),
        ];

        let map = JvmData::map(&jvm_data, &include, &[]);

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

    #[test]
    fn test_map_with_exclude() {
        let jvm_data = get_jvmdata();
        let exclude = vec!["architecture".to_string(), "os".to_string(), "size".to_string()];

        let map = JvmData::map(&jvm_data, &[], &exclude);

        assert!(map.get("architecture").is_none());
        assert_eq!(map.get("checksum").unwrap(), "sha256:checksum");
        assert_eq!(map.get("checksum_url").unwrap(), "http://example.com/checksum");
        assert_eq!(map.get("features").unwrap(), &json!(vec!["feature1", "feature2"]));
        assert_eq!(map.get("file_type").unwrap(), "tar.gz");
        assert_eq!(map.get("filename").unwrap(), "openjdk.tar.gz");
        assert_eq!(map.get("image_type").unwrap(), "jdk");
        assert_eq!(map.get("java_version").unwrap(), "11");
        assert_eq!(map.get("jvm_impl").unwrap(), "hotspot");
        assert!(map.get("os").is_none());
        assert_eq!(map.get("release_type").unwrap(), "ga");
        assert!(map.get("size").is_none());
        assert_eq!(map.get("url").unwrap(), "http://example.com/download");
        assert_eq!(map.get("vendor").unwrap(), "AdoptOpenJDK");
        assert_eq!(map.get("version").unwrap(), "11.0.2");
    }
}
