use crate::names::FunctionName;
use crate::{codegen, hir, hir_lowering, linker, prelude};
use anyhow::{bail, Context, Result};
use shiika_parser::{Parser, SourceFile};
use std::io::Write;
use std::path::Path;

pub fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let Some(path) = args.get(1) else {
        bail!("usage: cargo run --bin exp_shiika a.milika > a.mlir");
    };
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
        let txt = std::fs::read_to_string(path)
            .context(format!("failed to read {}", &path.to_string_lossy()))?;
        let src = SourceFile::new(path.to_path_buf(), txt);
        let mut hir = self.compile(src, false)?;

        for (name, fun_ty) in prelude::core_externs() {
            hir.externs.push(hir::Extern { name, fun_ty });
        }
        hir.funcs.append(&mut prelude::funcs(main_is_async(&hir)?));

        self.log(&format!("# -- verifier input --\n{hir}\n"));
        hir::verifier::run(&hir)?;

        let bc_path = path.with_extension("bc");
        let ll_path = path.with_extension("ll");
        codegen::run(&bc_path, Some(&ll_path), hir)?;
        linker::run(bc_path)?;
        Ok(())
    }

    fn compile(&mut self, src: SourceFile, is_prelude: bool) -> Result<hir::Program> {
        let ast = Parser::parse_files(&[src])?;
        let mut hir = hir::untyped::create(&ast)?;
        hir.externs = prelude::lib_externs(Path::new("lib/skc_runtime/"))?
            .into_iter()
            .map(|(name, fun_ty)| hir::Extern { name, fun_ty })
            .collect();
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
    let Some(main) = hir
        .funcs
        .iter()
        .find(|x| x.name == FunctionName::unmangled("chiika_main"))
    else {
        bail!("chiika_main not found");
    };
    // When chiika_main calls async function, it is lowered to take a continuation.
    Ok(main.params.len() > 0)
}
