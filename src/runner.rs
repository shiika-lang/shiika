use crate::error::*;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Generate .ll from .sk
pub fn compile<P: AsRef<Path>>(filepath: P) -> Result<(), Box<dyn std::error::Error>> {
    let path = filepath
        .as_ref()
        .to_str()
        .expect("failed to unwrap filepath")
        .to_string();
    let builtin = load_builtin()?;
    let str = builtin
        + &fs::read_to_string(filepath)
            .map_err(|e| runner_error(format!("{} is not utf8", path), e))?;
    let ast = crate::parser::Parser::parse(&str)?;
    let corelib = crate::corelib::Corelib::create();
    let hir = crate::hir::build(ast, corelib)?;
    crate::code_gen::run(&hir, &(path + ".ll"))?;
    Ok(())
}

fn load_builtin() -> Result<String, Box<dyn std::error::Error>> {
    let mut s = String::new();
    let dir = fs::read_dir("builtin").map_err(|e| runner_error("./builtin not found", e))?;
    for item in dir {
        let pathbuf = item?.path();
        let path = pathbuf
            .to_str()
            .ok_or_else(|| plain_runner_error("Filename not utf8"))?;
        if path.ends_with(".sk") {
            s += &fs::read_to_string(path)
                .map_err(|e| runner_error(format!("failed to load {}", path), e))?;
        }
    }
    Ok(s)
}

/// Execute compiled .ll
pub fn run<P: AsRef<Path>>(sk_path: P) -> Result<(), Box<dyn std::error::Error>> {
    run_(sk_path, false)?;
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
    let output = cmd
        .output()
        .map_err(|e| runner_error("failed to run llc", e))?;
    if !output.stderr.is_empty() {
        let s = String::from_utf8(output.stderr)
            .map_err(|e| runner_error("llc output is not utf8", e))?;
        println!("{}", s);
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
        return Err(Box::new(plain_runner_error("failed to run clang")));
    }

    //fs::remove_file(bc_path)?;
    fs::remove_file(asm_path).map_err(|e| runner_error("failed to remove .s", e))?;

    let mut cmd = Command::new(out_path);
    if capture_out {
        let output = cmd
            .output()
            .map_err(|e| runner_error("failed to execute process", e))?;
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
