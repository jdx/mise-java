use eyre::Result;
use log::info;
use rusqlite::params;

use crate::{config::Conf, meta::JavaMetaData};

pub struct Sqlite {}

impl Sqlite {
    pub fn insert(vendor: &str, meta_data: &Vec<JavaMetaData>) -> Result<()> {
        let conf = Conf::try_get()?;
        if !conf.sqlite.path.is_some() {
            return Ok(());
        }
        let database_url = conf.sqlite.path.unwrap();

        info!(
            "[{}] writing to SQLite [database_url=sqlite://{}]",
            vendor, database_url
        );

        let mut conn = rusqlite::Connection::open(database_url)?;
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
        "INSERT INTO JAVA_META_DATA
              (architecture, features, file_type, filename, image_type, java_version, jvm_impl, md5, os, release_type, sha1, sha256, sha512, size, url, vendor, version)
            VALUES
              (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            ON CONFLICT(url) DO UPDATE SET
              architecture = excluded.architecture,
              features = excluded.features,
              file_type = excluded.file_type,
              filename = excluded.filename,
              image_type = excluded.image_type,
              java_version = excluded.java_version,
              jvm_impl = excluded.jvm_impl,
              md5 = excluded.md5,
              modified_at = CURRENT_TIMESTAMP,
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

            let mut result = 0;
            for data in meta_data {
                let features = match &data.features {
                    Some(values) if !values.is_empty() => Some(values.join(",")),
                    _ => None,
                };
                result += stmt.execute(params![
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
            info!("[{}] inserted/modified {} records", vendor, result);
        }
        tx.commit()?;

        Ok(())
    }
}
