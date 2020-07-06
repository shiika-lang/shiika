use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use crate::error::*;

/// Generate .ll from .sk
pub fn compile<P: AsRef<Path>>(filepath: P) -> Result<(), Box<dyn std::error::Error>> {
    let s = filepath.as_ref().to_str().expect("failed to unwrap filepath").to_string();
    let builtin = load_builtin()?;
    let str = builtin + &fs::read_to_string(filepath)?;
    let ast = crate::parser::Parser::parse(&str)?;
    let corelib = crate::corelib::Corelib::create();
    let hir = crate::hir::build(ast, corelib)?;
    crate::code_gen::run(&hir, &(s + ".ll"))?;
    Ok(())
}

fn load_builtin() -> Result<String, Box<dyn std::error::Error>> {
    let mut s = String::new();
    for item in fs::read_dir("builtin")? {
        let pathbuf = item?.path();
        let path = pathbuf.to_str().expect("Filename not utf8");
        if path.ends_with(".sk") {
            s += &fs::read_to_string(path)?;
        }
    }
    Ok(s)
}

/// Execute compiled .ll
pub fn run<P: AsRef<Path>>(sk_path: P) -> Result<(String, String), Box<dyn std::error::Error>> {
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

    let mut cmd = Command::new(env::var("LLC").unwrap_or("llc".to_string()));
    cmd.arg(ll_path.clone());
    let output = cmd.output()?;
    if !output.stderr.is_empty() {
        println!("{}", String::from_utf8(output.stderr)?);
    }

    let mut cmd = Command::new(env::var("CLANG").unwrap_or("clang".to_string()));
    add_args_from_env(&mut cmd, "CFLAGS");
    add_args_from_env(&mut cmd, "LDFLAGS");
    add_args_from_env(&mut cmd, "LDLIBS");
    cmd.arg("-no-pie");
    cmd.arg("-lm");
    cmd.arg("-lgc");
    cmd.arg("-o");
    cmd.arg(out_path.clone());
    cmd.arg(asm_path.clone());
    cmd.output()?;

    //fs::remove_file(bc_path)?;
    fs::remove_file(asm_path)?;

    let mut cmd = Command::new(out_path);
    let output = cmd.output().expect("failed to execute process");
    let stdout = String::from_utf8(output.stdout).expect("invalid utf8 in stdout");
    let stderr = String::from_utf8(output.stderr).expect("invalid utf8 in stderr");

    Ok((stdout, stderr))
}

fn add_args_from_env(cmd: &mut Command, key: &str) {
    for arg in env::var(key).unwrap_or("".to_string()).split_ascii_whitespace() {
        cmd.arg(arg);
    }
}
