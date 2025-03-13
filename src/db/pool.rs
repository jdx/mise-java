use std::time::Duration;

use eyre::Result;
use openssl::ssl::{SslConnector, SslMethod};
use postgres_openssl::MakeTlsConnector;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;

use crate::config::Conf;

pub struct ConnectionPool {}

impl ConnectionPool {
    pub fn get_pool() -> Result<Pool<PostgresConnectionManager<MakeTlsConnector>>> {
        let conf: Conf = Conf::try_get()?;

        match conf.database.url {
            Some(url) => {
                if url.starts_with("postgres://") {
                    let mut connector = SslConnector::builder(SslMethod::tls())?;
                    match conf
                        .database
                        .ssl_mode
                        .unwrap_or("prefer".to_string())
                        .to_lowercase()
                        .as_ref()
                    {
                        "require" | "prefer" | "allow" => connector.set_verify(openssl::ssl::SslVerifyMode::NONE),
                        "verify-ca" => {
                            connector.set_verify(openssl::ssl::SslVerifyMode::PEER);
                            // disable hostname verification
                            connector.set_verify_callback(openssl::ssl::SslVerifyMode::PEER, |_, _| true);
                            connector.set_ca_file(conf.database.ssl_ca.expect("database.ssl_ca is not configured"))?;
                        }
                        // default to the verify-full
                        _ => {
                            connector.set_verify(
                                openssl::ssl::SslVerifyMode::PEER | openssl::ssl::SslVerifyMode::FAIL_IF_NO_PEER_CERT,
                            );
                            // disable hostname verification
                            connector.set_verify_callback(openssl::ssl::SslVerifyMode::PEER, |_, _| true);
                            connector.set_ca_file(conf.database.ssl_ca.expect("database.ssl_ca is not configured"))?;
                            connector.set_certificate_chain_file(
                                conf.database.ssl_cert.expect("database.ssl_cert is not configured"),
                            )?;
                            connector.set_private_key_file(
                                conf.database.ssl_key.expect("database.ssl_key is not configured"),
                                openssl::ssl::SslFiletype::PEM,
                            )?;
                        }
                    }
                    let tls_connector = MakeTlsConnector::new(connector.build());
                    let manager = PostgresConnectionManager::new(url.parse().unwrap(), tls_connector);
                    let pool = Pool::builder()
                        .max_size(conf.database.pool_size.unwrap_or(10))
                        .max_lifetime(Some(Duration::from_secs(60 * 60)))
                        .build(manager)?;
                    Ok(pool)
                } else {
                    Err(eyre::eyre!("unsupported database URL: {}", url))
                }
            }
            None => Err(eyre::eyre!("database.url is not configured")),
        }
    }
}
