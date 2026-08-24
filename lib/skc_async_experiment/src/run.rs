use crate::cli;
use anyhow::Result;
use clap::Parser;
use std::path::Path;

/// Compile a single `.sk` file. Used by the `erroneous` integration test.
pub fn compile(path: &Path) -> Result<()> {
    let mut cli = cli::Cli::init()?;
    cli.compile(&path.to_path_buf())
}

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
