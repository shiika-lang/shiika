use anyhow::Result;
use insta::{assert_snapshot, glob};
use shiika::runner;
use std::path::Path;

#[test]
fn test_erroneous() -> Result<()> {
    let base = Path::new("./tests/erroneous/").canonicalize()?;
    glob!("erroneous/**/*.sk", |sk_path| {
        let result = match runner::compile(sk_path) {
            Ok(_) => "".to_string(),
            Err(comp_err) => comp_err.to_string(),
        };
        let rel_path = sk_path.strip_prefix(&base).unwrap();
        assert_snapshot!(result);
    });
    Ok(())
}
