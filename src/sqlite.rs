use eyre::Result;
use rusqlite::params;

use crate::{config::Conf, meta::JavaMetaData};

pub struct Sqlite {}

impl Sqlite {
    pub fn insert(meta_data: &Vec<JavaMetaData>) -> Result<usize> {
        let mut conn = get_connection()?;
        let mut result = 0;
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
                  excluded.architecture != JAVA_META_DATA.architecture
              OR excluded.features != JAVA_META_DATA.features
              OR excluded.file_type != JAVA_META_DATA.file_type
              OR excluded.filename != JAVA_META_DATA.filename
              OR excluded.image_type != JAVA_META_DATA.image_type
              OR excluded.java_version != JAVA_META_DATA.java_version
              OR excluded.md5 != JAVA_META_DATA.md5
              OR excluded.md5_url != JAVA_META_DATA.md5_url
              OR excluded.release_type != JAVA_META_DATA.release_type
              OR excluded.sha1 != JAVA_META_DATA.sha1
              OR excluded.sha1_url != JAVA_META_DATA.sha1_url
              OR excluded.sha256 != JAVA_META_DATA.sha256
              OR excluded.sha256_url != JAVA_META_DATA.sha256_url
              OR excluded.sha512 != JAVA_META_DATA.sha512
              OR excluded.sha512_url != JAVA_META_DATA.sha512_url
              OR excluded.size != JAVA_META_DATA.size
              OR excluded.url != JAVA_META_DATA.url
              OR excluded.vendor != JAVA_META_DATA.vendor
              OR excluded.version != JAVA_META_DATA.version
            ;"
          )?;

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
        }
        tx.commit()?;

        Ok(result)
    }

    pub fn export(release_type: &str, arch: &str, os: &str) -> Result<Vec<JavaMetaData>> {
        let conn = get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT
                architecture,
                features,
                file_type,
                filename,
                image_type,
                java_version,
                jvm_impl,
                md5,
                md5_url,
                os,
                release_type,
                sha1,
                sha1_url,
                sha256,
                sha256_url,
                sha512,
                sha512_url,
                size,
                url,
                vendor,
                version
            FROM
                JAVA_META_DATA
            WHERE
                    file_type IN ('tar.gz','zip')
                AND release_type = ?1
                AND os = ?2
                AND architecture = ?3
            ;",
        )?;

        let mut data = Vec::new();
        let mut rows = stmt.query(params![release_type, os, arch])?;
        while let Some(row) = rows.next()? {
            data.push(JavaMetaData {
                architecture: row.get(0)?,
                features: row
                    .get::<_, Option<String>>(1)?
                    .map(|f| f.split(',').map(String::from).collect()),
                file_type: row.get(2)?,
                filename: row.get(3)?,
                image_type: row.get(4)?,
                java_version: row.get(5)?,
                jvm_impl: row.get(6)?,
                md5: row.get(7)?,
                md5_url: row.get(8)?,
                os: row.get(9)?,
                release_type: row.get(10)?,
                sha1: row.get(11)?,
                sha1_url: row.get(12)?,
                sha256: row.get(13)?,
                sha256_url: row.get(14)?,
                sha512: row.get(15)?,
                sha512_url: row.get(16)?,
                size: row.get(17)?,
                url: row.get(18)?,
                vendor: row.get(19)?,
                version: row.get(20)?,
                ..Default::default()
            });
        }
        Ok(data)
    }

    pub fn get_distinct(column: &str) -> Result<Vec<String>> {
        let conn = get_connection()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT DISTINCT {} FROM JAVA_META_DATA ORDER BY {} ASC;",
            column, column
        ))?;
        let mut data = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            data.push(row.get::<usize, String>(0)?);
        }
        Ok(data)
    }
}

fn get_connection() -> Result<rusqlite::Connection> {
    let conf = Conf::try_get()?;
    if !conf.sqlite.path.is_some() {
        return Err(eyre::eyre!("SQLite is not configured"));
    }
    let database_url = conf.sqlite.path.unwrap();
    let conn = rusqlite::Connection::open(database_url)?;
    Ok(conn)
}
