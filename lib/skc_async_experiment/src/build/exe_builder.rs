use crate::cli::Cli;
//use crate::package::PackageSpec;
use crate::{build, codegen, mir, prelude};
use anyhow::{Context, Result};
use shiika_parser::SourceFile;
use std::path::PathBuf;

/// Builds a single .sk file and generates an executable.
/// Returns the path to the generated executable.
pub fn run(cli: &mut Cli, entry_point: &PathBuf) -> Result<PathBuf> {
    let txt = std::fs::read_to_string(entry_point)
        .context(format!("failed to read {}", &entry_point.to_string_lossy()))?;
    let src = SourceFile::new(entry_point.clone(), txt);
    let mut mir = build::compiler::run(cli, src)?;

    for (name, fun_ty) in prelude::core_externs() {
        mir.program.externs.push(mir::Extern { name, fun_ty });
    }
    mir.program.funcs.append(&mut prelude::funcs());

    cli.log(&format!("# -- verifier input --\n{}\n", mir.program));
    mir::verifier::run(&mir.program)?;

    let bc_path = entry_point.with_extension("bc");
    let ll_path = entry_point.with_extension("ll");
    codegen::run(&bc_path, Some(&ll_path), mir)?;
    build::linker::run(bc_path, &vec![cli.built_core()?])
}
