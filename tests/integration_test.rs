use std::fs;
use std::path::Path;

#[test]
fn test_compile_and_run() -> Result<(), Box<dyn std::error::Error>> {
    let paths = fs::read_dir("tests/sk/")?;
    for item in paths {
        run_sk_test(&item.unwrap().path())?;
    }
    Ok(())
}

/// Execute tests/sk/x.sk
/// Fail if it prints something
fn run_sk_test(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    dbg!(&path);
    shiika::runner::compile(path)?;
    let (stdout, stderr) = shiika::runner::run(path)?;
    assert_eq!(stderr, "");
    assert_eq!(stdout, "ok\n");
    Ok(())
}
