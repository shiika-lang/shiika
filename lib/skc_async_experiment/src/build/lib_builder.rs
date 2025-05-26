//! Compiles the Shiika code in a package into single .bc.
use crate::build;
use crate::cli::Cli;
use crate::mir;
use crate::package::Package;
use anyhow::Result;
use skc_mir::LibraryExports;
use std::collections::HashMap;
use std::io::Write;

pub fn build(cli: &mut Cli, package: &Package) -> Result<()> {
    let deps = vec![]; // TODO: get deps from package
    let target = build::CompileTarget {
        entry_point: &package.entry_point(),
        out_dir: &cli.lib_target_dir(&package.spec),
        deps: &deps,
        detail: build::CompileTargetDetail::Lib { package },
    };
    let (_, mir) = build::compiler::compile(cli, &target)?;

    write_exports_json(&cli.lib_exports_path(&package.spec), &create_exports(&mir))?;
    Ok(())
}

fn create_exports(mir: &mir::CompilationUnit) -> LibraryExports {
    let mut constants = HashMap::new();
    for (name, ty) in &mir.program.constants {
        constants.insert(name.clone(), ty.clone());
    }
    for sk_type in mir.sk_types.0.values() {
        constants.insert(
            sk_type.fullname().to_const_fullname(),
            sk_type.term_ty().meta_ty(),
        );
    }
    LibraryExports {
        sk_types: mir.sk_types.clone(),
        vtables: mir.vtables.clone(),
        constants,
    }
}

/// Serialize LibraryExports into exports.json
fn write_exports_json(out_path: &std::path::Path, exports: &skc_mir::LibraryExports) -> Result<()> {
    let json = serde_json::to_string_pretty(exports)?;
    let mut f = std::fs::File::create(out_path)?;
    f.write_all(json.as_bytes())?;
    Ok(())
}
