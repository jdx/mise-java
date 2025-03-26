use std::collections::HashSet;

use crate::jvm::JvmData;
use eyre::Result;
use postgres_openssl::MakeTlsConnector;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;

const BATCH_SIZE: usize = 1000;

pub struct JvmRepository {
    pool: Pool<PostgresConnectionManager<MakeTlsConnector>>,
}

impl JvmRepository {
    pub fn new(pool: Pool<PostgresConnectionManager<MakeTlsConnector>>) -> Result<Self> {
        Ok(JvmRepository { pool })
    }

    pub fn insert(&self, jvm_data: &HashSet<JvmData>) -> Result<u64> {
        let mut conn = self.pool.get()?;
        let mut result = 0;
        let mut tx = conn.transaction()?;
        let columns = 15;

        for chunk in map_workaround(jvm_data).chunks(BATCH_SIZE) {
            let mut query = String::from(
                "INSERT INTO JVM
                (architecture, checksum, checksum_url, features, file_type, filename, image_type, java_version, jvm_impl, os, release_type, size, url, vendor, version)
                VALUES "
            );

            let mut params: Vec<&(dyn postgres::types::ToSql + Sync)> = Vec::new();
            for (i, data) in chunk.iter().enumerate() {
                if i > 0 {
                    query.push(',');
                }
                query.push_str(&format!(
                    "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
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
                " ON CONFLICT(vendor, version, os, architecture, image_type, file_type) DO UPDATE SET
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
                   excluded.architecture != JVM.architecture
                OR excluded.checksum != JVM.checksum
                OR excluded.checksum_url != JVM.checksum_url
                OR excluded.features != JVM.features
                OR excluded.file_type != JVM.file_type
                OR excluded.filename != JVM.filename
                OR excluded.image_type != JVM.image_type
                OR excluded.java_version != JVM.java_version
                OR excluded.jvm_impl != JVM.jvm_impl
                OR excluded.os != JVM.os
                OR excluded.release_type != JVM.release_type
                OR excluded.size != JVM.size
                OR excluded.url != JVM.url
                OR excluded.vendor != JVM.vendor
                OR excluded.version != JVM.version
                ;",
            );

            result += tx.execute(&query, &params)?;
        }

        tx.commit()?;
        Ok(result)
    }

    pub fn export_triple(&self, release_type: &str, arch: &str, os: &str) -> Result<Vec<JvmData>> {
        let mut conn = self.pool.get()?;
        let stmt = conn.prepare(
            "SELECT
                architecture,
                checksum,
                checksum_url,
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
                JVM
            WHERE
                    file_type IN ('tar.gz','zip')
                AND release_type = $1
                AND os = $2
                AND architecture = $3
            ORDER BY
                vendor,
                version,
                created_at
            DESC
            ;",
        )?;

        let mut data = Vec::new();
        let rows = conn.query(&stmt, &[&release_type, &os, &arch])?;
        for row in rows {
            data.push(JvmData {
                architecture: row.get("architecture"),
                checksum: row.get("checksum"),
                checksum_url: row.get("checksum_url"),
                features: row
                    .get::<_, Option<String>>("features")
                    .map(|f| f.split(',').map(String::from).collect()),
                file_type: row.get("file_type"),
                filename: row.get("filename"),
                image_type: row.get("image_type"),
                java_version: row.get("java_version"),
                jvm_impl: row.get("jvm_impl"),
                os: row.get("os"),
                release_type: row.get("release_type"),
                size: row.get::<_, Option<i32>>("size"),
                url: row.get("url"),
                vendor: row.get("vendor"),
                version: row.get("version"),
            });
        }
        Ok(data)
    }

    pub fn get_distinct(&self, column: &str) -> Result<Vec<String>> {
        let mut conn = self.pool.get()?;
        let stmt = conn.prepare(&format!("SELECT DISTINCT {} FROM JVM ORDER BY {} ASC;", column, column))?;
        let mut data = Vec::new();
        let rows = conn.query(&stmt, &[])?;
        for row in rows {
            data.push(row.get::<usize, String>(0));
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
            features: item.features.as_ref().map(|f| f.join(",")),
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
