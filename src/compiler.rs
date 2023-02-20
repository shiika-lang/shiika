use crate::config::from_shiika_root;
use crate::loader;
use crate::package::SkPackage;
use crate::targets;
use anyhow::{Context, Error, Result};
use shiika_parser::{Parser, SourceFile};
use skc_ast2hir;
use skc_codegen;
use skc_corelib;
use skc_mir::LibraryExports;
use std::fs;
use std::io::Read;
use std::path::Path;

/// Generate .ll from .sk
pub fn compile<P: AsRef<Path>>(filepath: P) -> Result<()> {
    let path = filepath.as_ref();
    let src = loader::load(path)?;
    let ast = Parser::parse_files(&src)?;
    log::debug!("created ast");
    let imports = load_builtin_exports()?;
    let hir = skc_ast2hir::make_hir(ast, &imports)?;
    log::debug!("created hir");
    let mir = skc_mir::build(hir, imports);
    log::debug!("created mir");
    let bc_path = path.with_extension("bc");
    let ll_path = path.with_extension("ll");
    let triple = targets::default_triple();
    skc_codegen::run(&mir, &bc_path, Some(&ll_path), true, Some(&triple))?;
    log::debug!("created .bc");
    Ok(())
}

pub fn compile_library<P: AsRef<Path>>(dir_: P) -> Result<()> {
    let dir = dir_.as_ref();
    let _package_info = SkPackage::load(dir.join("package.json5"))?;
    let path = dir.join("index.sk");
    let src = loader::load(&path)?;
    let ast = Parser::parse_files(&src)?;
    log::debug!("created ast");
    let imports = load_builtin_exports()?;
    let hir = skc_ast2hir::make_hir(ast, &imports)?;
    log::debug!("created hir");
    let mir = skc_mir::build(hir, imports);
    log::debug!("created mir");
    let exports = LibraryExports::new(&mir);
    let bc_path = path.with_extension("bc");
    let ll_path = path.with_extension("ll");
    let triple = targets::default_triple();
    skc_codegen::run(&mir, &bc_path, Some(&ll_path), true, Some(&triple))?;
    log::debug!("created .bc");
    exports.save(dir.join("exports.json"))?;
    log::debug!("created .json");
    Ok(())
}

/// Load builtin/exports.json
fn load_builtin_exports() -> Result<LibraryExports, Error> {
    let json_path = from_shiika_root("builtin/exports.json");
    let mut f = fs::File::open(&json_path).context(format!("{} not found", json_path.display()))?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .context("failed to read builtin exports")?;
    let exports: LibraryExports =
        serde_json::from_str(&contents).context("builtin exports is broken")?;
    Ok(exports)
}

/// Create builtin.bc and exports.json from builtin/*.sk and skc_corelib
pub fn build_corelib() -> Result<(), Error> {
    let builtin = load_builtin()?;
    let ast = Parser::parse_files(&builtin)?;
    log::debug!("created ast");
    let corelib = skc_corelib::create();
    log::debug!("loaded corelib");
    let imports = Default::default();
    let hir = skc_ast2hir::make_corelib_hir(ast, corelib)?;
    log::debug!("created hir");
    let mir = skc_mir::build(hir, imports);
    log::debug!("created mir");
    let exports = LibraryExports::new(&mir);
    let triple = targets::default_triple();
    skc_codegen::run(
        &mir,
        &from_shiika_root("builtin/builtin.bc"),
        Some(&from_shiika_root("builtin/builtin.ll")),
        false,
        Some(&triple),
    )?;
    log::debug!("created .bc");

    exports.save(from_shiika_root("builtin/exports.json"))?;
    log::debug!("created .json");
    debug_assert!(exports == load_builtin_exports()?);
    Ok(())
}

/// Load ./builtin/*.sk
fn load_builtin() -> Result<Vec<SourceFile>> {
    loader::load(&from_shiika_root("builtin/index.sk"))
}
