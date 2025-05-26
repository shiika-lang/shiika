use crate::build;
use crate::cli::Cli;
use crate::package::Package;
use anyhow::Result;
use std::path::PathBuf;

/// Builds a single .sk file and generates an executable.
/// Returns the path to the generated executable.
pub fn run(cli: &mut Cli, entry_point: &PathBuf) -> Result<PathBuf> {
    let deps = vec![Package::load_core(cli)?]; //TODO: load dependencies
    let total_deps = deps.iter().map(|x| x.spec.name.clone()).collect();
    let out_dir = entry_point.parent().unwrap();
    let target = build::CompileTarget {
        entry_point,
        out_dir: &out_dir,
        deps: &deps,
        detail: build::CompileTargetDetail::Bin {
            package: None,
            total_deps,
        },
    };
    let (bc_path, _) = build::compiler::compile(cli, &target)?;
    let artifacts = deps
        .iter()
        .flat_map(|pkg| pkg.artifacts.clone())
        .collect::<Vec<_>>();
    build::linker::run(bc_path, &artifacts)
}
