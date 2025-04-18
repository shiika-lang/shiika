use crate::targets;
use anyhow::{anyhow, Result};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Build a .bc file to an executable using clang.
/// Returns the path to the executable.
pub fn run<P: AsRef<Path>>(bc_path_: P, deps: &[PathBuf]) -> Result<PathBuf> {
    let bc_path = bc_path_.as_ref();
    let exe_ext = if cfg!(target_os = "windows") {
        "exe"
    } else {
        // Using "out" to gitignore test outputs
        // TODO: Option to set the output filename
        "out"
    };
    let exe_path = bc_path.canonicalize()?.with_extension(exe_ext);
    let mut cmd = build_clang_cmd(bc_path, exe_path.clone(), deps);
    if !cmd.status()?.success() {
        return Err(anyhow!("clang failed: {:?}", cmd));
    }
    Ok(exe_path)
}

fn build_clang_cmd(bc_path: &Path, exe_path: PathBuf, deps: &[PathBuf]) -> Command {
    let triple = targets::default_triple();
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
    cmd.arg(bc_path.to_path_buf());

    for dep in deps {
        cmd.arg(dep);
    }

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

    cmd
}

fn add_args_from_env(cmd: &mut Command, key: &str) {
    for arg in env::var(key)
        .unwrap_or_else(|_| "".to_string())
        .split_ascii_whitespace()
    {
        cmd.arg(arg);
    }
}
