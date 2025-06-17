use crate::build::{loader, CompileTarget};
use crate::names::FunctionName;
use crate::{cli, codegen, hir, hir_building, hir_to_mir, mir, mir_lowering, package, prelude};
use anyhow::{Context, Result};
use shiika_core::names::type_fullname;
use shiika_core::ty::Erasure;
use shiika_parser::SourceFile;
use skc_ast2hir::class_dict::ClassDict;
use skc_hir::{MethodSignature, MethodSignatures, SkTypeBase, SkTypes, Supertype};
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

    cli.log(&format!("# -- verifier input --\n{}\n", mir.program));
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

    let hir = generate_hir(cli, &ast, target)?;
    log::info!("Creating mir");
    let mut mir = hir_to_mir::run(hir, target)?;
    cli.log(format!("# -- typing output --\n{}\n", mir.program));
    mir.program = mir_lowering::asyncness_check::run(mir.program, &mut mir.sk_types);
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
    target: &CompileTarget,
) -> Result<hir::CompilationUnit> {
    let mut imports = LibraryExports::empty();
    let mut imported_asyncs = vec![];
    for package in target.deps {
        let exp = load_exports_json(&cli.lib_exports_path(&package.spec))?;
        imports.sk_types.merge(&exp.sk_types);
        imports.constants.extend(exp.constants);
        imports.vtables.merge(exp.vtables);
        // TODO: refer .asyncness directly
        for sk_type in imports.sk_types.0.values() {
            for (sig, _) in sk_type.base().method_sigs.unordered_iter() {
                if sig.asyncness == skc_hir::Asyncness::Async {
                    imported_asyncs.push(FunctionName::from_sig(sig));
                }
            }
        }
    }

    let defs = ast.defs();
    let type_index = skc_ast2hir::type_index::create(&defs, &Default::default(), &imports.sk_types);
    let mut class_dict = skc_ast2hir::class_dict::new(type_index, &imports.sk_types);
    if target.is_core_package() {
        bootstrap_classes(&mut class_dict);
    }
    class_dict.index_program(&defs)?;
    if let Some(pkg) = target.package() {
        merge_rustlib_methods(&mut class_dict.sk_types, pkg)?;
    }

    log::debug!("Create untyped AST");
    let mut hir = hir::untyped::create(&ast, &imports.constants)?;
    log::info!("Create `new`");
    hir_building::define_new::run(&mut hir, &mut class_dict);
    log::info!("Type checking");
    let hir = hir::typing::run(hir, &class_dict, &imports.constants)?;
    let sk_types = class_dict.sk_types;
    Ok(hir::CompilationUnit {
        package: target.package().cloned(),
        imports,
        imported_asyncs,
        program: hir,
        sk_types,
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

fn merge_rustlib_methods(sk_types: &mut SkTypes, p: &package::Package) -> Result<()> {
    for exp in p.export_files() {
        for (type_name, sig_str, is_async) in package::load_exports_json5(&exp)? {
            let mut sig = parse_sig(type_name.clone(), sig_str)?;
            sig.asyncness = if is_async {
                skc_hir::Asyncness::Async
            } else {
                skc_hir::Asyncness::Sync
            };
            sk_types.define_method(&type_fullname(type_name), sig);
        }
    }
    Ok(())
}

fn parse_sig(type_name: String, sig_str: String) -> Result<MethodSignature> {
    let ast_sig = shiika_parser::Parser::parse_signature(&sig_str)?;
    Ok(hir::untyped::compile_signature(type_name, &ast_sig))
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
