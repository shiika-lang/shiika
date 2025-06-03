//! Compiles the Shiika code in a package into single .bc.
use crate::build;
use crate::cli::Cli;
use crate::hir;
use crate::mir;
use crate::package::{self, Package};
use anyhow::Result;
use shiika_core::names::type_fullname;
use skc_hir::{MethodSignature, SkTypes};
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

    write_exports_json(
        &cli.lib_exports_path(&package.spec),
        &create_exports(&mir, package)?,
    )?;
    Ok(())
}

fn create_exports(mir: &mir::CompilationUnit, package: &Package) -> Result<LibraryExports> {
    let mut sk_types = mir.sk_types.clone();
    // Write back asyncness to SkType (REFACTOR: do this during mir generation?)
    for func in &mir.program.funcs {
        if let Some(sig) = &func.sig {
            let asyncness = match func.asyncness {
                mir::Asyncness::Sync => skc_hir::Asyncness::Sync,
                mir::Asyncness::Async => skc_hir::Asyncness::Async,
                mir::Asyncness::Unknown => unreachable!(),
                mir::Asyncness::Lowered => unreachable!(),
            };
            let sk_type = sk_types
                .0
                .get_mut(&sig.fullname.type_name)
                .expect("Function type not found in sk_types");
            let (sig2, _) = sk_type
                .base_mut()
                .method_sigs
                .get_mut(&sig.fullname.first_name)
                .expect("Function signature not found in sk_types");
            sig2.asyncness = asyncness;
        }
    }
    // Convert constants to HashMap
    let mut constants = HashMap::new();
    for (name, ty) in &mir.program.constants {
        constants.insert(name.clone(), ty.clone());
    }
    // Add type obj constants
    for sk_type in mir.sk_types.0.values() {
        constants.insert(
            sk_type.fullname().to_const_fullname(),
            sk_type.term_ty().meta_ty(),
        );
    }
    // Merge rustlib methods
    merge_rustlib_methods(&mut sk_types, package)?;
    Ok(LibraryExports {
        sk_types,
        vtables: mir.vtables.clone(),
        constants,
    })
}

fn merge_rustlib_methods(sk_types: &mut SkTypes, p: &Package) -> Result<()> {
    for exp in p.export_files() {
        for (type_name, sig_str, is_async) in package::load_exports_json5(&exp)? {
            let mut sig = parse_sig(type_name.clone(), sig_str)?;
            sig.asyncness = if is_async {
                skc_hir::Asyncness::Async
            } else {
                skc_hir::Asyncness::Sync
            };
            sk_types.define_method(&type_fullname(type_name), sig);
        }
    }
    Ok(())
}

fn parse_sig(type_name: String, sig_str: String) -> Result<MethodSignature> {
    let ast_sig = shiika_parser::Parser::parse_signature(&sig_str)?;
    Ok(hir::untyped::compile_signature(type_name, &ast_sig))
}

/// Serialize LibraryExports into exports.json
fn write_exports_json(out_path: &std::path::Path, exports: &skc_mir::LibraryExports) -> Result<()> {
    let json = serde_json::to_string_pretty(exports)?;
    let mut f = std::fs::File::create(out_path)?;
    f.write_all(json.as_bytes())?;
    Ok(())
}
