use std::collections::HashMap;

use clap::Subcommand;

mod release_type;
mod vendor;

#[derive(Debug, Subcommand)]
enum Commands {
    ReleaseType(release_type::ReleaseType),
    Vendor(vendor::Vendor),
}

impl Commands {
    pub fn run(self) -> eyre::Result<()> {
        match self {
            Self::ReleaseType(cmd) => cmd.run(),
            Self::Vendor(cmd) => cmd.run(),
        }
    }
}

/// Export JVM data
#[derive(Debug, clap::Args)]
pub struct Export {
    #[clap(subcommand)]
    command: Commands,
}

impl Export {
    pub fn run(self) -> eyre::Result<()> {
        self.command.run()
    }
}

fn get_filter_map(filters: Vec<String>) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for filter in filters {
        let parts: Vec<&str> = filter.split('=').collect();
        if parts.len() != 2 {
            continue;
        }
        let key = parts[0].to_string();
        let value = parts[1].split(",").map(|s| s.to_string()).collect::<Vec<_>>();
        map.entry(key).or_default().extend(value);
    }
    map
}
