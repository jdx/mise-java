use std::{fs::File, path::PathBuf};

use eyre::Result;
use log::info;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde_json::{Map, Value};

use crate::{
    config::Conf,
    db::{meta_repository::MetaRepository, pool::ConnectionPool},
    jvm::JvmData,
};

/// Export as a triple {release_type}/{os}/{architecture}
///
/// Will export JSON files in form of <release_type>/<os>/<arch>.json to the path specified in the configuration file
/// or ROAST_EXPORT_PATH environment variable
#[derive(Debug, clap::Args)]
#[clap(verbatim_doc_comment)]
pub struct Triple {
    /// Release types e.g.: ea, ga
    #[clap(short = 't', long, num_args = 0.., value_delimiter = ',', value_name = "TYPE")]
    pub release_type: Option<Vec<String>>,
    /// Operating systems e.g.: linux, macosx, windows
    #[clap(short = 'o', long, num_args = 0.., value_delimiter = ',', value_name = "OS")]
    pub os: Option<Vec<String>>,
    /// Architectures e.g.: aarch64, arm32, x86_64
    #[clap(short = 'a', long, num_args = 0.., value_delimiter = ',', value_name = "ARCH")]
    pub arch: Option<Vec<String>>,
    /// Properties e.g.: architecture, os, vendor, version
    #[clap(short = 'p', long, num_args = 0.., value_delimiter = ',', value_name = "PROPERTY")]
    pub properties: Option<Vec<String>>,
    /// Pretty print JSON
    #[clap(long, default_value = "false")]
    pub pretty: bool,
}

impl Triple {
    pub fn run(self) -> Result<()> {
        let conf = Conf::try_get()?;
        if conf.export.path.is_none() {
            return Err(eyre::eyre!("export.path is not configured"));
        }
        let conn_pool = ConnectionPool::get_pool()?;
        let db = MetaRepository::new(conn_pool)?;

        let release_types_default = db.get_distinct("release_type")?;
        let release_types = self.release_type.unwrap_or(release_types_default);
        let oses_default = db.get_distinct("os")?;
        let oses = self.os.unwrap_or(oses_default);
        let arch_default = db.get_distinct("architecture")?;
        let archs = self.arch.unwrap_or(arch_default);

        let export_path = conf.export.path.unwrap();

        for release_type in &release_types {
            for os in &oses {
                for arch in &archs {
                    let data = db.export(release_type, arch, os)?;
                    let size = data.len();

                    let export_data = data
                        .into_par_iter()
                        .map(|item| JvmData::map(&item, &self.properties))
                        .collect::<Vec<Map<String, Value>>>();

                    info!("exporting {} records for {} {} {}", size, release_type, os, arch);
                    let path = PathBuf::from(&export_path)
                        .join(release_type)
                        .join(os)
                        .join(format!("{}.json", arch));
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
