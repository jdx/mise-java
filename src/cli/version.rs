use std::sync::LazyLock;

use eyre::Result;
use versions::Versioning;

use crate::{build_time::BUILD_TIME, env};

/// Show version information
#[derive(Debug, clap::Args)]
#[clap(alias = "v")]
pub struct Version {}

pub static OS: LazyLock<String> = LazyLock::new(|| std::env::consts::OS.into());
pub static ARCH: LazyLock<String> = LazyLock::new(|| {
    match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        _ => std::env::consts::ARCH,
    }
    .to_string()
});

pub static VERSION: LazyLock<String> = LazyLock::new(|| {
    let mut v = V.to_string();
    if cfg!(debug_assertions) {
        v.push_str("-DEBUG");
    };
    let build_time = BUILD_TIME.format("%Y-%m-%d");
    format!("{v} {os}-{arch} ({build_time})", os = *OS, arch = *ARCH)
});

pub static V: LazyLock<Versioning> = LazyLock::new(|| Versioning::new(env!("CARGO_PKG_VERSION")).unwrap());

impl Version {
    pub fn run(self) -> Result<()> {
        show_version()?;
        Ok(())
    }
}

pub fn print_version_if_requested(args: &[String]) -> std::io::Result<()> {
    if args.len() == 2 && *env::BINARY_NAME == "roast" {
        let cmd = &args[1].to_lowercase();
        if cmd == "version" || cmd == "-v" || cmd == "--version" {
            show_version()?;
            std::process::exit(0);
        }
    }
    Ok(())
}

fn show_version() -> std::io::Result<()> {
    println!("{}", *VERSION);
    Ok(())
}
