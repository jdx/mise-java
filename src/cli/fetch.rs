use std::thread;

use eyre::Result;
use log::{error, info};

use crate::meta::{self, vendor::Vendor};

#[derive(Debug, clap::Args)]
#[clap(verbatim_doc_comment, after_long_help = AFTER_LONG_HELP)]
pub struct Fetch {}

impl Fetch {
    pub fn run(self) -> Result<()> {
        // TODO: implement all vendors
        let vendors: Vec<Box<dyn Vendor>> = vec![
            Box::new(meta::vendor::adoptopenjdk::AdoptOpenJDK {}),
            Box::new(meta::vendor::corretto::Corretto {}),
            Box::new(meta::vendor::graalvm::GraalVM {}),
            Box::new(meta::vendor::liberica::Liberica {}),
            Box::new(meta::vendor::microsoft::Microsoft {}),
            Box::new(meta::vendor::openjdk::OpenJDK {}),
            Box::new(meta::vendor::oracle::Oracle {}),
            Box::new(meta::vendor::temurin::Temurin {}),
            Box::new(meta::vendor::zulu::Zulu {}),
        ];

        info!("fetching all vendors");
        let start = std::time::Instant::now();
        let mut tasks = vec![];
        for vendor in vendors {
            let task = thread::Builder::new()
                .name(vendor.get_name())
                .spawn(move || {
                    let name = vendor.get_name();
                    info!("[{}] fetching meta data", name);
                    let meta_data = match vendor.fetch() {
                        Ok(data) => data,
                        Err(err) => {
                            error!("[{}] failed to fetch meta data: {}", name, err);
                            return;
                        }
                    };

                    // Write to JSON file
                    info!("[{}] writing to JSON", name);
                    match store_json(&meta_data, &format!("data/{name}.json")) {
                        Ok(_) => {}
                        Err(err) => {
                            error!("[{}] failed to write to JSON: {}", name, err);
                            return;
                        }
                    };
                    // info!("[{}] writing to SQLite", name);
                    // store_sqlite(&meta_data, "data/meta.sqlite3")?;
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
}

fn store_json(meta_data: &Vec<meta::JavaMetaData>, json_path: &str) -> Result<()> {
    let json = serde_json::to_string_pretty(&meta_data)?;
    std::fs::write(json_path, json)?;

    Ok(())
}

// fn store_sqlite(meta_data: &Vec<meta::JavaMetaData>, db_path: &str) -> Result<()> {
//     let mut conn = rusqlite::Connection::open(db_path)?;
//
//     let tx = conn.transaction()?;
//     {
//         let mut stmt = tx.prepare(
//             "INSERT OR REPLACE INTO
//             JAVA_META_DATA
//               (architecture, features, file_type, filename, image_type, java_version, jvm_impl, md5, os, release_type, sha1, sha256, sha512, size, url, vendor, version)
//             VALUES
//               (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
//         )?;
//
//         for data in meta_data {
//             let features = data.features.clone().unwrap_or_default().join(",");
//
//             stmt.execute(params![
//                 data.architecture,
//                 features,
//                 data.file_type,
//                 data.filename,
//                 data.image_type,
//                 data.java_version,
//                 data.jvm_impl,
//                 data.md5,
//                 data.os,
//                 data.release_type,
//                 data.sha1,
//                 data.sha256,
//                 data.sha512,
//                 data.size,
//                 data.url,
//                 data.vendor,
//                 data.version,
//             ])?;
//         }
//     }
//     tx.commit()?;
//
//     Ok(())
// }

static AFTER_LONG_HELP: &str = color_print::cstr!(
    r#"<bold><underline>Examples:</underline></bold>

$ <bold>jmdb crawl</bold>
"#
);
