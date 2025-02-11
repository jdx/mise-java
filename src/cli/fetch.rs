use std::{collections::HashMap, thread};

use eyre::Result;
use log::{error, info};
use rusqlite::params;

use crate::{
    config::Conf,
    meta::{
        self,
        vendor::{Vendor, VENDORS},
    },
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

                    let conf = match Conf::try_get() {
                        Ok(conf) => conf,
                        Err(err) => {
                            error!("[{}] failed to load configuration: {}", name, err);
                            return;
                        }
                    };

                    // TODO move to JSON module
                    if let Some(json_path) = conf.json.path {
                        info!("[{}] writing to JSON", name);
                        match store_json(
                            &meta_data,
                            &format!("{path}{name}.json", path = json_path, name = name),
                        ) {
                            Ok(_) => {}
                            Err(err) => {
                                error!("[{}] failed to write to JSON: {}", name, err);
                                return;
                            }
                        }
                    }

                    // TODO move to DB module
                    if let Some(db_path) = conf.sqlite.path {
                        info!("[{}] writing to SQLite", name);
                        match store_sqlite(&meta_data, &db_path) {
                            Ok(_) => {}
                            Err(err) => {
                                error!("[{}] failed to write to SQLite: {}", name, err);
                                return;
                            }
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
        let mut stmt = tx.prepare(
      "INSERT INTO JAVA_META_DATA
            (architecture, features, file_type, filename, image_type, java_version, jvm_impl, md5, os, release_type, sha1, sha256, sha512, size, url, vendor, version)
          VALUES
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
          ON CONFLICT(architecture, os, vendor, version) DO UPDATE SET
            architecture = excluded.architecture,
            features = excluded.features,
            file_type = excluded.file_type,
            filename = excluded.filename,
            image_type = excluded.image_type,
            java_version = excluded.java_version,
            jvm_impl = excluded.jvm_impl,
            md5 = excluded.md5,
            os = excluded.os,
            release_type = excluded.release_type,
            sha1 = excluded.sha1,
            sha256 = excluded.sha256,
            sha512 = excluded.sha512,
            size = excluded.size,
            vendor = excluded.vendor,
            version = excluded.version
          WHERE
               excluded.md5 != JAVA_META_DATA.md5
            OR excluded.md5_url != JAVA_META_DATA.md5_url
            OR excluded.sha1 != JAVA_META_DATA.sha1
            OR excluded.sha1_url != JAVA_META_DATA.sha1_url
            OR excluded.sha256 != JAVA_META_DATA.sha256
            OR excluded.sha256_url != JAVA_META_DATA.sha256_url
            OR excluded.sha512 != JAVA_META_DATA.sha512
            OR excluded.sha512_url != JAVA_META_DATA.sha512_url
            OR excluded.url != JAVA_META_DATA.url
          ;"
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

$ <bold>jmeta fetch</bold>
"#
);
