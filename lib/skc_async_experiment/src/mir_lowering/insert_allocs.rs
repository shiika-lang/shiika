//! Insert `Alloc` statements at the beginning of each function body.
//!
//! This pass collects all `LVarDecl` variables and inserts `Alloc` statements
//! at the function entry so stack space is allocated before the variables are used.
//!
//! Intended to be run after async_splitter.
use crate::mir;

pub fn run(mir: mir::Program) -> mir::Program {
    let funcs = mir.funcs.into_iter().map(compile_func).collect();
    mir::Program::new(mir.classes, mir.externs, funcs, mir.constants)
}

fn compile_func(orig_func: mir::Function) -> mir::Function {
    let lvars = mir::visitor::LVarDecls::collect(&orig_func.body_stmts);
    let mut new_body = vec![];
    for (name, ty) in lvars {
        new_body.push(mir::Expr::alloc(name, ty));
    }

    // Unwrap original body and append to allocs
    if let (mir::Expr::Exprs(exprs), _) = orig_func.body_stmts {
        new_body.extend(exprs);
    } else {
        new_body.push(orig_func.body_stmts);
    }

    mir::Function {
        body_stmts: mir::Expr::exprs(new_body),
        ..orig_func
    }
}
