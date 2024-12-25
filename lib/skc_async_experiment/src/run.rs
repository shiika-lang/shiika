use crate::{codegen, hir, hir_lowering, linker, prelude};
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
        let mut hir = self.compile(src)?;

        for (name, fun_ty) in prelude::core_externs() {
            hir.externs.push(hir::Extern { name, fun_ty });
        }
        hir.funcs.append(&mut prelude::funcs());

        self.log(&format!("# -- verifier input --\n{hir}\n"));
        hir::verifier::run(&hir)?;

        let bc_path = path.with_extension("bc");
        let ll_path = path.with_extension("ll");
        codegen::run(&bc_path, Some(&ll_path), hir)?;
        linker::run(bc_path)?;
        Ok(())
    }

    fn compile(&mut self, src: SourceFile) -> Result<hir::Program> {
        let ast = Parser::parse_files(&[src])?;

        // TEMP: Create a dummy imports
        let imports = {
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
                    method_sigs: MethodSignatures::from_iterator(
                        vec![object_initialize].into_iter(),
                    ),
                    foreign: false,
                };
                skc_hir::SkClass::nonmeta(base, None)
            };

            let int_initialize = MethodSignature {
                fullname: method_fullname_raw("Int", "initialize"),
                ret_ty: ty::raw("Int"),
                params: vec![],
                typarams: vec![],
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
        };
        let defs = ast.defs();
        let type_index =
            skc_ast2hir::type_index::create(&defs, &Default::default(), &imports.sk_types);
        let _class_dict = skc_ast2hir::class_dict::create(&defs, type_index, &imports.sk_types)?;

        let mut hir = hir::untyped::create(&ast)?;
        hir.externs = prelude::lib_externs(Path::new("lib/skc_runtime/"))?
            .into_iter()
            .map(|(name, fun_ty)| hir::Extern { name, fun_ty })
            .collect();
        hir::typing::run(&mut hir)?;
        self.log(format!("# -- typing output --\n{hir}\n"));
        hir = hir_lowering::asyncness_check::run(hir);
        self.log(format!("# -- asyncness_check output --\n{hir}\n"));
        hir = hir_lowering::pass_async_env::run(hir);
        self.log(format!("# -- pass_async_env output --\n{hir}\n"));
        hir = hir_lowering::async_splitter::run(hir)?;
        self.log(format!("# -- async_splitter output --\n{hir}\n"));
        hir = hir_lowering::resolve_env_op::run(hir);
        Ok(hir)
    }

    fn log(&mut self, s: impl AsRef<str>) {
        self.log_file.write_all(s.as_ref().as_bytes()).unwrap();
    }
}
