use anyhow::{anyhow, Result};
use shiika::runner;
use std::env;
use std::fs;

#[test]
fn test_compile_and_run() -> Result<()> {
    let filter = env::var("FILTER").ok();
    let paths = fs::read_dir("tests/sk/")?;
    for item in paths {
        let pathbuf = item?.path();
        let path = pathbuf
            .to_str()
            .ok_or_else(|| anyhow!("Filename not utf8"))?;
        if path.ends_with(".sk") {
            if let Some(s) = &filter {
                if !path.contains(s) {
                    continue;
                }
            }
            run_sk_test(path)?;
        }
    }
    Ok(())
}

#[test]
fn test_no_panic() -> Result<()> {
    let path = "tests/no_panic.sk";
    // `compile` may return an Err here; it just should not panic.
    let _ = runner::compile(path);
    runner::cleanup(path)?;
    Ok(())
}

/// Execute tests/sk/x.sk
/// Fail if it prints something
fn run_sk_test(path: &str) -> Result<()> {
    dbg!(&path);
    runner::compile(path)?;
    let (stdout, stderr) = runner::run_and_capture(path)?;
    assert_eq!(stderr, "");
    assert_eq!(stdout, "ok\n");
    runner::cleanup(path)?;
    Ok(())
}
