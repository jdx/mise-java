use std::{collections::HashMap, thread};

use eyre::Result;
use log::{error, info};
use rusqlite::params;

use crate::meta::{
    self,
    vendor::{Vendor, VENDORS},
};

#[derive(Debug, clap::Args)]
#[clap(verbatim_doc_comment, after_long_help = AFTER_LONG_HELP)]
pub struct Fetch {
    /// Vendor(s) to fetch
    /// e.g.: openjdk, zulu
    #[clap(value_name = "VENDOR")]
    pub vendors: Vec<String>,
}

impl Fetch {
    pub fn run(self) -> Result<()> {
        if self.vendors.is_empty() {
            info!("fetching all vendors");
        } else {
            info!("fetching vendors: {:?}", self.vendors);
        }

        let start = std::time::Instant::now();
        let mut tasks = vec![];

        // TODO: use a thread pool here
        for (name, vendor) in self.get_vendors() {
            let task = thread::Builder::new()
                .name(name.clone())
                .spawn(move || {
                    info!("[{}] fetching meta data", name);
                    let meta_data = match vendor.fetch() {
                        Ok(data) => data,
                        Err(err) => {
                            error!("[{}] failed to fetch meta data: {}", name, err);
                            return;
                        }
                    };

                    // TODO move to JSON module
                    info!("[{}] writing to JSON", name);
                    match store_json(&meta_data, &format!("data/{name}.json")) {
                        Ok(_) => {}
                        Err(err) => {
                            error!("[{}] failed to write to JSON: {}", name, err);
                            return;
                        }
                    };
                    // TODO move to DB module
                    info!("[{}] writing to SQLite", name);
                    match store_sqlite(&meta_data, "data/meta.sqlite3") {
                        Ok(_) => {}
                        Err(err) => {
                            error!("[{}] failed to write to SQLite: {}", name, err);
                            return;
                        }
                    }
                })
                .unwrap();
            tasks.push(task);
        }

        for task in tasks {
            task.join().unwrap();
        }

        info!(
            "fetched all vendors in {:.2} seconds",
            start.elapsed().as_secs_f32()
        );
        Ok(())
    }

    fn get_vendors(&self) -> HashMap<String, &'static Box<dyn Vendor>> {
        VENDORS
            .iter()
            .map(|v| (v.get_name(), v))
            .filter(|(k, _v)| self.vendors.is_empty() || self.vendors.contains(k))
            .collect()
    }
}

fn store_json(meta_data: &Vec<meta::JavaMetaData>, json_path: &str) -> Result<()> {
    let json = serde_json::to_string_pretty(&meta_data)?;
    std::fs::write(json_path, json)?;

    Ok(())
}

fn store_sqlite(meta_data: &Vec<meta::JavaMetaData>, db_path: &str) -> Result<()> {
    let mut conn = rusqlite::Connection::open(db_path)?;

    let tx = conn.transaction()?;
    {
        // TODO change to a real UPSERT (INSERT ON CONFLICT)
        /*
        INSERT INTO
          TABLE ()
          VALUES ()
        ON CONFLICT(url) DO UPDATE SET
          md5=excluded.md5,
          sha1=excluded.sha1
          sha256=excluded.sha256
          sha512=excluded.sha512
        WHERE excluded.md5 != TABLE.md5
          OR excluded.sha1 != TABLE.sha1
          OR excluded.sha256 != TABLE.sha256
          OR excluded.sha512 != TABLE.sha512;
        */
        let mut stmt = tx.prepare(
            "INSERT OR REPLACE INTO
            JAVA_META_DATA
              (architecture, features, file_type, filename, image_type, java_version, jvm_impl, md5, os, release_type, sha1, sha256, sha512, size, url, vendor, version)
            VALUES
              (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        )?;

        for data in meta_data {
            let features = data.features.clone().unwrap_or_default().join(",");

            stmt.execute(params![
                data.architecture,
                features,
                data.file_type,
                data.filename,
                data.image_type,
                data.java_version,
                data.jvm_impl,
                data.md5,
                data.os,
                data.release_type,
                data.sha1,
                data.sha256,
                data.sha512,
                data.size,
                data.url,
                data.vendor,
                data.version,
            ])?;
        }
    }
    tx.commit()?;

    Ok(())
}

static AFTER_LONG_HELP: &str = color_print::cstr!(
    r#"<bold><underline>Examples:</underline></bold>

$ <bold>jmdb fetch</bold>
"#
);
