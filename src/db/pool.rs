use eyre::Result;
use openssl::ssl::{SslConnector, SslMethod};
use postgres_openssl::MakeTlsConnector;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;

use crate::config::Conf;

pub fn get_pool() -> Result<Pool<PostgresConnectionManager<MakeTlsConnector>>> {
    let conf: Conf = Conf::try_get()?;

    match conf.database.url {
        Some(url) => {
            if url.starts_with("postgres://") {
                let connector =
                    MakeTlsConnector::new(SslConnector::builder(SslMethod::tls())?.build());
                let manager = PostgresConnectionManager::new(url.parse().unwrap(), connector);
                let pool = Pool::builder()
                    .max_size(conf.database.pool_size.unwrap_or(10))
                    .build(manager)?;
                Ok(pool)
            } else {
                return Err(eyre::eyre!("unsupported database URL: {}", url));
            }
        }
        None => return Err(eyre::eyre!("database.url is not configured")),
    }
}
