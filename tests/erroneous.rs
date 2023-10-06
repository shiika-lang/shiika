use anyhow::Result;
use concolor;
use insta::{assert_snapshot, glob};
use shiika::runner;
use std::path::Path;

#[test]
fn test_erroneous() -> Result<()> {
    concolor::set(concolor::ColorChoice::Never);
    let base = Path::new(".").canonicalize()?;
    glob!("erroneous/**/*.sk", |sk_path_| {
        // Make the path relative to the project root so that the resulting .snap will be
        // identical on my machine and in the CI environment.
        let sk_path = sk_path_.strip_prefix(&base).unwrap();
        let compiler_output = match runner::compile(sk_path) {
            Ok(_) => "".to_string(),
            Err(comp_err) => comp_err.to_string(),
        };
        assert_snapshot!(compiler_output);
    });
    Ok(())
}
