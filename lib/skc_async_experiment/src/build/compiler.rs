use crate::build::loader;
use crate::names::FunctionName;
use crate::{cli, codegen, hir, hir_building, hir_to_mir, mir, mir_lowering, package, prelude};
use anyhow::{Context, Result};
use shiika_core::names::method_fullname_raw;
use shiika_core::names::type_fullname;
use shiika_core::ty::{self, Erasure};
use shiika_parser::SourceFile;
use skc_hir::{MethodSignature, MethodSignatures, SkTypeBase, Supertype};
use skc_mir::LibraryExports;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub fn compile(
    cli: &mut cli::Cli,
    package: Option<&package::Package>,
    entry_point: &Path,
    out_dir: &Path,
    deps: &[package::Package],
    is_bin: bool,
) -> Result<PathBuf> {
    let package_name = package.map(|p| p.spec.name.clone());
    let src = loader::load(entry_point)?;
    let mut mir = generate_mir(cli, &src, package_name, &deps, is_bin)?;

    if is_bin {
        mir.program.funcs.append(&mut prelude::main_funcs());
    } else {
        for (name, fun_ty) in prelude::intrinsic_externs() {
            mir.program.externs.push(mir::Extern { name, fun_ty });
        }
    }
    for (name, fun_ty) in prelude::core_externs() {
        mir.program.externs.push(mir::Extern { name, fun_ty });
    }

    cli.log(&format!("# -- verifier input --\n{}\n", mir.program));
    mir::verifier::run(&mir.program)?;

    fs::create_dir_all(out_dir).context(format!("failed to create {}", out_dir.display()))?;
    if !is_bin {
        let mut constants = HashMap::new();
        for (name, ty) in &mir.program.constants {
            constants.insert(name.clone(), ty.clone());
        }
        let exports = LibraryExports {
            sk_types: mir.sk_types.clone(),
            vtables: mir.vtables.clone(),
            constants,
        };
        let out_path = cli.lib_exports_path(&package.unwrap().spec);
        write_exports_json(&out_path, &exports)?;
    }
    let bc_path = out_path(out_dir, entry_point, "bc");
    let ll_path = out_path(out_dir, entry_point, "ll");
    codegen::run(&bc_path, Some(&ll_path), mir, is_bin)?;
    Ok(bc_path)
}

fn out_path(out_dir: &Path, entry_point: &Path, ext: &str) -> PathBuf {
    out_dir
        .join(entry_point.file_stem().unwrap())
        .with_extension(ext)
}

fn generate_mir(
    cli: &mut cli::Cli,
    src: &[SourceFile],
    package_name: Option<String>,
    deps: &[package::Package],
    is_bin: bool,
) -> Result<mir::CompilationUnit> {
    log::info!("Creating ast");
    let ast = shiika_parser::Parser::parse_files(src)?;

    let hir = generate_hir(cli, &ast, package_name, deps)?;
    log::info!("Creating mir");
    let mut mir = hir_to_mir::run(hir, is_bin)?;
    cli.log(format!("# -- typing output --\n{}\n", mir.program));
    mir.program = mir_lowering::asyncness_check::run(mir.program);
    cli.log(format!("# -- asyncness_check output --\n{}\n", mir.program));
    mir.program = mir_lowering::pass_async_env::run(mir.program);
    cli.log(format!("# -- pass_async_env output --\n{}\n", mir.program));
    mir.program = mir_lowering::async_splitter::run(mir.program)?;
    cli.log(format!("# -- async_splitter output --\n{}\n", mir.program));
    mir.program = mir_lowering::resolve_env_op::run(mir.program);
    Ok(mir)
}

fn generate_hir(
    cli: &mut cli::Cli,
    ast: &shiika_ast::Program,
    package_name: Option<String>,
    deps: &[package::Package],
) -> Result<hir::CompilationUnit> {
    let mut imports = create_imports();
    let mut imported_asyncs = vec![];
    for package in deps {
        for exp in package.export_files() {
            imported_asyncs.append(&mut load_externs(&exp, &mut imports)?);
        }
        let exp = load_exports_json(&cli.lib_exports_path(&package.spec))?;
        imports.sk_types.merge(&exp.sk_types);
        imports.constants.extend(exp.constants);
    }

    let defs = ast.defs();
    let type_index = skc_ast2hir::type_index::create(&defs, &Default::default(), &imports.sk_types);
    let mut class_dict = skc_ast2hir::class_dict::create(&defs, type_index, &imports.sk_types)?;

    log::info!("Type checking");
    let mut hir = hir::untyped::create(&ast, &imports.constants)?;
    hir_building::define_new::run(&mut hir, &mut class_dict);
    let hir = hir::typing::run(hir, &class_dict, &imports.constants)?;
    let sk_types = class_dict.sk_types;
    Ok(hir::CompilationUnit {
        package_name,
        imports,
        imported_asyncs,
        program: hir,
        sk_types,
    })
}

/// Load exports.json5 (i.e. Rust func definitions)
/// Also returns hir::FunTy because type checker needs it
pub fn load_externs(
    exports_json5: &PathBuf,
    imports: &mut LibraryExports,
) -> Result<Vec<FunctionName>> {
    let mut imported_asyncs = vec![];
    for (type_name, sig_str, is_async) in package::load_exports_json5(exports_json5)? {
        let sig = parse_sig(type_name.clone(), sig_str)?;
        if is_async {
            imported_asyncs.push(FunctionName::from_sig(&sig));
        }
        imports
            .sk_types
            .define_method(&type_fullname(type_name), sig);
    }
    Ok(imported_asyncs)
}

fn parse_sig(type_name: String, sig_str: String) -> Result<MethodSignature> {
    let ast_sig = shiika_parser::Parser::parse_signature(&sig_str)?;
    Ok(hir::untyped::compile_signature(type_name, &ast_sig))
}

/// Serialize LibraryExports into exports.json
pub fn write_exports_json(
    out_path: &std::path::Path,
    exports: &skc_mir::LibraryExports,
) -> Result<()> {
    let json = serde_json::to_string_pretty(exports)?;
    let mut f = std::fs::File::create(out_path)?;
    f.write_all(json.as_bytes())?;
    Ok(())
}

/// Deserialize exports.json into LibraryExports
fn load_exports_json(path: &Path) -> Result<LibraryExports> {
    let mut f = fs::File::open(&path).context(format!("{} not found", path.display()))?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .context(format!("failed to read {}", path.display()))?;
    let exports: LibraryExports =
        serde_json::from_str(&contents).context(format!("failed to parse {}", path.display()))?;
    Ok(exports)
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
