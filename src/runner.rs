use crate::error::*;
use log;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Generate .ll from .sk
pub fn compile<P: AsRef<Path>>(filepath: P) -> Result<(), Error> {
    let path = filepath
        .as_ref()
        .to_str()
        .expect("failed to unwrap filepath")
        .to_string();
    let builtin = wrap_error(load_builtin())?;
    let str = builtin
        + &fs::read_to_string(filepath)
            .map_err(|e| runner_error(format!("{} is not utf8", path), Box::new(e)))?;
    let ast = crate::parser::Parser::parse(&str)?;
    log::debug!("created ast");
    let corelib = crate::corelib::Corelib::create();
    log::debug!("loaded corelib");
    let hir = crate::hir::build(ast, corelib)?;
    log::debug!("created hir");
    let mir = crate::mir::build(hir);
    log::debug!("created mir");
    wrap_error(crate::code_gen::run(&mir, &(path + ".ll")))?;
    log::debug!("created .ll");
    Ok(())
}

fn load_builtin() -> Result<String, Box<dyn std::error::Error>> {
    let mut s = String::new();
    let dir =
        fs::read_dir("builtin").map_err(|e| runner_error("./builtin not found", Box::new(e)))?;
    for item in dir {
        let pathbuf = item?.path();
        let path = pathbuf
            .to_str()
            .ok_or_else(|| plain_runner_error("Filename not utf8"))?;
        if path.ends_with(".sk") {
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
    let s = sk_path.as_ref().to_str().expect("failed to unwrap sk_path");
    let ll_path = s.to_string() + ".ll";
    //let opt_ll_path = s.to_string() + ".opt.ll";
    //let bc_path = s.to_string() + ".bc";
    let asm_path = s.to_string() + ".s";
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

    let mut cmd = Command::new(env::var("LLC").unwrap_or_else(|_| "llc".to_string()));
    cmd.arg(ll_path);
    cmd.output()
        .map_err(|e| runner_error("failed to run llc", Box::new(e)))?;
    if !cmd.status()?.success() {
        return Err(Box::new(plain_runner_error("llc failed")));
    }

    let mut cmd = Command::new(env::var("CLANG").unwrap_or_else(|_| "clang".to_string()));
    add_args_from_env(&mut cmd, "CFLAGS");
    add_args_from_env(&mut cmd, "LDFLAGS");
    add_args_from_env(&mut cmd, "LDLIBS");
    cmd.arg("-no-pie");
    cmd.arg("-lm");
    cmd.arg("-lgc");
    cmd.arg("-o");
    cmd.arg(out_path.clone());
    cmd.arg(asm_path.clone());
    if !cmd.status()?.success() {
        return Err(Box::new(plain_runner_error("clang failed")));
    }

    //fs::remove_file(bc_path)?;
    fs::remove_file(asm_path).map_err(|e| runner_error("failed to remove .s", Box::new(e)))?;

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

/// Remove .ll and .out
pub fn cleanup<P: AsRef<Path>>(sk_path: P) -> Result<(), Box<dyn std::error::Error>> {
    let s = sk_path.as_ref().to_str().expect("failed to unwrap sk_path");
    let ll_path = s.to_string() + ".ll";
    let out_path = s.to_string() + ".out";
    fs::remove_file(ll_path)?;
    fs::remove_file(out_path)?;
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

fn wrap_error<T>(result: Result<T, Box<dyn std::error::Error>>) -> Result<T, Error> {
    result.map_err(|err| runner_error(format!("{}", err), err))
}
