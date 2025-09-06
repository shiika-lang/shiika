use crate::build::{self, loader, CompileTarget};
use crate::{cli, codegen, mir, mir_lowering, mirgen, package, prelude};
use anyhow::{Context, Result};
use shiika_core::names::type_fullname;
use shiika_core::ty::Erasure;
use shiika_parser::SourceFile;
use skc_ast2hir::class_dict::ClassDict;
use skc_hir::{MethodSignatures, SkTypeBase, Supertype};
use skc_mir::LibraryExports;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn compile(
    cli: &mut cli::Cli,
    target: &CompileTarget,
) -> Result<(PathBuf, mir::CompilationUnit)> {
    let src = loader::load(target.entry_point)?;
    let mut mir = generate_mir(cli, &src, target)?;

    if target.is_bin() {
        mir.program.funcs.append(&mut prelude::main_funcs());
    } else {
        for (name, fun_ty) in prelude::intrinsic_externs() {
            mir.program.externs.push(mir::Extern { name, fun_ty });
        }
    }
    for (name, fun_ty) in prelude::core_externs() {
        mir.program.externs.push(mir::Extern { name, fun_ty });
    }

    mir::verifier::run(&mir.program)?;

    fs::create_dir_all(target.out_dir)
        .context(format!("failed to create {}", target.out_dir.display()))?;
    let bc_path = out_path(target.out_dir, target.entry_point, "bc");
    let ll_path = out_path(target.out_dir, target.entry_point, "ll");
    codegen::run(&bc_path, Some(&ll_path), mir.clone(), target.is_bin())?;
    Ok((bc_path, mir))
}

fn out_path(out_dir: &Path, entry_point: &Path, ext: &str) -> PathBuf {
    out_dir
        .join(entry_point.file_stem().unwrap())
        .with_extension(ext)
}

fn generate_mir(
    cli: &mut cli::Cli,
    src: &[SourceFile],
    target: &CompileTarget,
) -> Result<mir::CompilationUnit> {
    log::info!("Creating ast");
    let ast = shiika_parser::Parser::parse_files(src)?;

    let uni = generate_hir(cli, ast, target)?;
    log::info!("Creating mir");

    let mut mir = mirgen::run(uni, target)?;
    cli.log(format!("# -- typing output --\n{}\n", mir.program));

    mir.program = mir_lowering::simplify_return::run(mir.program);
    cli.log(format!("# -- simplify_return output --\n{}\n", mir.program));

    mir.program = mir_lowering::asyncness_check::run(mir.program, &mut mir.sk_types);
    cli.log(format!("# -- asyncness_check output --\n{}\n", mir.program));

    mir.program = mir_lowering::pass_async_env::run(mir.program);
    cli.log(format!("# -- pass_async_env output --\n{}\n", mir.program));

    mir.program = mir_lowering::splice_exprs::run(mir.program);
    cli.log(format!("# -- splice_exprs output --\n{}\n", mir.program));

    mir.program = mir_lowering::async_splitter::run(mir.program)?;
    cli.log(format!("# -- async_splitter output --\n{}\n", mir.program));

    mir.program = mir_lowering::resolve_env_op::run(mir.program);
    cli.log(format!("# -- resolve_env_op output --\n{}\n", mir.program));

    Ok(mir)
}

fn generate_hir(
    cli: &mut cli::Cli,
    ast: shiika_ast::Program,
    target: &CompileTarget,
) -> Result<build::CompilationUnit> {
    let imports = {
        let mut imports = LibraryExports::empty();
        for package in target.deps {
            let exp = load_exports_json(&cli.lib_exports_path(&package.spec))?;
            imports.sk_types.merge(&exp.sk_types);
            imports.constants.extend(exp.constants);
            imports.vtables.merge(exp.vtables);
        }
        imports
    };

    let class_dict = {
        let defs = ast.defs();
        let type_index =
            skc_ast2hir::type_index::create(&defs, &Default::default(), &imports.sk_types);
        let mut class_dict = skc_ast2hir::class_dict::new(type_index, &imports.sk_types);
        if target.is_core_package() {
            bootstrap_classes(&mut class_dict);
        }
        class_dict.index_program(&defs)?;
        if let Some(pkg) = target.package() {
            merge_rustlib_methods(&mut class_dict, pkg)?;
        }
        class_dict
    };

    let hir = {
        let mut hir_maker = skc_ast2hir::hir_maker::HirMaker::new(class_dict, &imports.constants);
        hir_maker.define_class_constants()?;
        let (main_exprs, main_lvars) = hir_maker.convert_toplevel_items(ast.toplevel_items)?;
        hir_maker.extract_hir(main_exprs, main_lvars)
    };

    Ok(build::CompilationUnit {
        package_name: target.package_name(),
        imports,
        hir,
    })
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

/// Insert signatures in exports.json5 (methods written in Rust) into SkTypes.
fn merge_rustlib_methods(class_dict: &mut ClassDict, p: &package::Package) -> Result<()> {
    for exp in p.export_files() {
        for (type_name, sig_str, is_async) in package::load_exports_json5(&exp)? {
            let sk_type = class_dict
                .sk_types
                .get_type(&type_fullname(&type_name))
                .ok_or_else(|| anyhow::anyhow!("Type {} not found", type_name))?;
            let (inheritable, superclass) = if let skc_hir::SkType::Class(sk_class) = sk_type {
                (
                    sk_class.is_final == Some(false),
                    sk_class.superclass.clone(),
                )
            } else {
                (false, None)
            };
            let ast_sig = shiika_parser::Parser::parse_signature(&sig_str)?;

            let mut sig = class_dict.create_maybe_virtual_signature(
                inheritable,
                &sk_type.base().erasure.namespace(),
                sk_type.fullname(),
                &ast_sig,
                &sk_type.base().typarams,
                &superclass,
            )?;
            sig.asyncness = if is_async {
                skc_hir::Asyncness::Async
            } else {
                skc_hir::Asyncness::Sync
            };
            class_dict
                .sk_types
                .rustlib_methods
                .push(sig.fullname.clone());
            class_dict
                .sk_types
                .define_method(&type_fullname(type_name), sig);
        }
    }
    Ok(())
}

fn bootstrap_classes(class_dict: &mut ClassDict) {
    // Add `Object` (the only class without superclass)
    class_dict.add_type(skc_hir::SkClass::nonmeta(
        SkTypeBase {
            erasure: Erasure::nonmeta("Object"),
            typarams: Default::default(),
            method_sigs: MethodSignatures::new(),
            foreign: false,
        },
        None,
    ));
    class_dict.add_type(skc_hir::SkClass::meta(SkTypeBase {
        erasure: Erasure::meta("Object"),
        typarams: Default::default(),
        method_sigs: MethodSignatures::new(),
        foreign: false,
    }));

    // Add `Class`
    class_dict.add_type(skc_hir::SkClass::nonmeta(
        SkTypeBase {
            erasure: Erasure::nonmeta("Class"),
            typarams: Default::default(),
            method_sigs: MethodSignatures::new(),
            foreign: false,
        },
        Some(Supertype::simple("Object")),
    ));
    class_dict.add_type(skc_hir::SkClass::meta(SkTypeBase {
        erasure: Erasure::meta("Class"),
        typarams: Default::default(),
        method_sigs: MethodSignatures::new(),
        foreign: false,
    }));

    // Add `Void` (the only non-enum class whose const_is_obj=true)
    let mut void = skc_hir::SkClass::nonmeta(
        SkTypeBase {
            erasure: Erasure::nonmeta("Void"),
            typarams: Default::default(),
            method_sigs: MethodSignatures::new(),
            foreign: false,
        },
        Some(Supertype::simple("Object")),
    );
    void.const_is_obj = true;
    class_dict.add_type(void);
    class_dict.add_type(skc_hir::SkClass::meta(SkTypeBase {
        erasure: Erasure::meta("Void"),
        typarams: Default::default(),
        method_sigs: MethodSignatures::new(),
        foreign: false,
    }));
}
