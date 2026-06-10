use std::path::PathBuf;

use eyre::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::config::Conf;

pub struct ConnectionPool {}

impl ConnectionPool {
    pub fn get_pool() -> Result<Pool<SqliteConnectionManager>> {
        let conf: Conf = Conf::try_get()?;
        let path = PathBuf::from(conf.database.path.unwrap_or("roast.sqlite3".to_string()));
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }

        let manager = SqliteConnectionManager::file(path).with_init(|conn| {
            conn.pragma_update(None, "journal_mode", "WAL")?;
            conn.pragma_update(None, "foreign_keys", "ON")?;
            Ok(())
        });

        let pool = Pool::builder()
            .max_size(conf.database.pool_size.unwrap_or(10))
            .build(manager)?;
        pool.get()?.execute_batch(include_str!("../../sql/schema.sql"))?;
        Ok(pool)
    }
}
