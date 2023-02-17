use crate::config::from_shiika_root;
use crate::targets;
use anyhow::{anyhow, Context, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    cmd.arg(from_shiika_root("builtin/builtin.bc"));
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

    if !cmd.status()?.success() {
        return Err(anyhow!("clang failed"));
    }

    fs::remove_file(bc_path)?;

    let mut cmd = Command::new(&exe_path);
    if capture_out {
        let output = cmd.output().context("failed to execute process")?;
        let stdout = String::from_utf8(output.stdout).expect("invalid utf8 in stdout");
        let stderr = String::from_utf8(output.stderr).expect("invalid utf8 in stderr");
        Ok((stdout, stderr))
    } else {
        cmd.status()
            .context(format!("failed to run {}", exe_path.display()))?;
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

fn cargo_target_path() -> PathBuf {
    if let Ok(s) = env::var("SHIIKA_CARGO_TARGET") {
        PathBuf::from(s)
    } else {
        from_shiika_root("target")
    }
}
