use crate::cli;
use anyhow::Result;
use clap::Parser;

pub fn main() -> Result<()> {
    env_logger::init();
    let mut cli = cli::Cli::new();
    let options = cli::CommandLineOptions::try_parse()?;
    match &options.command {
        Some(cli::Command::Build { path: _path }) => {
            todo!("build");
        }
        Some(cli::Command::Compile { path: _path }) => {
            todo!("compile");
        }
        Some(cli::Command::Run { path }) => {
            cli.run(path)?;
        }
        None => {}
    }
    Ok(())
}
