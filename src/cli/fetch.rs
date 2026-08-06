use crossbeam_channel::{select, unbounded};
use eyre::{Result, bail};
use log::{error, info};
use std::{collections::HashMap, sync::Arc};

use crate::{
    db::{jvm_repository::JvmRepository, pool::ConnectionPool},
    jvm::vendor::{VENDORS, Vendor},
};

/// Fetch data from JVM vendors
///
/// Will crawl data from all vendors if none are specified
#[derive(Debug, clap::Args)]
#[clap(verbatim_doc_comment)]
pub struct Fetch {
    /// Vendors to fetch e.g.: openjdk, zulu
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
        let conn_pool = ConnectionPool::get_pool()?;
        let pool = rayon::ThreadPoolBuilder::default().build()?;
        let (failure_tx, failure_rx) = unbounded();
        pool.scope(|s| {
            let run = |name: String, vendor: Arc<dyn Vendor>| {
                let conn_pool = conn_pool.clone();
                let failure_tx = failure_tx.clone();
                s.spawn(move |_| {
                    let db = match JvmRepository::new(conn_pool) {
                        Ok(db) => db,
                        Err(err) => {
                            error!("[{name}] failed to connect to database: {err}");
                            failure_tx.send(name).unwrap();
                            return;
                        }
                    };

                    info!("[{name}] fetching meta data");
                    let jvm_data = match vendor.fetch() {
                        Ok(data) => data,
                        Err(err) => {
                            error!("[{name}] failed to fetch meta data: {err}");
                            failure_tx.send(name).unwrap();
                            return;
                        }
                    };

                    info!("[{name}] writing to database");
                    match db.insert(&jvm_data) {
                        Ok(result) => {
                            info!("[{name}] inserted/modified {result} records")
                        }
                        Err(err) => {
                            error!("[{name}] failed to write to database: {err}");
                            failure_tx.send(name).unwrap();
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
        drop(failure_tx);

        fail_if_any_vendor_updates_failed(failure_rx.into_iter().collect())?;

        info!("fetched all vendors in {:.2} seconds", start.elapsed().as_secs_f32());
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

fn fail_if_any_vendor_updates_failed(mut failures: Vec<String>) -> Result<()> {
    if !failures.is_empty() {
        failures.sort();
        bail!("failed to update vendors: {}", failures.join(", "));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::fail_if_any_vendor_updates_failed;

    #[test]
    fn fails_when_vendor_updates_fail() {
        let err = fail_if_any_vendor_updates_failed(vec!["zulu".into(), "temurin".into()]).unwrap_err();

        assert_eq!(err.to_string(), "failed to update vendors: temurin, zulu");
    }

    #[test]
    fn succeeds_when_vendor_updates_succeed() {
        assert!(fail_if_any_vendor_updates_failed(vec![]).is_ok());
    }
}
