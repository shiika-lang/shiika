use crate::build;
use crate::cli::Cli;
use crate::package::Package;
use anyhow::Result;
use std::path::PathBuf;

/// Builds a single .sk file and generates an executable.
/// Returns the path to the generated executable.
pub fn run(cli: &mut Cli, entry_point: &PathBuf) -> Result<PathBuf> {
    let deps = vec![Package::load_core(cli)?];
    let out_dir = entry_point.parent().unwrap();
    let bc_path = build::compiler::compile(cli, entry_point, out_dir, &deps, true)?;
    let artifacts = deps
        .iter()
        .flat_map(|pkg| pkg.artifacts.clone())
        .collect::<Vec<_>>();
    build::linker::run(bc_path, &artifacts)
}
