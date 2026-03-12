//! Lower VTableRef to GetVTable + NativeArrayRef
use crate::mir;
use crate::mir::rewriter::MirRewriter;
use anyhow::Result;

pub fn run(mir: mir::Program) -> mir::Program {
    let funcs = mir.funcs.into_iter().map(compile_func).collect();
    mir::Program::new(mir.classes, mir.externs, funcs, mir.constants)
}

fn compile_func(orig_func: mir::Function) -> mir::Function {
    let new_body_stmts = Lower.walk_expr(orig_func.body_stmts).unwrap();
    mir::Function {
        asyncness: orig_func.asyncness,
        name: orig_func.name,
        params: orig_func.params,
        ret_ty: orig_func.ret_ty,
        body_stmts: new_body_stmts,
        sig: orig_func.sig,
        lvar_count: orig_func.lvar_count,
    }
}

struct Lower;

impl MirRewriter for Lower {
    fn rewrite_expr(&mut self, texpr: mir::TypedExpr) -> Result<mir::TypedExpr> {
        let new_texpr = match texpr.0 {
            mir::Expr::VTableRef(receiver, idx, _debug_name) => {
                let vtable = mir::Expr::get_vtable(*receiver);
                (mir::Expr::NativeArrayRef(Box::new(vtable), idx), texpr.1)
            }
            _ => texpr,
        };
        Ok(new_texpr)
    }
}
