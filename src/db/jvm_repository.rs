use std::collections::HashSet;

use crate::jvm::JvmData;
use eyre::Result;
use indoc::indoc;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::types::ToSql;

const BATCH_SIZE: usize = 1000;

pub struct JvmRepository {
    pool: Pool<SqliteConnectionManager>,
}

impl JvmRepository {
    pub fn new(pool: Pool<SqliteConnectionManager>) -> Result<Self> {
        Ok(JvmRepository { pool })
    }

    pub fn insert(&self, jvm_data: &HashSet<JvmData>) -> Result<u64> {
        let mut conn = self.pool.get()?;
        let mut result = 0;
        let tx = conn.transaction()?;
        let columns = 15;

        for chunk in map_workaround(jvm_data).chunks(BATCH_SIZE) {
            let mut query = String::from(
                "INSERT INTO JVM
                (architecture, checksum, checksum_url, features, file_type, filename, image_type, java_version, jvm_impl, os, release_type, size, url, vendor, version)
                VALUES "
            );

            let mut params: Vec<&dyn ToSql> = Vec::new();
            for (i, data) in chunk.iter().enumerate() {
                if i > 0 {
                    query.push(',');
                }
                query.push_str(&format!(
                    "(?{}, ?{}, ?{}, ?{}, ?{}, ?{}, ?{}, ?{}, ?{}, ?{}, ?{}, ?{}, ?{}, ?{}, ?{})",
                    i * columns + 1,
                    i * columns + 2,
                    i * columns + 3,
                    i * columns + 4,
                    i * columns + 5,
                    i * columns + 6,
                    i * columns + 7,
                    i * columns + 8,
                    i * columns + 9,
                    i * columns + 10,
                    i * columns + 11,
                    i * columns + 12,
                    i * columns + 13,
                    i * columns + 14,
                    i * columns + 15
                ));
                params.push(&data.architecture);
                params.push(&data.checksum);
                params.push(&data.checksum_url);
                params.push(&data.features);
                params.push(&data.file_type);
                params.push(&data.filename);
                params.push(&data.image_type);
                params.push(&data.java_version);
                params.push(&data.jvm_impl);
                params.push(&data.os);
                params.push(&data.release_type);
                params.push(&data.size);
                params.push(&data.url);
                params.push(&data.vendor);
                params.push(&data.version);
            }

            query.push_str(
                " ON CONFLICT(url) DO UPDATE SET
                architecture = excluded.architecture,
                checksum = excluded.checksum,
                checksum_url = excluded.checksum_url,
                features = excluded.features,
                file_type = excluded.file_type,
                filename = excluded.filename,
                image_type = excluded.image_type,
                java_version = excluded.java_version,
                jvm_impl = excluded.jvm_impl,
                modified_at = CURRENT_TIMESTAMP,
                os = excluded.os,
                release_type = excluded.release_type,
                size = excluded.size,
                url = excluded.url,
                vendor = excluded.vendor,
                version = excluded.version
                WHERE
                   excluded.architecture IS NOT JVM.architecture
                OR excluded.checksum IS NOT JVM.checksum
                OR excluded.checksum_url IS NOT JVM.checksum_url
                OR excluded.features IS NOT JVM.features
                OR excluded.file_type IS NOT JVM.file_type
                OR excluded.filename IS NOT JVM.filename
                OR excluded.image_type IS NOT JVM.image_type
                OR excluded.java_version IS NOT JVM.java_version
                OR excluded.jvm_impl IS NOT JVM.jvm_impl
                OR excluded.os IS NOT JVM.os
                OR excluded.release_type IS NOT JVM.release_type
                OR excluded.size IS NOT JVM.size
                OR excluded.url IS NOT JVM.url
                OR excluded.vendor IS NOT JVM.vendor
                OR excluded.version IS NOT JVM.version
                ;",
            );

            result += tx.execute(&query, params.as_slice())? as u64;
        }

        tx.commit()?;
        Ok(result)
    }

    pub fn export_release_type(&self, release_type: &str, arch: &str, os: &str) -> Result<Vec<JvmData>> {
        let stmt = indoc! {
          "SELECT
              architecture,
              checksum,
              checksum_url,
              created_at,
              features,
              file_type,
              filename,
              image_type,
              java_version,
              jvm_impl,
              os,
              release_type,
              size,
              url,
              vendor,
              version
          FROM
              JVM_VIEW
          WHERE
              release_type = ?1
              AND os = ?2
              AND architecture = ?3
          ORDER BY
              url
          ;",
        };

        self.export(stmt, &[&release_type, &os, &arch])
    }

    pub fn export_vendor(&self, vendor: &str, os: &str, arch: &str) -> Result<Vec<JvmData>> {
        let stmt = indoc::indoc! {
          "SELECT
              architecture,
              checksum,
              checksum_url,
              created_at,
              features,
              file_type,
              filename,
              image_type,
              java_version,
              jvm_impl,
              os,
              release_type,
              size,
              url,
              vendor,
              version
          FROM
              JVM_VIEW
          WHERE
              vendor = ?1
              AND os = ?2
              AND architecture = ?3
          ORDER BY
              url
          ;"
        };

        self.export(stmt, &[&vendor, &os, &arch])
    }

    fn export(&self, query: &str, params: &[&dyn ToSql]) -> Result<Vec<JvmData>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(query)?;
        let mut data = Vec::new();
        let mut rows = stmt.query(params)?;
        while let Some(row) = rows.next()? {
            data.push(JvmData {
                architecture: row.get("architecture")?,
                checksum: row.get("checksum")?,
                checksum_url: row.get("checksum_url")?,
                created_at: row.get("created_at")?,
                features: decode_features(row.get::<_, Option<String>>("features")?),
                file_type: row.get("file_type")?,
                filename: row.get("filename")?,
                image_type: row.get("image_type")?,
                java_version: row.get("java_version")?,
                jvm_impl: row.get("jvm_impl")?,
                os: row.get("os")?,
                release_type: row.get("release_type")?,
                size: row.get::<_, Option<i32>>("size")?,
                url: row.get("url")?,
                vendor: row.get("vendor")?,
                version: row.get("version")?,
            });
        }
        Ok(data)
    }

    pub fn get_distinct(&self, column: &str) -> Result<Vec<String>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT DISTINCT {column} FROM JVM_VIEW ORDER BY {column} ASC;"
        ))?;
        let mut data = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            data.push(row.get::<usize, String>(0)?);
        }
        Ok(data)
    }
}

#[derive(Clone, Default, Debug)]
struct DbJvmData {
    pub architecture: String,
    pub checksum: Option<String>,
    pub checksum_url: Option<String>,
    pub features: Option<String>,
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

fn map_workaround(jvm_data: &HashSet<JvmData>) -> Vec<DbJvmData> {
    jvm_data
        .iter()
        // workaround for the `feature` field which needs to be joined
        // and therefore would not live long enough in context of a
        // batch insert
        .map(|item| DbJvmData {
            architecture: item.architecture.clone(),
            checksum: item.checksum.clone(),
            checksum_url: item.checksum_url.clone(),
            features: encode_features(&item.features),
            file_type: item.file_type.clone(),
            filename: item.filename.clone(),
            image_type: item.image_type.clone(),
            java_version: item.java_version.clone(),
            jvm_impl: item.jvm_impl.clone(),
            os: item.os.clone(),
            release_type: item.release_type.clone(),
            size: item.size,
            url: item.url.clone(),
            vendor: item.vendor.clone(),
            version: item.version.clone(),
        })
        .collect::<Vec<DbJvmData>>()
}

fn encode_features(features: &Option<Vec<String>>) -> Option<String> {
    let features = features
        .as_ref()?
        .iter()
        .filter(|feature| !feature.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    if features.is_empty() {
        None
    } else {
        Some(features.join(","))
    }
}

fn decode_features(features: Option<String>) -> Option<Vec<String>> {
    let features = features?
        .split(',')
        .filter(|feature| !feature.is_empty())
        .map(String::from)
        .collect::<Vec<_>>();
    if features.is_empty() { None } else { Some(features) }
}

#[cfg(test)]
mod tests {
    use super::{decode_features, encode_features};

    #[test]
    fn empty_features_are_not_encoded_as_an_empty_string() {
        assert_eq!(encode_features(&None), None);
        assert_eq!(encode_features(&Some(vec![])), None);
        assert_eq!(encode_features(&Some(vec!["".to_string()])), None);
        assert_eq!(
            encode_features(&Some(vec!["musl".to_string(), "javafx".to_string()])),
            Some("musl,javafx".to_string())
        );
    }

    #[test]
    fn blank_db_features_are_not_exported_as_a_feature() {
        assert_eq!(decode_features(None), None);
        assert_eq!(decode_features(Some("".to_string())), None);
        assert_eq!(decode_features(Some(",".to_string())), None);
        assert_eq!(
            decode_features(Some("musl,javafx".to_string())),
            Some(vec!["musl".to_string(), "javafx".to_string()])
        );
    }
}
