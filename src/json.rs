use eyre::Result;
use log::info;
use std::fs::File;

use crate::{config::Conf, meta::JavaMetaData};

pub struct Json {}

impl Json {
    pub fn save(vendor: &str, meta_data: &Vec<JavaMetaData>) -> Result<()> {
        let conf = Conf::try_get()?;
        if conf.json.path.is_none() {
            return Ok(());
        }

        let path = format!(
            "{path}{name}.json",
            path = conf.json.path.unwrap(),
            name = vendor
        );

        info!("[{}] writing to JSON [path={}]", vendor, path);

        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, meta_data)?;
        Ok(())
    }
}
