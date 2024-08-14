use anyhow::{bail, Context, Result};
use ariadne::{Label, Report, ReportKind, Source};
use skc_async_experiment::{codegen, hir, hir_lowering, parser, prelude, verifier};
use std::io::Write;
use std::path::Path;

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let Some(path) = args.get(1) else {
        bail!("usage: cargo run --bin exp_shiika a.milika > a.mlir");
    };
    println!("--CUTHERE--");
    Main::new().run(path)
}

struct Main {
    log_file: std::fs::File,
}

impl Main {
    fn new() -> Self {
        Self {
            log_file: std::fs::File::create("log.milikac").unwrap(),
        }
    }

    fn run<P: AsRef<Path>>(&mut self, filepath: P) -> Result<()> {
        let path = filepath.as_ref();
        let src = std::fs::read_to_string(path)
            .context(format!("failed to read {}", &path.to_string_lossy()))?;
        let mut hir = self.compile(&src, &path.to_string_lossy(), false)?;

        let prelude_txt = prelude::prelude_funcs(main_is_async(&hir)?);
        let mut prelude_hir = self.compile(&prelude_txt, "src/prelude.rs", true)?;
        for e in prelude_hir.externs {
            if !e.is_internal {
                hir.externs.push(e);
            }
        }
        hir.funcs.append(&mut prelude_hir.funcs);

        self.log(&format!("# -- verifier input --\n{hir}\n"));
        verifier::run(&hir)?;

        let bc_path = path.with_extension("bc");
        let ll_path = path.with_extension("ll");
        codegen::run(bc_path, Some(ll_path), hir)?;
        Ok(())
    }

    fn compile(&mut self, src: &str, path: &str, is_prelude: bool) -> Result<hir::Program> {
        let ast = match parser::parse(src) {
            Ok(ast) => ast,
            Err(e) => {
                dbg!(&e);
                let span = e.location.offset..e.location.offset;
                Report::build(ReportKind::Error, path, e.location.offset)
                    .with_label(Label::new((path, span)).with_message("here"))
                    .finish()
                    .print((path, Source::from(src)))
                    .unwrap();
                bail!("parse error");
            }
        };
        let mut hir = hir::untyped::create(&ast)?;
        hir::typing::run(&mut hir)?;
        if !is_prelude {
            self.debug(format!("# -- typing output --\n{hir}\n"), !is_prelude);
            hir = hir::asyncness_check::run(hir);
            self.debug(
                format!("# -- asyncness_check output --\n{hir}\n"),
                !is_prelude,
            );
            hir = hir_lowering::async_splitter::run(hir)?;
            self.debug(
                format!("# -- async_splitter output --\n{hir}\n"),
                !is_prelude,
            );
        }
        Ok(hir)
    }

    fn debug(&mut self, s: String, print: bool) {
        if print {
            self.log(&s);
        }
    }

    fn log(&mut self, s: &str) {
        self.log_file.write_all(s.as_bytes()).unwrap();
    }
}

fn main_is_async(hir: &hir::Program) -> Result<bool> {
    let Some(main) = hir.funcs.iter().find(|x| x.name == "chiika_main") else {
        bail!("chiika_main not found");
    };
    // When chiika_main calls async function, it is lowered to take a continuation.
    Ok(main.params.len() > 0)
}
