use crate::compiler;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
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
    let exe_path = compiler::create_executable(sk_path_)?;
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
