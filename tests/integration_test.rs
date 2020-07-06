use std::fs;
use shiika::error::*;

#[test]
fn test_compile_and_run() -> Result<(), Box<dyn std::error::Error>> {
    let paths = fs::read_dir("tests/sk/")?;
    for item in paths {
        let pathbuf = item?.path();
        let path = pathbuf.to_str()
            .ok_or(plain_runner_error("Filename not utf8"))?;
        if path.ends_with(".sk") {
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
    let (stdout, stderr) = shiika::runner::run(path)?;
    assert_eq!(stderr, "");
    assert_eq!(stdout, "ok\n");
    shiika::runner::cleanup(path)?;
    Ok(())
}
