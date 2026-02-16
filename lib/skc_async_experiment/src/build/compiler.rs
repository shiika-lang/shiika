use crate::build::{self, bootstrap_classes, loader, CompileTarget};
use crate::codegen::prelude;
use crate::{cli, codegen, mir, mir_lowering, mirgen, package};
use anyhow::{Context, Result};
use shiika_core::names::type_fullname;
use shiika_parser::SourceFile;
use skc_ast2hir::class_dict::RustMethods;
use skc_mir::LibraryExports;
use std::collections::HashMap;
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

    mir::verifier::run(&mir)?;

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
    cli.write_debug_log("01-mirgen.mirdump", &mir.program);

    mir.program = mir_lowering::asyncness_check::run(mir.program, &mut mir.sk_types);
    cli.write_debug_log("02-asyncness_check.mirdump", &mir.program);

    mir.program = mir_lowering::let_bind_async::run(mir.program);
    cli.write_debug_log("03-let_bind_async.mirdump", &mir.program);

    mir.program = mir_lowering::pass_async_env::run(mir.program);
    cli.write_debug_log("04-pass_async_env.mirdump", &mir.program);

    mir.program = mir_lowering::splice_exprs::run(mir.program);
    cli.write_debug_log("05-splice_exprs.mirdump", &mir.program);

    mir.program = mir_lowering::async_splitter::run(mir.program)?;
    cli.write_debug_log("06-async_splitter.mirdump", &mir.program);

    mir.program = mir_lowering::resolve_env_op::run(mir.program);
    cli.write_debug_log("07-resolve_env_op.mirdump", &mir.program);

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
            bootstrap_classes::add_to(&mut class_dict);
        }
        let rustlib_methods = if let Some(pkg) = target.package() {
            list_rustlib_methods(pkg)?
        } else {
            Default::default()
        };
        class_dict.index_program(&defs, rustlib_methods)?;
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

fn list_rustlib_methods(p: &package::Package) -> Result<RustMethods> {
    let mut methods = HashMap::new();
    for exp in p.export_files() {
        for (type_name, sig_str, is_async) in package::load_exports_json5(&exp)? {
            let tname = type_fullname(&type_name);
            let sig = shiika_parser::Parser::parse_signature(&sig_str)?;
            if !methods.contains_key(&tname) {
                methods.insert(tname.clone(), vec![]);
            }
            methods.get_mut(&tname).unwrap().push((sig, is_async));
        }
    }
    Ok(methods)
}
