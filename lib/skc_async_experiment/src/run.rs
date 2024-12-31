use crate::{codegen, hir, hir_to_mir, linker, mir, mir_lowering, prelude};
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
        let mut mir = self.compile(src)?;

        for (name, fun_ty) in prelude::core_externs() {
            mir.externs.push(mir::Extern { name, fun_ty });
        }
        mir.funcs.append(&mut prelude::funcs());

        self.log(&format!("# -- verifier input --\n{mir}\n"));
        mir::verifier::run(&mir)?;

        let bc_path = path.with_extension("bc");
        let ll_path = path.with_extension("ll");
        codegen::run(&bc_path, Some(&ll_path), mir)?;
        linker::run(bc_path)?;
        Ok(())
    }

    fn compile(&mut self, src: SourceFile) -> Result<mir::Program> {
        let ast = Parser::parse_files(&[src])?;
        let mut hir = hir::untyped::create(&ast)?;
        hir.externs = prelude::lib_externs(Path::new("lib/skc_runtime/"))?
            .into_iter()
            .map(|(name, fun_ty)| hir::Extern { name, fun_ty })
            .collect();
        hir::typing::run(&mut hir)?;
        self.log(format!("# -- typing output --\n{hir}\n"));
        let mut mir = hir_to_mir::run(hir);
        mir = mir_lowering::asyncness_check::run(mir);
        self.log(format!("# -- asyncness_check output --\n{mir}\n"));
        mir = mir_lowering::pass_async_env::run(mir);
        self.log(format!("# -- pass_async_env output --\n{mir}\n"));
        mir = mir_lowering::async_splitter::run(mir)?;
        self.log(format!("# -- async_splitter output --\n{mir}\n"));
        mir = mir_lowering::resolve_env_op::run(mir);
        Ok(mir)
    }

    fn log(&mut self, s: impl AsRef<str>) {
        self.log_file.write_all(s.as_ref().as_bytes()).unwrap();
    }
}
