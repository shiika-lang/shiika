use shiika::error::*;
use std::fs;
use std::env;

#[test]
fn test_compile_and_run() -> Result<(), Box<dyn std::error::Error>> {
    let filter = env::var("FILTER").ok();
    let paths = fs::read_dir("tests/sk/")?;
    for item in paths {
        let pathbuf = item?.path();
        let path = pathbuf
            .to_str()
            .ok_or(plain_runner_error("Filename not utf8"))?;
        if path.ends_with(".sk") {
            if let Some(s) = &filter {
                if !path.contains(s) { continue; }
            }
            run_sk_test(path)?;
        }
    }
    Ok(())
}

/// Execute tests/sk/x.sk
/// Fail if it prints something
fn run_sk_test(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    dbg!(&path);
    shiika::runner::compile(path)?;
    let (stdout, stderr) = shiika::runner::run_and_capture(path)?;
    assert_eq!(stderr, "");
    assert_eq!(stdout, "ok\n");
    shiika::runner::cleanup(path)?;
    Ok(())
}
