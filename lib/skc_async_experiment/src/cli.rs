mod command_line_options;
use crate::{build, codegen, hir, hir_building, hir_to_mir, mir, mir_lowering, package, prelude};
use anyhow::{bail, Context, Result};
pub use command_line_options::{Command, CommandLineOptions};
use shiika_core::names::method_fullname_raw;
use shiika_core::ty::{self, Erasure};
use shiika_parser::SourceFile;
use skc_hir::{MethodSignature, MethodSignatures, SkTypeBase, Supertype};
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};

const SHIIKA_ROOT: &str = "SHIIKA_ROOT";
const SHIIKA_WORK: &str = "SHIIKA_WORK";

pub fn shiika_root() -> Result<PathBuf> {
    let Ok(shiika_root) = env::var(SHIIKA_ROOT) else {
        bail!("please set {} (where you cloned shiika repo)", SHIIKA_ROOT);
    };
    let path = PathBuf::from(shiika_root);
    if !path.exists() {
        bail!("{} does not exist", path.display());
    }
    Ok(path)
}

pub fn shiika_work() -> Result<PathBuf> {
    let Ok(shiika_work) = env::var(SHIIKA_WORK) else {
        bail!("please set {} (eg. ~/.shiika)", SHIIKA_WORK);
    };
    let path = PathBuf::from(shiika_work);
    if !path.exists() {
        bail!("{} does not exist", path.display());
    }
    Ok(path)
}

pub struct Cli {
    pub log_file: std::fs::File,
    pub shiika_work: PathBuf,
}

impl Cli {
    pub fn init() -> Result<Self> {
        Ok(Self {
            log_file: std::fs::File::create("log.milikac").unwrap(),
            shiika_work: shiika_work()?,
        })
    }

    pub fn build(&mut self, filepath: &PathBuf) -> Result<()> {
        let (dir, spec) = package::load_spec(filepath)?;
        build::cargo_builder::run(self, &dir, &spec)?;
        Ok(())
    }

    pub fn run<P: AsRef<Path>>(&mut self, filepath: P) -> Result<()> {
        let path = filepath.as_ref();
        let txt = std::fs::read_to_string(path)
            .context(format!("failed to read {}", &path.to_string_lossy()))?;
        let src = SourceFile::new(path.to_path_buf(), txt);
        let mut mir = self.compile(src)?;

        for (name, fun_ty) in prelude::core_externs() {
            mir.program.externs.push(mir::Extern { name, fun_ty });
        }
        mir.program.funcs.append(&mut prelude::funcs());

        self.log(&format!("# -- verifier input --\n{}\n", mir.program));
        mir::verifier::run(&mir.program)?;

        let bc_path = path.with_extension("bc");
        let ll_path = path.with_extension("ll");
        codegen::run(&bc_path, Some(&ll_path), mir)?;
        build::linker::run(bc_path)?;
        Ok(())
    }

    fn compile(&mut self, src: SourceFile) -> Result<mir::CompilationUnit> {
        log::info!("Creating ast");
        let ast = shiika_parser::Parser::parse_files(&[src])?;

        let hir = {
            let mut imports = create_imports();
            let imported_asyncs =
                prelude::load_lib_externs(Path::new("packages/core/"), &mut imports)?;

            let defs = ast.defs();
            let type_index =
                skc_ast2hir::type_index::create(&defs, &Default::default(), &imports.sk_types);
            let mut class_dict =
                skc_ast2hir::class_dict::create(&defs, type_index, &imports.sk_types)?;

            log::info!("Type checking");
            let mut hir = hir::untyped::create(&ast)?;
            hir_building::define_new::run(&mut hir, &mut class_dict);
            let hir = hir::typing::run(hir, &class_dict)?;
            let sk_types = class_dict.sk_types;
            hir::CompilationUnit {
                imports,
                imported_asyncs,
                program: hir,
                sk_types,
            }
        };
        log::info!("Creating mir");
        let mut mir = hir_to_mir::run(hir);
        self.log(format!("# -- typing output --\n{}\n", mir.program));
        mir.program = mir_lowering::asyncness_check::run(mir.program);
        self.log(format!("# -- asyncness_check output --\n{}\n", mir.program));
        mir.program = mir_lowering::pass_async_env::run(mir.program);
        self.log(format!("# -- pass_async_env output --\n{}\n", mir.program));
        mir.program = mir_lowering::async_splitter::run(mir.program)?;
        self.log(format!("# -- async_splitter output --\n{}\n", mir.program));
        mir.program = mir_lowering::resolve_env_op::run(mir.program);
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
    let class_bool = {
        let base = SkTypeBase {
            erasure: Erasure::nonmeta("Bool"),
            typarams: Default::default(),
            method_sigs: MethodSignatures::from_iterator(vec![].into_iter()),
            foreign: false,
        };
        skc_hir::SkClass::nonmeta(base, Some(Supertype::simple("Object")))
    };
    let class_int = {
        let base = SkTypeBase {
            erasure: Erasure::nonmeta("Int"),
            typarams: Default::default(),
            method_sigs: MethodSignatures::from_iterator(vec![].into_iter()),
            foreign: false,
        };
        skc_hir::SkClass::nonmeta(base, Some(Supertype::simple("Object")))
    };
    let class_void = {
        let base = SkTypeBase {
            erasure: Erasure::nonmeta("Void"),
            typarams: Default::default(),
            method_sigs: MethodSignatures::from_iterator(vec![].into_iter()),
            foreign: false,
        };
        skc_hir::SkClass::nonmeta(base, Some(Supertype::simple("Object")))
    };
    let class_metaclass = {
        let base = SkTypeBase {
            erasure: Erasure::nonmeta("Metaclass"),
            typarams: Default::default(),
            method_sigs: MethodSignatures::from_iterator(vec![].into_iter()),
            foreign: false,
        };
        skc_hir::SkClass::nonmeta(base, Some(Supertype::simple("Object")))
    };
    let class_class = {
        let base = SkTypeBase {
            erasure: Erasure::nonmeta("Class"),
            typarams: Default::default(),
            method_sigs: MethodSignatures::from_iterator(vec![].into_iter()),
            foreign: false,
        };
        skc_hir::SkClass::nonmeta(base, Some(Supertype::simple("Metaclass")))
    };

    let sk_types = skc_hir::SkTypes::from_iterator(
        vec![
            class_object.into(),
            class_bool.into(),
            class_int.into(),
            class_void.into(),
            class_metaclass.into(),
            class_class.into(),
        ]
        .into_iter(),
    );

    let vtables = skc_mir::VTables::build(&sk_types, &Default::default());
    skc_mir::LibraryExports {
        sk_types,
        vtables,
        constants: Default::default(),
    }
}
