use crate::loader;
use crate::targets;
use anyhow::{anyhow, Context, Error, Result};
use shiika_parser::{Parser, SourceFile};
use skc_ast2hir;
use skc_codegen;
use skc_corelib;
use skc_mir::LibraryExports;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

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

    let json = serde_json::to_string_pretty(&exports).unwrap();
    let mut f = fs::File::create(from_shiika_root("builtin/exports.json")).unwrap();
    f.write_all(json.as_bytes()).unwrap();
    log::debug!("created .json");
    Ok(())
}

/// Load ./builtin/*.sk
fn load_builtin() -> Result<Vec<SourceFile>> {
    loader::load(&from_shiika_root("builtin/index.sk"))
}

/// Execute compiled .ll
pub fn run<P: AsRef<Path>>(sk_path: P) -> Result<()> {
    run_(sk_path, false)?;
    Ok(())
}

/// Execute compiled .ll and return the outputs (for tests)
pub fn run_and_capture<P: AsRef<Path>>(sk_path: P) -> Result<(String, String)> {
    run_(sk_path, true)
}

fn run_<P: AsRef<Path>>(sk_path_: P, capture_out: bool) -> Result<(String, String)> {
    let triple = targets::default_triple();
    let sk_path = sk_path_.as_ref();
    let bc_path = sk_path.with_extension("bc");
    let exe_path = if cfg!(target_os = "windows") {
        sk_path.with_extension("exe")
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
    cmd.arg(from_shiika_root("builtin/builtin.bc"));
    let cargo_target = env::var("SHIIKA_CARGO_TARGET").unwrap_or_else(|_| "target".to_string());
    if cfg!(target_os = "windows") {
        cmd.arg(format!("{}/debug/skc_rustlib.lib", cargo_target));
    } else {
        cmd.arg(format!("{}/debug/libskc_rustlib.a", cargo_target));
    }
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

    if !cmd.status()?.success() {
        return Err(anyhow!("clang failed"));
    }

    fs::remove_file(bc_path)?;

    let mut cmd = Command::new(exe_path);
    if capture_out {
        let output = cmd.output().context("failed to execute process")?;
        let stdout = String::from_utf8(output.stdout).expect("invalid utf8 in stdout");
        let stderr = String::from_utf8(output.stderr).expect("invalid utf8 in stderr");
        Ok((stdout, stderr))
    } else {
        cmd.status()?;
        Ok(("".to_string(), "".to_string()))
    }
}

/// Remove .bc and .out (used by unit tests)
pub fn cleanup<P: AsRef<Path>>(sk_path_: P) -> Result<()> {
    let sk_path = sk_path_.as_ref();
    let bc_path = sk_path.with_extension("bc");
    let exe_path = if cfg!(target_os = "windows") {
        sk_path.with_extension("exe")
    } else {
        sk_path.with_extension("out")
    };
    let _ = fs::remove_file(bc_path);
    let _ = fs::remove_file(exe_path);
    Ok(())
}

fn add_args_from_env(cmd: &mut Command, key: &str) {
    for arg in env::var(key)
        .unwrap_or_else(|_| "".to_string())
        .split_ascii_whitespace()
    {
        cmd.arg(arg);
    }
}

fn from_shiika_root(s: &str) -> PathBuf {
    shiika_root().join(s)
}

fn shiika_root() -> PathBuf {
    PathBuf::from(env::var("SHIIKA_ROOT").unwrap_or_else(|_| ".".to_string()))
}
