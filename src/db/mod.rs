use std::sync::Arc;

use eyre::Result;
use postgres::Postgres;
use sqlite::Sqlite;

use crate::{config::Conf, meta::JavaMetaData};

mod postgres;
mod sqlite;

pub trait Operations: Send + Sync {
    fn insert(&self, meta_data: &[JavaMetaData]) -> Result<u64>;
    fn export(&self, release_type: &str, arch: &str, os: &str) -> Result<Vec<JavaMetaData>>;
    fn get_distinct(&self, column: &str) -> Result<Vec<String>>;
}

pub struct Database {}

impl Database {
    pub fn get() -> Result<Arc<dyn Operations>> {
        let conf = Conf::try_get()?;
        if conf.database.url.is_none() {
            return Err(eyre::eyre!("database.url is not configured"));
        }
        match conf.database.url.as_deref() {
            Some(url) => {
                if url.starts_with("sqlite://") {
                    Ok(Arc::new(Sqlite::new()))
                } else if url.starts_with("postgres://") {
                    Ok(Arc::new(Postgres::new()))
                } else {
                    Err(eyre::eyre!("unsupported database URL: {}", url))
                }
            }
            None => Err(eyre::eyre!("database.url is not configured")),
        }
    }
}
