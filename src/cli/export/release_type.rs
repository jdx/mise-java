use std::{collections::HashMap, fs::File, path::PathBuf};

use eyre::Result;
use log::info;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde_json::{Map, Value};

use crate::{
    config::Conf,
    db::{jvm_repository::JvmRepository, pool::ConnectionPool},
    jvm::JvmData,
};

use super::get_filter_map;

/// Export by {release_type}/{os}/{architecture}
///
/// Will export JSON files in form of {release_type}/{os}/{arch}.json to the path specified in the configuration file
/// or ROAST_EXPORT_PATH environment variable
#[derive(Debug, clap::Args)]
#[clap(verbatim_doc_comment)]
pub struct ReleaseType {
    /// Release types e.g.: ea, ga
    #[clap(short = 't', long, num_args = 0.., value_delimiter = ',', value_name = "TYPE")]
    pub release_type: Option<Vec<String>>,
    /// Operating systems e.g.: linux, macosx, windows
    #[clap(short = 'o', long, num_args = 0.., value_delimiter = ',', value_name = "OS")]
    pub os: Option<Vec<String>>,
    /// Architectures e.g.: aarch64, arm32, x86_64
    #[clap(short = 'a', long, num_args = 0.., value_delimiter = ',', value_name = "ARCH")]
    pub arch: Option<Vec<String>>,
    /// Properties to include e.g.: checksum, features, release_type, vendor, version
    #[clap(short = 'i', long, num_args = 0.., value_delimiter = ',', value_name = "PROPERTY")]
    pub include: Option<Vec<String>>,
    /// Properties to exclude e.g.: architecture, os, size
    #[clap(short = 'e', long, num_args = 0.., value_delimiter = ',', value_name = "PROPERTY")]
    pub exclude: Option<Vec<String>>,
    /// Filters to apply to the data e.g.: file_type=tar.gz,zip&features=musl,javafx,!lite
    ///
    /// Filters are separated with '&' and values are separated with ','. The filter will match if
    /// any of the values match unless the filter is negated with '!'. For example features=musl,javafx,!lite
    /// matches entries where the array `features` include musl or javafx but not lite. This is mostly useful for
    /// arrays that can contain multiple values.
    #[clap(short = 'f', long, num_args = 0.., value_delimiter = '&', value_name = "FILTER")]
    pub filters: Option<Vec<String>>,
    /// Pretty print JSON
    #[clap(long, default_value = "false")]
    pub pretty: bool,
}

impl ReleaseType {
    pub fn run(self) -> Result<()> {
        let conf = Conf::try_get()?;
        if conf.export.path.is_none() {
            return Err(eyre::eyre!("export.path is not configured"));
        }
        let conn_pool = ConnectionPool::get_pool()?;
        let db = JvmRepository::new(conn_pool)?;

        let release_types_default = db.get_distinct("release_type")?;
        let release_types = self.release_type.unwrap_or(release_types_default);

        let oses_default = db.get_distinct("os")?;
        let oses = self.os.unwrap_or(oses_default);

        let arch_default = db.get_distinct("architecture")?;
        let archs = self.arch.unwrap_or(arch_default);

        let include = self.include.unwrap_or_default();
        let exclude = self.exclude.unwrap_or_default();

        let filters = get_filter_map(self.filters.unwrap_or_default());

        let export_path = conf.export.path.unwrap();

        for release_type in &release_types {
            for os in &oses {
                for arch in &archs {
                    let data = with_openjdk_ea_aliases(db.export_release_type(release_type, arch, os)?);

                    let export_data = data
                        .into_par_iter()
                        .filter(|item| JvmData::filter(item, &filters))
                        .map(|item| JvmData::map(&item, &include, &exclude))
                        .collect::<Vec<Map<String, Value>>>();
                    let size = export_data.len();

                    info!("exporting {size} records to {release_type}/{os}/{arch}.json");
                    let path = PathBuf::from(&export_path)
                        .join(release_type)
                        .join(os)
                        .join(format!("{arch}.json"));
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }

                    let file = File::create(path)?;
                    match self.pretty {
                        true => serde_json::to_writer_pretty(file, &export_data)?,
                        false => serde_json::to_writer(file, &export_data)?,
                    }
                }
            }
        }
        Ok(())
    }
}

fn with_openjdk_ea_aliases(mut data: Vec<JvmData>) -> Vec<JvmData> {
    let existing = data
        .iter()
        .filter(|item| item.vendor == "openjdk")
        .map(|item| item.version.as_str())
        .collect::<std::collections::HashSet<&str>>();
    let mut aliases: HashMap<String, (u64, JvmData)> = HashMap::new();

    for item in &data {
        if item.vendor != "openjdk" || item.release_type != "ea" {
            continue;
        }
        let Some((alias, build)) = item.version.split_once('+') else {
            continue;
        };
        if !alias.ends_with("-ea") || existing.contains(alias) {
            continue;
        }
        let Ok(build) = build.parse::<u64>() else {
            continue;
        };

        let mut alias_item = item.clone();
        alias_item.java_version = alias.to_string();
        alias_item.version = alias.to_string();
        aliases
            .entry(alias.to_string())
            .and_modify(|(existing_build, existing_item)| {
                if build > *existing_build {
                    *existing_build = build;
                    *existing_item = alias_item.clone();
                }
            })
            .or_insert((build, alias_item));
    }

    data.extend(aliases.into_values().map(|(_, item)| item));
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn openjdk(version: &str) -> JvmData {
        JvmData {
            architecture: "x86_64".to_string(),
            file_type: "tar.gz".to_string(),
            image_type: "jdk".to_string(),
            java_version: version.to_string(),
            jvm_impl: "hotspot".to_string(),
            os: "linux".to_string(),
            release_type: "ea".to_string(),
            url: format!("https://example.com/openjdk-{version}.tar.gz"),
            vendor: "openjdk".to_string(),
            version: version.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_with_openjdk_ea_aliases() {
        let data = with_openjdk_ea_aliases(vec![openjdk("28.0.0-ea+1")]);
        let versions = data.iter().map(|item| item.version.as_str()).collect::<HashSet<_>>();

        assert!(versions.contains("28.0.0-ea+1"));
        assert!(versions.contains("28.0.0-ea"));
    }

    #[test]
    fn test_with_openjdk_ea_aliases_keeps_existing_alias() {
        let data = with_openjdk_ea_aliases(vec![openjdk("28.0.0-ea+1"), openjdk("28.0.0-ea")]);
        let aliases = data.iter().filter(|item| item.version == "28.0.0-ea").count();

        assert_eq!(aliases, 1);
    }

    #[test]
    fn test_with_openjdk_ea_aliases_uses_highest_build() {
        let data = with_openjdk_ea_aliases(vec![openjdk("28.0.0-ea+1"), openjdk("28.0.0-ea+2")]);
        let alias = data.iter().find(|item| item.version == "28.0.0-ea").unwrap();

        assert_eq!(alias.url, "https://example.com/openjdk-28.0.0-ea+2.tar.gz");
    }
}
