use crate::{codegen, hir, hir_to_mir, linker, mir, mir_lowering, prelude};
use anyhow::{bail, Context, Result};
use shiika_core::names::method_fullname_raw;
use shiika_core::ty::{self, Erasure};
use shiika_parser::{Parser, SourceFile};
use skc_hir::{MethodSignature, MethodSignatures, SkTypeBase};
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

        let hir = {
            let mut imports = create_imports();
            let imported_asyncs =
                prelude::load_lib_externs(Path::new("lib/skc_runtime/"), &mut imports)?;

            let defs = ast.defs();
            let type_index =
                skc_ast2hir::type_index::create(&defs, &Default::default(), &imports.sk_types);
            let class_dict = skc_ast2hir::class_dict::create(&defs, type_index, &imports.sk_types)?;

            let hir = hir::untyped::create(&ast)?;
            let mut hir = hir::typing::run(hir, &class_dict)?;
            hir.imports = imports;
            hir.imported_asyncs = imported_asyncs;
            hir
        };
        let mir = hir_to_mir::run(hir);
        self.log(format!("# -- typing output --\n{mir}\n"));
        let mir = mir_lowering::asyncness_check::run(mir);
        self.log(format!("# -- asyncness_check output --\n{mir}\n"));
        let mir = mir_lowering::pass_async_env::run(mir);
        self.log(format!("# -- pass_async_env output --\n{mir}\n"));
        let mir = mir_lowering::async_splitter::run(mir)?;
        self.log(format!("# -- async_splitter output --\n{mir}\n"));
        let mir = mir_lowering::resolve_env_op::run(mir);
        Ok(mir)
    }

    fn log(&mut self, s: impl AsRef<str>) {
        self.log_file.write_all(s.as_ref().as_bytes()).unwrap();
    }
}

// TODO: should be built from ./buitlin
fn create_imports() -> skc_mir::LibraryExports {
    let object_initialize = MethodSignature {
        fullname: method_fullname_raw("Object", "initialize"),
        ret_ty: ty::raw("Object"),
        params: vec![],
        typarams: vec![],
    };
    let class_object = {
        let base = SkTypeBase {
            erasure: Erasure::nonmeta("Object"),
            typarams: Default::default(),
            method_sigs: MethodSignatures::from_iterator(vec![object_initialize].into_iter()),
            foreign: false,
        };
        skc_hir::SkClass::nonmeta(base, None)
    };
    let class_int = {
        let base = SkTypeBase {
            erasure: Erasure::nonmeta("Int"),
            typarams: Default::default(),
            method_sigs: MethodSignatures::from_iterator(vec![].into_iter()),
            foreign: false,
        };
        skc_hir::SkClass::nonmeta(base, None)
    };

    let class_class = {
        let base = SkTypeBase {
            erasure: Erasure::nonmeta("Class"),
            typarams: Default::default(),
            method_sigs: MethodSignatures::from_iterator(vec![].into_iter()),
            foreign: false,
        };
        skc_hir::SkClass::nonmeta(base, None)
    };

    skc_mir::LibraryExports {
        sk_types: skc_hir::SkTypes::from_iterator(
            vec![class_object.into(), class_int.into(), class_class.into()].into_iter(),
        ),
        vtables: Default::default(),
        constants: Default::default(),
    }
}
