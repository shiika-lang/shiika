use crate::error::*;
use crate::library;
use crate::targets;
use rand::prelude::SliceRandom;
use rand::SeedableRng;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

/// Generate .ll from .sk
pub fn compile<P: AsRef<Path>>(filepath: P) -> Result<(), Error> {
    let path = filepath
        .as_ref()
        .to_str()
        .expect("failed to unwrap filepath")
        .to_string();
    let str = fs::read_to_string(filepath)
        .map_err(|e| runner_error(format!("{} is not utf8", path), Box::new(e)))?;
    let ast = crate::parser::Parser::parse(&str)?;
    log::debug!("created ast");
    let imports = load_builtin_exports()?;
    let hir = crate::hir::build(ast, None, &imports)?;
    log::debug!("created hir");
    let mir = crate::mir::build(hir, imports);
    log::debug!("created mir");
    let bc_path = path.clone() + ".bc";
    let ll_path = path + ".ll";
    let triple = targets::default_triple();
    crate::code_gen::run(&mir, &bc_path, Some(&ll_path), true, Some(&triple))?;
    log::debug!("created .bc");
    Ok(())
}

pub fn load_builtin_exports() -> Result<library::LibraryExports, Error> {
    let mut f = fs::File::open("builtin/exports.json")
        .map_err(|e| runner_error("builtin exports not found", Box::new(e)))?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .map_err(|e| runner_error("failed to read builtin exports", Box::new(e)))?;
    let exports: library::LibraryExports = serde_json::from_str(&contents)
        .map_err(|e| runner_error("builtin exports is broken", Box::new(e)))?;
    Ok(exports)
}

pub fn build_corelib() -> Result<(), Error> {
    let builtin = wrap_error(load_builtin())?;
    let ast = crate::parser::Parser::parse(&builtin)?;
    log::debug!("created ast");
    let corelib = crate::corelib::create();
    log::debug!("loaded corelib");
    let imports = Default::default();
    let hir = crate::hir::build(ast, Some(corelib), &imports)?;
    log::debug!("created hir");
    let mir = crate::mir::build(hir, imports);
    log::debug!("created mir");
    let exports = library::LibraryExports::new(&mir);
    let triple = targets::default_triple();
    crate::code_gen::run(
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

/// Load ./builtin/*.sk into a String
fn load_builtin() -> Result<String, Box<dyn std::error::Error>> {
    let mut s = String::new();
    let dir =
        fs::read_dir("builtin").map_err(|e| runner_error("./builtin not found", Box::new(e)))?;
    let mut files = dir.collect::<Vec<_>>();
    if false {
        // Randomize loading order (for debugging purpose)
        let mut rng = rand_pcg::Pcg64::seed_from_u64(123);
        files.shuffle(&mut rng);
    }
    for item in files {
        let pathbuf = item?.path();
        let path = pathbuf
            .to_str()
            .ok_or_else(|| plain_runner_error("Filename not utf8"))?;
        if path.ends_with(".sk") {
            //dbg!(&path);
            s += &fs::read_to_string(path)
                .map_err(|e| runner_error(format!("failed to load {}", path), Box::new(e)))?;
        }
    }
    Ok(s)
}

/// Execute compiled .ll
pub fn run<P: AsRef<Path>>(sk_path: P) -> Result<(), Error> {
    wrap_error(run_(sk_path, false))?;
    Ok(())
}

/// Execute compiled .ll and return the outputs
pub fn run_and_capture<P: AsRef<Path>>(
    sk_path: P,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    run_(sk_path, true)
}

fn run_<P: AsRef<Path>>(
    sk_path: P,
    capture_out: bool,
) -> Result<(String, String), Box<dyn std::error::Error>> {
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
    cmd.arg("-o");
    cmd.arg(out_path.clone());
    cmd.arg("builtin/builtin.bc");
    cmd.arg("src/rustlib/target/debug/librustlib.a");
    cmd.arg(bc_path.clone());
    cmd.arg("-ldl");
    cmd.arg("-lpthread");
    if !cmd.status()?.success() {
        return Err(Box::new(plain_runner_error("clang failed")));
    }

    fs::remove_file(bc_path)?;

    let mut cmd = Command::new(format!("./{}", out_path));
    if capture_out {
        let output = cmd
            .output()
            .map_err(|e| runner_error("failed to execute process", Box::new(e)))?;
        let stdout = String::from_utf8(output.stdout).expect("invalid utf8 in stdout");
        let stderr = String::from_utf8(output.stderr).expect("invalid utf8 in stderr");
        Ok((stdout, stderr))
    } else {
        cmd.status()?;
        Ok(("".to_string(), "".to_string()))
    }
}

/// Remove .bc and .out
pub fn cleanup<P: AsRef<Path>>(sk_path: P) -> Result<(), Box<dyn std::error::Error>> {
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

/// Wrap std::error::Error with shiika::error::Error
fn wrap_error<T>(result: Result<T, Box<dyn std::error::Error>>) -> Result<T, Error> {
    result.map_err(|err| runner_error(format!("{}", err), err))
}
