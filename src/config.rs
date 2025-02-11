#![allow(unused)]

use confique::{Config, Error};
use shellexpand::tilde;

#[derive(Config, Debug)]
pub struct FetchConf {
    // Define fields for fetch configuration
    // ...
}

#[derive(Config, Debug)]
pub struct JsonConf {
    /// Path to the JSON directory
    #[config(env = "JMETA_JSON_PATH")]
    pub path: Option<String>,
}

#[derive(Config, Debug)]
pub struct SqliteConf {
    /// Path to the SQLite database
    #[config(env = "JMETA_SQLITE_PATH")]
    pub path: Option<String>,
}

#[derive(Config, Debug)]
pub struct Conf {
    #[config(nested)]
    pub fetch: FetchConf,
    #[config(nested)]
    pub json: JsonConf,
    #[config(nested)]
    pub sqlite: SqliteConf,
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
