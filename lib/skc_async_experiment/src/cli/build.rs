use crate::build::cargo_builder;
use crate::cli::Cli;
use crate::package;
use anyhow::Result;
use std::path::PathBuf;

pub fn run(cli: &mut Cli, path: &PathBuf) -> Result<()> {
    let (dir, spec) = package::load_spec(path)?;
    cargo_builder::run(cli, &dir, &spec)?;
    Ok(())
}
