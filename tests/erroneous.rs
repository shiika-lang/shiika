use anyhow::Result;
use insta::{assert_snapshot, glob};
use skc_async_experiment::run;
use std::path::Path;

#[test]
fn test_erroneous() -> Result<()> {
    // `Cli::init` requires SHIIKA_ROOT; default it to the crate root so that
    // `cargo test` works without extra setup.
    if std::env::var("SHIIKA_ROOT").is_err() {
        std::env::set_var("SHIIKA_ROOT", ".");
    }
    let base = Path::new(".").canonicalize()?;
    glob!("erroneous/**/*.sk", |sk_path_| {
        // Make the path relative to the project root so that the resulting .snap will be
        // identical on my machine and in the CI environment.
        let sk_path = sk_path_.strip_prefix(&base).unwrap();
        let compiler_output = match run::compile(sk_path) {
            Ok(_) => "".to_string(),
            Err(comp_err) => comp_err.to_string(),
        };
        assert_snapshot!(compiler_output);
    });
    Ok(())
}
