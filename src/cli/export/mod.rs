use clap::Subcommand;

mod triple;

#[derive(Debug, Subcommand)]
enum Commands {
    Triple(triple::Triple),
}

impl Commands {
    pub fn run(self) -> eyre::Result<()> {
        match self {
            Self::Triple(cmd) => cmd.run(),
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
