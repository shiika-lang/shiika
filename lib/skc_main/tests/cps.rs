use anyhow::Result;
use insta::{assert_snapshot, glob};
use shiika_parser::{Parser, SourceFile};
use skc_async_experiment::{hir, hir_lowering, prelude};
use std::path::Path;

#[test]
fn test_cps_transformation() -> Result<()> {
    let base = Path::new(".").canonicalize()?;
    glob!("cps/**/*.sk", |sk_path_| {
        // Make the path relative to the project root so that the resulting .snap will be
        // identical on my machine and in the CI environment.
        let sk_path = sk_path_.strip_prefix(&base).unwrap();
        assert_snapshot!(compile(sk_path).unwrap());
    });
    Ok(())
}

fn compile(sk_path: &Path) -> Result<String> {
    let txt = std::fs::read_to_string(sk_path).unwrap();
    let src = SourceFile::new(sk_path.to_path_buf(), txt);
    let ast = Parser::parse_files(&[src])?;
    let mut hir = hir::untyped::create(&ast)?;
    hir.externs = prelude::lib_externs(Path::new("../skc_runtime/"))?
        .into_iter()
        .map(|(name, fun_ty)| hir::Extern { name, fun_ty })
        .collect();
    hir::typing::run(&mut hir)?;
    hir = hir::asyncness_check::run(hir);
    hir = hir_lowering::async_splitter::run(hir)?;
    let output = hir.to_string();
    Ok(output)
}
