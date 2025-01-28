use eyre::Result;
use log::info;
use rusqlite::params;

use crate::meta::{self, vendor::Vendor};

#[derive(Debug, clap::Args)]
#[clap(verbatim_doc_comment, after_long_help = AFTER_LONG_HELP)]
pub struct Fetch {}

impl Fetch {
    pub fn run(self) -> Result<()> {
        // TODO: fetch data from all vendors
        // TODO: parallelize fetching
        let vendors: Vec<Box<dyn Vendor>> = vec![
            // Box::new(meta::vendor::adoptopenjdk::AdoptOpenJDK {}),
            // Box::new(meta::vendor::corretto::Corretto {}),
            Box::new(meta::vendor::microsoft::Microsoft {}),
            // Box::new(meta::vendor::temurin::Temurin {}),
            // Box::new(meta::vendor::zulu::Zulu {}),
        ];

        for vendor in vendors {
            let name = vendor.get_name();
            info!("[{}] fetching data", name);
            let meta_data = vendor.fetch()?;

            // Write to JSON file
            info!("[{}] writing to JSON", name);
            store_json(&meta_data, &format!("data/{name}.json"))?;
            info!("[{}] writing to SQLite", name);
            // store_sqlite(&meta_data, "data/meta.sqlite3")?;
        }

        Ok(())
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

$ <bold>jmdb crawl</bold>
"#
);
