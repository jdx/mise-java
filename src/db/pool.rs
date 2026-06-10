use std::path::PathBuf;

use eyre::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::config::Conf;

pub struct ConnectionPool {}

impl ConnectionPool {
    /// Create and initialize an r2d2 SQLite connection pool using the application's configuration.
    ///
    /// Ensures the database file's parent directory exists (if applicable), configures each
    /// connection with WAL journal mode and foreign key enforcement, applies the bundled
    /// SQL schema (sql/schema.sql), and returns the ready-to-use pool. Returns an error if
    /// configuration, filesystem operations, pool creation, or schema initialization fail.
    ///
    /// # Examples
    ///
    /// ```
    /// let pool = crate::db::pool::ConnectionPool::get_pool().expect("failed to create pool");
    /// // obtain a connection to validate the pool works
    /// let conn = pool.get().expect("failed to get connection");
    /// conn.execute_batch("SELECT 1;").expect("query failed");
    /// ```
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
