use crate::cli::Cli;
use crate::package::Package;
use anyhow::{bail, Result};
use std::process::Command;

pub fn run(cli: &mut Cli, p: &Package) -> Result<()> {
    let Some(rust_libs) = &p.spec.rust_libs else {
        return Ok(());
    };
    for rust_lib in rust_libs {
        let manifest_path = p
            .spec_path
            .parent()
            .unwrap()
            .join(rust_lib)
            .join("Cargo.toml");
        let target_dir = cli.cargo_target_dir(&p.spec);
        let mut cmd = Command::new("cargo");
        cmd.arg("build");
        cmd.arg("--manifest-path").arg(manifest_path);
        cmd.arg("--target-dir").arg(target_dir);
        if !cmd.status()?.success() {
            bail!("cargo failed ({:?})", cmd);
        }
    }
    Ok(())
}
