use std::collections::HashSet;

use crate::meta::JavaMetaData;
use eyre::Result;
use postgres_openssl::MakeTlsConnector;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;

const BATCH_SIZE: usize = 1000;

pub struct Postgres {
    pool: Pool<PostgresConnectionManager<MakeTlsConnector>>,
}

impl Postgres {
    pub fn new(pool: Pool<PostgresConnectionManager<MakeTlsConnector>>) -> Result<Self> {
        Ok(Postgres { pool })
    }

    pub fn insert(&self, meta_data: &HashSet<JavaMetaData>) -> Result<u64> {
        let mut conn = self.pool.get()?;
        let mut result = 0;
        let mut tx = conn.transaction()?;

        for chunk in map_workaround(meta_data).chunks(BATCH_SIZE) {
            let mut query = String::from(
                "INSERT INTO JAVA_META_DATA
                (architecture, features, file_type, filename, image_type, java_version, jvm_impl, md5, md5_url, os, release_type, sha1, sha1_url, sha256, sha256_url, sha512, sha512_url, size, url, vendor, version)
                VALUES "
            );
            let mut params: Vec<&(dyn postgres::types::ToSql + Sync)> = Vec::new();
            for (i, data) in chunk.iter().enumerate() {
                if i > 0 {
                    query.push(',');
                }
                query.push_str(&format!(
                    "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
                    i * 21 + 1, i * 21 + 2, i * 21 + 3, i * 21 + 4, i * 21 + 5, i * 21 + 6, i * 21 + 7, i * 21 + 8, i * 21 + 9, i * 21 + 10,
                    i * 21 + 11, i * 21 + 12, i * 21 + 13, i * 21 + 14, i * 21 + 15, i * 21 + 16, i * 21 + 17, i * 21 + 18, i * 21 + 19, i * 21 + 20, i * 21 + 21
                ));
                params.push(&data.architecture);
                params.push(&data.features);
                params.push(&data.file_type);
                params.push(&data.filename);
                params.push(&data.image_type);
                params.push(&data.java_version);
                params.push(&data.jvm_impl);
                params.push(&data.md5);
                params.push(&data.md5_url);
                params.push(&data.os);
                params.push(&data.release_type);
                params.push(&data.sha1);
                params.push(&data.sha1_url);
                params.push(&data.sha256);
                params.push(&data.sha256_url);
                params.push(&data.sha512);
                params.push(&data.sha512_url);
                params.push(&data.size);
                params.push(&data.url);
                params.push(&data.vendor);
                params.push(&data.version);
            }

            query.push_str(
                " ON CONFLICT(url) DO UPDATE SET
                architecture = excluded.architecture,
                features = excluded.features,
                file_type = excluded.file_type,
                filename = excluded.filename,
                image_type = excluded.image_type,
                java_version = excluded.java_version,
                jvm_impl = excluded.jvm_impl,
                md5 = excluded.md5,
                md5_url = excluded.md5_url,
                modified_at = CURRENT_TIMESTAMP,
                os = excluded.os,
                release_type = excluded.release_type,
                sha1 = excluded.sha1,
                sha1_url = excluded.sha1_url,
                sha256 = excluded.sha256,
                sha256_url = excluded.sha256_url,
                sha512 = excluded.sha512,
                sha512_url = excluded.sha512_url,
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
                OR excluded.version != JAVA_META_DATA.version
                ;",
            );

            result += tx.execute(&query, &params)?;
        }

        tx.commit()?;
        Ok(result)
    }

    pub fn export(&self, release_type: &str, arch: &str, os: &str) -> Result<Vec<JavaMetaData>> {
        let mut conn = self.pool.get()?;
        let stmt = conn.prepare(
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
                AND release_type = $1
                AND os = $2
                AND architecture = $3
            ;",
        )?;

        let mut data = Vec::new();
        let rows = conn.query(&stmt, &[&release_type, &os, &arch])?;
        for row in rows {
            data.push(JavaMetaData {
                architecture: row.get("architecture"),
                features: row
                    .get::<_, Option<String>>("features")
                    .map(|f| f.split(',').map(String::from).collect()),
                file_type: row.get("file_type"),
                filename: row.get("filename"),
                image_type: row.get("image_type"),
                java_version: row.get("java_version"),
                jvm_impl: row.get("jvm_impl"),
                md5: row.get("md5"),
                md5_url: row.get("md5_url"),
                os: row.get("os"),
                release_type: row.get("release_type"),
                sha1: row.get("sha1"),
                sha1_url: row.get("sha1_url"),
                sha256: row.get("sha256"),
                sha256_url: row.get("sha256_url"),
                sha512: row.get("sha512"),
                sha512_url: row.get("sha512_url"),
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
        let stmt = conn.prepare(&format!(
            "SELECT DISTINCT {} FROM JAVA_META_DATA ORDER BY {} ASC;",
            column, column
        ))?;
        let mut data = Vec::new();
        let rows = conn.query(&stmt, &[])?;
        for row in rows {
            data.push(row.get::<usize, String>(0));
        }
        Ok(data)
    }
}

#[derive(Clone, Default, Debug)]
struct DbJavaMetaData {
    pub architecture: String,
    pub features: Option<String>,
    pub file_type: String,
    pub filename: String,
    pub image_type: String,
    pub java_version: String,
    pub jvm_impl: String,
    pub md5: Option<String>,
    pub md5_url: Option<String>,
    pub os: String,
    pub release_type: String,
    pub sha1: Option<String>,
    pub sha1_url: Option<String>,
    pub sha256: Option<String>,
    pub sha256_url: Option<String>,
    pub sha512: Option<String>,
    pub sha512_url: Option<String>,
    pub size: Option<i32>,
    pub url: String,
    pub vendor: String,
    pub version: String,
}

fn map_workaround(meta_data: &HashSet<JavaMetaData>) -> Vec<DbJavaMetaData> {
    meta_data
        .iter()
        // workaround for the `feature` field which needs to be joined
        // and therefore would not live long enough in context of a
        // batch insert
        .map(|item| DbJavaMetaData {
            architecture: item.architecture.clone(),
            features: item.features.as_ref().map(|f| f.join(",")),
            file_type: item.file_type.clone(),
            filename: item.filename.clone(),
            image_type: item.image_type.clone(),
            java_version: item.java_version.clone(),
            jvm_impl: item.jvm_impl.clone(),
            md5: item.md5.clone(),
            md5_url: item.md5_url.clone(),
            os: item.os.clone(),
            release_type: item.release_type.clone(),
            sha1: item.sha1.clone(),
            sha1_url: item.sha1_url.clone(),
            sha256: item.sha256.clone(),
            sha256_url: item.sha256_url.clone(),
            sha512: item.sha512.clone(),
            sha512_url: item.sha512_url.clone(),
            size: item.size,
            url: item.url.clone(),
            vendor: item.vendor.clone(),
            version: item.version.clone(),
        })
        .collect::<Vec<DbJavaMetaData>>()
}
