use clap::{FromArgMatches, Subcommand};
use color_eyre::Result;
use indoc::indoc;

mod export;
mod fetch;
pub mod version;

pub struct Cli {}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Fetch(fetch::Fetch),
    Export(export::Export),
    Version(version::Version),
}

impl Commands {
    pub fn run(self) -> Result<()> {
        match self {
            #[cfg(debug_assertions)]
            Self::Fetch(cmd) => cmd.run(),
            Self::Export(cmd) => cmd.run(),
            Self::Version(cmd) => cmd.run(),
        }
    }
}

impl Cli {
    pub fn command() -> clap::Command {
        Commands::augment_subcommands(
            clap::Command::new("jmdb")
                .version(version::VERSION.to_string())
                .about(env!("CARGO_PKG_DESCRIPTION"))
                .author("Roland Schär <@roele>")
                .long_about(LONG_ABOUT)
                .arg_required_else_help(true)
                .subcommand_required(true),
        )
    }

    pub fn run(args: &Vec<String>) -> Result<()> {
        crate::env::ARGS.write().unwrap().clone_from(args);
        version::print_version_if_requested(args)?;

        let matches = Self::command()
            .try_get_matches_from(args)
            .unwrap_or_else(|_| Self::command().get_matches_from(args));

        // debug!("ARGS: {}", &args.join(" "));

        match Commands::from_arg_matches(&matches) {
            Ok(cmd) => cmd.run(),
            Err(err) => matches.subcommand().ok_or(err).map(|_| {
                // No subcommand was provided, so we'll just print the help message
                Self::command().print_help().unwrap();
                Ok(())
            })?,
        }
    }
}

const LONG_ABOUT: &str = indoc! {"
jmdb is a tool for managing Java metadata. https://github.com/roele/jmdb

A database which contains metadata about the various Java distributions.
"};
