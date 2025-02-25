#![allow(unused)]

use confique::{Config, Error};
use shellexpand::tilde;

#[derive(Config, Debug)]
pub struct ExportConf {
    /// Path to the export directory
    #[config(env = "JMETA_EXPORT_PATH")]
    pub path: Option<String>,
}

#[derive(Config, Debug)]
pub struct DatabaseConf {
    /// Database connection pool size
    /// Default: 10
    #[config(env = "JMETA_DATABASE_POOL_SIZE")]
    pub pool_size: Option<u32>,
    /// Database connection URL
    #[config(env = "JMETA_DATABASE_URL")]
    pub url: Option<String>,
}

#[derive(Config, Debug)]
pub struct Conf {
    #[config(nested)]
    pub export: ExportConf,
    #[config(nested)]
    pub database: DatabaseConf,
}

impl Conf {
    pub fn try_get() -> Result<Self, Error> {
        let conf = Config::builder()
            .env()
            .file("config.toml")
            .file(tilde("~/.config/jmeta/config.toml").into_owned())
            .load()?;
        Ok(conf)
    }
}
