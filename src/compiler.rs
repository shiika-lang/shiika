use crate::config::from_shiika_root;
use crate::loader;
use crate::package::SkPackage;
use crate::targets;
use anyhow::{anyhow, Context, Error, Result};
use shiika_parser::{Parser, SourceFile};
use skc_ast2hir;
use skc_codegen;
use skc_codegen::PackageName;
use skc_corelib;
use skc_mir::LibraryExports;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(PartialEq, Debug, Default, Clone)]
pub struct ExeDependencies {
    top_consts: HashSet<String>,
    exports: LibraryExports,
    link_files: Vec<PathBuf>,
}

impl ExeDependencies {
    pub fn new() -> ExeDependencies {
        ExeDependencies {
            top_consts: Default::default(),
            exports: LibraryExports::empty(),
            link_files: vec![PathBuf::from(from_shiika_root("builtin/builtin.bc"))],
        }
    }
}

pub fn build<P: AsRef<Path>>(dir_: P) -> Result<()> {
    let dir = dir_.as_ref();
    let package_info = SkPackage::load(dir)?;
    let mut exe_deps = ExeDependencies::new();
    _build(&package_info, &mut exe_deps)?;
    for main_file in package_info.spec.apps.as_ref().unwrap_or(&vec![]) {
        // PERF: avoid this clone
        compile_executable(main_file, exe_deps.exports.clone())?;
        create_executable(main_file, &exe_deps.link_files)?;
    }
    Ok(())
}

pub fn _build(package_info: &SkPackage, exe_deps: &mut ExeDependencies) -> Result<()> {
    // Build dependencies
    for dep in &package_info.spec.dependencies {
        let dep_package = dep.resolve()?;
        _build(&dep_package, exe_deps)?;
        if let Some(c) = &package_info.spec.export {
            exe_deps.top_consts.insert(c.clone());
        }
        exe_deps.link_files.extend(dep_package.link_files());
    }

    if package_info.spec.export.is_some() {
        let exports = compile_library(&package_info)?;
        exe_deps.exports.merge(exports);
    }
    Ok(())
}

/// Generate .ll from a .sk (without any dependendency)
pub fn compile_single<P: AsRef<Path>>(filepath: P) -> Result<()> {
    compile_executable(filepath, Default::default())
}

/// Generate .ll from .sk
fn compile_executable<P: AsRef<Path>>(path_: P, mut imports: LibraryExports) -> Result<()> {
    let path = path_.as_ref();
    imports.merge(load_builtin_exports()?);
    let mir = create_mir(path, imports)?;
    let bc_path = path.with_extension("bc");
    let ll_path = path.with_extension("ll");
    let triple = targets::default_triple();
    skc_codegen::run(
        &mir,
        &bc_path,
        Some(&ll_path),
        &PackageName::Main,
        Some(&triple),
    )?;
    log::debug!("created .bc");
    Ok(())
}

pub fn compile_library(package_info: &SkPackage) -> Result<LibraryExports> {
    let path = package_info.dir().join("index.sk");
    // TODO: Load dependencies of this library
    let imports = load_builtin_exports()?;
    let mir = create_mir(&path, imports)?;
    let exports = LibraryExports::new(&mir);
    let bc_path = path.with_extension("bc");
    let ll_path = path.with_extension("ll");
    let triple = targets::default_triple();
    skc_codegen::run(
        &mir,
        &bc_path,
        Some(&ll_path),
        &PackageName::Library(package_info.spec.export.as_ref().unwrap().clone()),
        Some(&triple),
    )?;
    log::debug!("created .bc");
    exports.save(package_info.dir().join("exports.json"))?;
    log::debug!("created .json");
    Ok(exports)
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
        &PackageName::Builtin,
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

/// Create MIR from an entry point .sk
fn create_mir<P: AsRef<Path>>(path_: P, imports: LibraryExports) -> Result<skc_mir::Mir> {
    let path = path_.as_ref();
    let src = loader::load(path)?;
    let ast = Parser::parse_files(&src)?;
    log::debug!("created ast");
    let hir = skc_ast2hir::make_hir(ast, &imports)?;
    log::debug!("created hir");
    let mir = skc_mir::build(hir, imports);
    log::debug!("created mir");
    Ok(mir)
}

/// Create an executable by linkng *.bc, *.a, etc.
pub fn create_executable<P: AsRef<Path>>(sk_path_: P, link_files: &[PathBuf]) -> Result<PathBuf> {
    let triple = targets::default_triple();
    let sk_path = sk_path_.as_ref();
    let bc_path = sk_path.with_extension("bc");
    let exe_path = if cfg!(target_os = "windows") {
        sk_path.canonicalize()?.with_extension("exe")
    } else {
        sk_path.with_extension("out")
    };

    let mut cmd = Command::new(env::var("CLANG").unwrap_or_else(|_| "clang".to_string()));
    add_args_from_env(&mut cmd, "CFLAGS");
    add_args_from_env(&mut cmd, "LDFLAGS");
    add_args_from_env(&mut cmd, "LDLIBS");
    cmd.arg("-target");
    cmd.arg(triple.as_str().to_str().unwrap());
    if cfg!(target_os = "linux") {
        cmd.arg("-lm");
    }
    if cfg!(target_os = "macos") {
        // Link CoreFoundation for timezones for `Time`
        cmd.arg("-framework");
        cmd.arg("Foundation");
    }
    cmd.arg("-o");
    cmd.arg(exe_path.clone());
    for p in link_files {
        cmd.arg(p);
    }
    let skc_rustlib = if cfg!(target_os = "windows") {
        "skc_rustlib.lib"
    } else {
        "libskc_rustlib.a"
    };
    cmd.arg(cargo_target_path().join("debug").join(skc_rustlib));
    cmd.arg(bc_path.clone());

    if cfg!(target_os = "windows") {
        cmd.arg("-luser32");
        cmd.arg("-lkernel32");
        cmd.arg("-lws2_32");

        cmd.arg("-Xlinker");
        cmd.arg("/NODEFAULTLIB");
        cmd.arg("-lmsvcrt");
        cmd.arg("-lucrt");
        cmd.arg("-lvcruntime");
        //cmd.arg("-lucrt");

        cmd.arg("-lbcrypt");
        cmd.arg("-ladvapi32");
        cmd.arg("-luserenv");
    } else {
        cmd.arg("-ldl");
        cmd.arg("-lpthread");
    }

    log::debug!("{:?}", cmd);

    if !cmd.status()?.success() {
        return Err(anyhow!("clang failed"));
    }

    fs::remove_file(bc_path)?;
    Ok(exe_path)
}

fn add_args_from_env(cmd: &mut Command, key: &str) {
    for arg in env::var(key)
        .unwrap_or_else(|_| "".to_string())
        .split_ascii_whitespace()
    {
        cmd.arg(arg);
    }
}

fn cargo_target_path() -> PathBuf {
    if let Ok(s) = env::var("SHIIKA_CARGO_TARGET") {
        PathBuf::from(s)
    } else {
        from_shiika_root("target")
    }
}
