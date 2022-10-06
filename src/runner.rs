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
use std::path::Path;
use std::process::Command;

/// Generate .ll from .sk
pub fn compile<P: AsRef<Path>>(filepath: P) -> Result<()> {
    let path = filepath
        .as_ref()
        .to_str()
        .expect("failed to unwrap filepath")
        .to_string();
    let src = loader::load(filepath.as_ref())?;
    let ast = Parser::parse_files(&src)?;
    log::debug!("created ast");
    let imports = load_builtin_exports()?;
    let hir = skc_ast2hir::make_hir(ast, &imports)?;
    log::debug!("created hir");
    let mir = skc_mir::build(hir, imports);
    log::debug!("created mir");
    let bc_path = path.clone() + ".bc";
    let ll_path = path + ".ll";
    let triple = targets::default_triple();
    skc_codegen::run(&mir, &bc_path, Some(&ll_path), true, Some(&triple))?;
    log::debug!("created .bc");
    Ok(())
}

/// Load builtin/exports.json
fn load_builtin_exports() -> Result<LibraryExports, Error> {
    let mut f = fs::File::open("builtin/exports.json").context("builtin exports not found")?;
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
        "builtin/builtin.bc",
        Some("builtin/builtin.ll"),
        false,
        Some(&triple),
    )?;
    log::debug!("created .bc");

    let json = serde_json::to_string_pretty(&exports).unwrap();
    let mut f = fs::File::create("builtin/exports.json").unwrap();
    f.write_all(json.as_bytes()).unwrap();
    log::debug!("created .json");
    Ok(())
}

/// Load ./builtin/*.sk
fn load_builtin() -> Result<Vec<SourceFile>> {
    loader::load(Path::new("./builtin/index.sk"))
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

fn run_<P: AsRef<Path>>(sk_path: P, capture_out: bool) -> Result<(String, String)> {
    let triple = targets::default_triple();
    let s = sk_path.as_ref().to_str().expect("failed to unwrap sk_path");
    //let ll_path = s.to_string() + ".ll";
    //let opt_ll_path = s.to_string() + ".opt.ll";
    let bc_path = s.to_string() + ".bc";
    //let asm_path = s.to_string() + ".s";
    let out_path = s.to_string() + ".out";

    //    let mut cmd = Command::new("opt");
    //    cmd.arg("-O3");
    //    cmd.arg(ll_path);
    //    cmd.arg("-o");
    //    cmd.arg(bc_path.clone());
    //    let output = cmd.output()?;
    //    if !output.stderr.is_empty() {
    //        println!("{}", String::from_utf8(output.stderr)?);
    //    }
    //
    //    let mut cmd = Command::new("llvm-dis");
    //    cmd.arg(bc_path.clone());
    //    cmd.arg("-o");
    //    cmd.arg(opt_ll_path);
    //    cmd.output()?;

    // let mut cmd = Command::new(env::var("LLC").unwrap_or_else(|_| "llc".to_string()));
    // cmd.arg(ll_path);
    // cmd.output()
    //     .map_err(|e| runner_error("failed to run llc", Box::new(e)))?;
    // if !cmd.status()?.success() {
    //     return Err(Box::new(plain_runner_error("llc failed")));
    // }

    let mut cmd = Command::new(env::var("CLANG").unwrap_or_else(|_| "clang".to_string()));
    add_args_from_env(&mut cmd, "CFLAGS");
    add_args_from_env(&mut cmd, "LDFLAGS");
    add_args_from_env(&mut cmd, "LDLIBS");
    //cmd.arg("-no-pie");
    cmd.arg("-target");
    cmd.arg(triple.as_str().to_str().unwrap());
    cmd.arg("-lm");
    if cfg!(target_os = "macos") {
        // Link CoreFoundation for timezones for `Time`
        cmd.arg("-framework");
        cmd.arg("Foundation");
    }
    cmd.arg("-o");
    cmd.arg(out_path.clone());
    cmd.arg("builtin/builtin.bc");
    let cargo_target = env::var("SHIIKA_CARGO_TARGET").unwrap_or_else(|_| "target".to_string());
    cmd.arg(format!("{}/debug/libskc_rustlib.a", cargo_target));
    cmd.arg(bc_path.clone());
    cmd.arg("-ldl");
    cmd.arg("-lpthread");
    if !cmd.status()?.success() {
        return Err(anyhow!("clang failed"));
    }

    fs::remove_file(bc_path)?;

    let exe_path = if out_path.starts_with('/') {
        out_path
    } else {
        format!("./{}", out_path)
    };
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

/// Remove .bc and .out
pub fn cleanup<P: AsRef<Path>>(sk_path: P) -> Result<()> {
    let s = sk_path.as_ref().to_str().expect("failed to unwrap sk_path");
    let bc_path = s.to_string() + ".bc";
    let out_path = s.to_string() + ".out";
    let _ = fs::remove_file(bc_path);
    let _ = fs::remove_file(out_path);
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
