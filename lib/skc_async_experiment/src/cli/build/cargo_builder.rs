use crate::cli::Cli;
use crate::package::PackageSpec;
use anyhow::{bail, Result};
use std::path::PathBuf;
use std::process::Command;

pub fn run(cli: &mut Cli, spec_dir: &PathBuf, spec: &PackageSpec) -> Result<()> {
    let Some(rust_libs) = &spec.rust_libs else {
        return Ok(());
    };
    for rust_lib in rust_libs {
        let manifest_path = spec_dir.join(rust_lib).join("Cargo.toml");
        let target_dir = cli
            .shiika_work
            .join(format!("{}-{}", &spec.name, &spec.version));
        let mut cmd = Command::new("cargo");
        cmd.arg("build");
        cmd.arg("--manifest-path").arg(manifest_path);
        cmd.arg("--target-dir").arg(target_dir);
        dbg!(&cmd);
        if !cmd.status()?.success() {
            bail!("cargo failed ({:?})", cmd);
        }
    }
    Ok(())
}
