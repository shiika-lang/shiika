//! Compiles the Shiika code in a package into single .bc.
use crate::build;
use crate::cli::Cli;
use crate::package::Package;
use anyhow::Result;

pub fn build(cli: &mut Cli, package: &Package) -> Result<()> {
    let deps = vec![]; // TODO: get deps from package
    let out_dir = cli.lib_target_dir(&package.spec);
    build::compiler::compile(cli, &package.entry_point(), &out_dir, &deps, false)?;
    Ok(())
}
