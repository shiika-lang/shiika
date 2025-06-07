use crate::build;
use crate::cli::Cli;
use crate::package::Package;
use anyhow::Result;

pub fn run(cli: &mut Cli, p: &Package) -> Result<()> {
    build::cargo_builder::run(cli, &p)?;
    build::lib_builder::build(cli, &p)?;
    Ok(())
}
