use crate::cli;
use anyhow::Result;
use clap::Parser;

pub fn main() -> Result<()> {
    env_logger::init();
    let mut cli = cli::Cli::init()?;
    let options = cli::CommandLineOptions::try_parse()?;
    match &options.command {
        Some(cli::Command::Build { path }) => {
            cli.build(path)?;
        }
        Some(cli::Command::Compile { path }) => {
            cli.compile(path)?;
        }
        Some(cli::Command::Run { path }) => {
            cli.run(path)?;
        }
        None => {}
    }
    Ok(())
}
