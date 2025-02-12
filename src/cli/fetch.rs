use crossbeam_channel::{select, unbounded};
use eyre::Result;
use log::{error, info};
use std::{collections::HashMap, sync::Arc};

use crate::{
    json::Json,
    meta::vendor::{Vendor, VENDORS},
    sqlite::Sqlite,
};

#[derive(Debug, clap::Args)]
#[clap(verbatim_doc_comment, after_long_help = AFTER_LONG_HELP)]
pub struct Fetch {
    /// Vendor(s) to fetch
    /// e.g.: openjdk, zulu
    #[clap(value_name = "VENDOR")]
    pub vendors: Vec<String>,
}

impl Fetch {
    pub fn run(self) -> Result<()> {
        if self.vendors.is_empty() {
            info!("fetching all vendors");
        } else {
            info!("fetching vendors: {:?}", self.vendors);
        }

        let start = std::time::Instant::now();

        let pool = rayon::ThreadPoolBuilder::default().build()?;
        pool.scope(|s| {
            let run = |name: String, vendor: Arc<dyn Vendor>| {
                s.spawn(move |_| {
                    info!("[{}] fetching meta data", name);
                    let meta_data = match vendor.fetch() {
                        Ok(data) => data,
                        Err(err) => {
                            error!("[{}] failed to fetch meta data: {}", name, err);
                            return;
                        }
                    };

                    match Json::save(&name, &meta_data) {
                        Ok(_) => {}
                        Err(err) => {
                            error!("[{}] failed to write to JSON: {}", name, err);
                        }
                    }

                    match Sqlite::insert(&name, &meta_data) {
                        Ok(_) => {}
                        Err(err) => {
                            error!("[{}] failed to write to SQLite: {}", name, err);
                        }
                    };
                });
            };

            let (tx, rx) = unbounded();
            for (name, vendor) in self.get_vendors() {
                tx.send((name, vendor)).unwrap();
            }
            drop(tx);

            loop {
                select! {
                    recv(rx) -> msg => {
                        match msg {
                            Ok((name, vendor)) => run(name, vendor),
                            Err(_) => break,
                        }
                    }
                }
            }
        });

        info!(
            "fetched all vendors in {:.2} seconds",
            start.elapsed().as_secs_f32()
        );
        Ok(())
    }

    fn get_vendors(&self) -> HashMap<String, Arc<dyn Vendor>> {
        VENDORS
            .iter()
            .map(|v| (v.get_name(), v.to_owned()))
            .filter(|(k, _v)| self.vendors.is_empty() || self.vendors.contains(k))
            .collect()
    }
}

static AFTER_LONG_HELP: &str = color_print::cstr!(
    r#"<bold><underline>Examples:</underline></bold>

$ <bold>jmeta fetch</bold>
"#
);
