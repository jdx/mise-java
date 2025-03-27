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
