use std::{collections::HashSet, fs::File, path::PathBuf};

use eyre::Result;
use log::info;

use crate::{
    config::Conf,
    db::{jvm_repository::JvmRepository, pool::ConnectionPool},
    jvm::JvmData,
};

/// Import exported JVM JSON data into the database
#[derive(Debug, clap::Args)]
#[clap(verbatim_doc_comment)]
pub struct Import {}

impl Import {
    /// Imports JVM JSON files from the configured export directory into the database.
    ///
    /// Scans a directory tree rooted at `conf.export.path` (defaulting to `public/api/jvm/`)
    /// expecting the layout `<release_type>/<os>/<architecture>.json`. Each JSON file is
    /// deserialized to `JvmData`, enriched with `release_type`, `os`, and `architecture`,
    /// deduplicated, and inserted/updated via `JvmRepository`.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration lookup, filesystem access, JSON deserialization,
    /// database pool/repository creation, or the insert operation fail.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let cmd = Import {};
    /// cmd.run().unwrap();
    /// ```
    pub fn run(self) -> Result<()> {
        let conf = Conf::try_get()?;
        let export_path = conf.export.path.unwrap_or("public/api/jvm/".to_string());
        let root = PathBuf::from(export_path);
        let mut data = HashSet::new();

        for release_type_entry in std::fs::read_dir(&root)? {
            let release_type_entry = release_type_entry?;
            if !release_type_entry.file_type()?.is_dir() {
                continue;
            }
            let release_type = release_type_entry.file_name().to_string_lossy().to_string();
            for os_entry in std::fs::read_dir(release_type_entry.path())? {
                let os_entry = os_entry?;
                if !os_entry.file_type()?.is_dir() {
                    continue;
                }
                let os = os_entry.file_name().to_string_lossy().to_string();
                for arch_entry in std::fs::read_dir(os_entry.path())? {
                    let arch_entry = arch_entry?;
                    if arch_entry.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
                        continue;
                    }
                    let architecture = arch_entry.path().file_stem().unwrap().to_string_lossy().to_string();
                    let file = File::open(arch_entry.path())?;
                    let mut items: Vec<JvmData> = serde_json::from_reader(file)?;
                    for item in &mut items {
                        item.release_type.clone_from(&release_type);
                        item.os.clone_from(&os);
                        item.architecture.clone_from(&architecture);
                    }
                    data.extend(items);
                }
            }
        }

        let db = JvmRepository::new(ConnectionPool::get_pool()?)?;
        let result = db.insert(&data)?;
        info!("imported {} records, inserted/modified {result} records", data.len());
        Ok(())
    }
}
