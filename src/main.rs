use cli::Cli;
use color_eyre::{Section, SectionExt};
use itertools::Itertools;

use crate::cli::version::VERSION;

pub mod build_time;
mod cli;
mod config;
mod db;
mod env;
mod github;
mod http;
mod meta;

#[macro_use]
mod output;

fn main() -> eyre::Result<()> {
    env_logger::builder()
        .format_target(false)
        .format_timestamp_millis()
        .init();

    let args = std::env::args().collect_vec();
    match Cli::run(&args).with_section(|| VERSION.to_string().header("Version:")) {
        Ok(()) => Ok(()),
        Err(err) => handle_err(err),
    }
}

fn handle_err(err: eyre::Report) -> eyre::Result<()> {
    if let Some(err) = err.downcast_ref::<std::io::Error>() {
        if err.kind() == std::io::ErrorKind::BrokenPipe {
            return Ok(());
        }
    }
    Err(err)
}
