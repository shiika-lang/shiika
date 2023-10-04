use anyhow::Result;
use concolor;
use insta::{assert_snapshot, glob};
use shiika::runner;

#[test]
fn test_erroneous() -> Result<()> {
    concolor::set(concolor::ColorChoice::Never);
    glob!("erroneous/**/*.sk", |sk_path| {
        let compiler_output = match runner::compile(sk_path) {
            Ok(_) => "".to_string(),
            Err(comp_err) => comp_err.to_string(),
        };
        assert_snapshot!(compiler_output);
    });
    Ok(())
}
