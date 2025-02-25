use crate::meta::JavaMetaData;
use eyre::Result;
use postgres_openssl::MakeTlsConnector;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;

pub struct Postgres {
    pool: Pool<PostgresConnectionManager<MakeTlsConnector>>,
}

impl Postgres {
    pub fn new(pool: Pool<PostgresConnectionManager<MakeTlsConnector>>) -> Result<Self> {
        Ok(Postgres { pool })
    }

    pub fn insert(&self, meta_data: &[JavaMetaData]) -> Result<u64> {
        let mut conn = self.pool.get()?;
        let mut result = 0;
        let mut tx = conn.transaction()?;
        {
            let stmt = tx.prepare(
        "INSERT INTO JAVA_META_DATA
              (architecture, features, file_type, filename, image_type, java_version, jvm_impl, md5, md5_url, os, release_type, sha1, sha1_url, sha256, sha256_url, sha512, sha512_url, size, url, vendor, version)
            VALUES
              ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
            ON CONFLICT(url) DO UPDATE SET
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
                result += tx.execute(
                    &stmt,
                    &[
                        &data.architecture,
                        &features,
                        &data.file_type,
                        &data.filename,
                        &data.image_type,
                        &data.java_version,
                        &data.jvm_impl,
                        &data.md5,
                        &data.md5_url,
                        &data.os,
                        &data.release_type,
                        &data.sha1,
                        &data.sha1_url,
                        &data.sha256,
                        &data.sha256_url,
                        &data.sha512,
                        &data.sha512_url,
                        &data.size.map(|s| s as i32),
                        &data.url,
                        &data.vendor,
                        &data.version,
                    ],
                )?;
            }
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
                architecture: row.get(0),
                features: row
                    .get::<_, Option<String>>(1)
                    .map(|f| f.split(',').map(String::from).collect()),
                file_type: row.get(2),
                filename: row.get(3),
                image_type: row.get(4),
                java_version: row.get(5),
                jvm_impl: row.get(6),
                md5: row.get(7),
                md5_url: row.get(8),
                os: row.get(9),
                release_type: row.get(10),
                sha1: row.get(11),
                sha1_url: row.get(12),
                sha256: row.get(13),
                sha256_url: row.get(14),
                sha512: row.get(15),
                sha512_url: row.get(16),
                size: row.get::<_, Option<i32>>(17).map(|s| s as u64),
                url: row.get(18),
                vendor: row.get(19),
                version: row.get(20),
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
