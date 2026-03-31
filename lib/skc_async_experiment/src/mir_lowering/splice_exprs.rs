//! Splice mir::Expr::Exprs into its body.
//!
//! ## Example
//!
//! ```
//! // Before
//! Exprs([f(), Exprs[g(), h()]]));
//! // After
//! Exprs([f(), g(), h()]);
use crate::mir;

pub fn run(mir: mir::Program) -> mir::Program {
    let funcs = mir.funcs.into_iter().map(compile_func).collect();
    mir::Program::new(mir.classes, mir.externs, funcs, mir.constants)
}

fn compile_func(orig_func: mir::Function) -> mir::Function {
    let new_body_stmts = splice_exprs(orig_func.body_stmts);
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

fn splice_exprs(exprs: mir::TypedExpr) -> mir::TypedExpr {
    let mut new_exprs = vec![];
    for expr in mir::expr::into_exprs(exprs) {
        let new_expr = splice(expr, &mut new_exprs);
        new_exprs.push(new_expr);
    }
    mir::Expr::exprs(new_exprs)
}

fn splice(expr: mir::TypedExpr, new_exprs: &mut Vec<mir::TypedExpr>) -> mir::TypedExpr {
    match expr.0 {
        mir::Expr::Number(_) => expr,
        mir::Expr::PseudoVar(_) => expr,
        mir::Expr::StringLiteral(_) => expr,
        mir::Expr::LVarRef(_) => expr,
        mir::Expr::IVarRef(obj_expr, idx, name) => {
            let new_obj = splice(*obj_expr, new_exprs);
            mir::Expr::ivar_ref(new_obj, idx, name, expr.1)
        }
        mir::Expr::ArgRef(_, _) => expr,
        mir::Expr::EnvRef(_, _) => expr,
        mir::Expr::EnvSet(idx, value_expr, name) => {
            let new_value = splice(*value_expr, new_exprs);
            mir::Expr::env_set(idx, new_value, name)
        }
        mir::Expr::ConstRef(_) => expr,
        mir::Expr::FuncRef(_) => expr,
        mir::Expr::FunCall(func_expr, args) => {
            let new_func = splice(*func_expr, new_exprs);
            let new_args = args.into_iter().map(|a| splice(a, new_exprs)).collect();
            mir::Expr::fun_call(new_func, new_args)
        }
        mir::Expr::VTableRef(receiver, idx, name) => {
            let new_receiver = splice(*receiver, new_exprs);
            let mir::Ty::Fun(fun_ty) = expr.1 else {
                panic!("[BUG] VTableRef must have Fun type");
            };
            mir::Expr::vtable_ref(new_receiver, idx, name, fun_ty)
        }
        mir::Expr::GetVTable(obj_expr) => {
            let new_obj = splice(*obj_expr, new_exprs);
            mir::Expr::get_vtable(new_obj)
        }
        mir::Expr::WTableKey(_) => expr,
        mir::Expr::WTableRow(_, _) => expr,
        mir::Expr::WTableRef(receiver, module, idx, name) => {
            let new_receiver = splice(*receiver, new_exprs);
            let mir::Ty::Fun(fun_ty) = expr.1 else {
                panic!("[BUG] WTableRef must have Fun type");
            };
            mir::Expr::wtable_ref(new_receiver, module, idx, name, fun_ty)
        }
        mir::Expr::If(cond, then, else_) => {
            let new_cond = splice(*cond, new_exprs);
            let new_then = splice_exprs(*then);
            let new_else = splice_exprs(*else_);
            mir::Expr::if_(new_cond, new_then, new_else)
        }
        mir::Expr::While(cond, body) => {
            let new_cond = splice(*cond, new_exprs);
            let new_body = splice_exprs(*body);
            mir::Expr::while_(new_cond, new_body)
        }
        mir::Expr::Spawn(e) => {
            let new_e = splice(*e, new_exprs);
            mir::Expr::spawn(new_e)
        }
        mir::Expr::Alloc(_, _) => expr,
        mir::Expr::LVarDecl(name, rhs, writable) => {
            let new_rhs = splice(*rhs, new_exprs);
            mir::Expr::lvar_decl(name, new_rhs, writable)
        }
        mir::Expr::LVarSet(name, rhs) => {
            let new_rhs = splice(*rhs, new_exprs);
            mir::Expr::lvar_set(name, new_rhs)
        }
        mir::Expr::IVarSet(obj_expr, idx, value_expr, name) => {
            let new_obj = splice(*obj_expr, new_exprs);
            let new_value = splice(*value_expr, new_exprs);
            mir::Expr::ivar_set(new_obj, idx, new_value, name)
        }
        mir::Expr::ConstSet(fullname, value_expr) => {
            let new_value = splice(*value_expr, new_exprs);
            mir::Expr::const_set(fullname, new_value)
        }
        mir::Expr::Return(opt_inner) => match opt_inner {
            Some(inner) => {
                let last = splice(*inner, new_exprs);
                mir::Expr::return_(last)
            }
            None => mir::Expr::return_cvoid(),
        },
        mir::Expr::Exprs(inner_exprs) => {
            for ie in inner_exprs {
                let new_ie = splice(ie, new_exprs);
                new_exprs.push(new_ie);
            }
            new_exprs.pop().unwrap()
        }
        mir::Expr::Cast(cast_type, inner) => {
            let new_inner = splice(*inner, new_exprs);
            mir::Expr::cast(cast_type, new_inner)
        }
        mir::Expr::CreateObject(_) => expr,
        mir::Expr::CreateTypeObject(_) => expr,
        mir::Expr::CreateNativeArray(elems) => {
            let new_elems = elems.into_iter().map(|e| splice(e, new_exprs)).collect();
            mir::Expr::create_native_array(new_elems)
        }
        mir::Expr::NativeArrayRef(arr, idx) => {
            let new_arr = splice(*arr, new_exprs);
            mir::Expr::native_array_ref(new_arr, idx, expr.1)
        }
        mir::Expr::CellNew(value) => {
            let new_value = splice(*value, new_exprs);
            mir::Expr::cell_new(new_value)
        }
        mir::Expr::CellGet(cell) => {
            let new_cell = splice(*cell, new_exprs);
            mir::Expr::cell_get(new_cell, expr.1)
        }
        mir::Expr::CellSet(cell, value) => {
            let new_cell = splice(*cell, new_exprs);
            let new_value = splice(*value, new_exprs);
            mir::Expr::cell_set(new_cell, new_value)
        }
        mir::Expr::Unbox(inner) => {
            let new_inner = splice(*inner, new_exprs);
            mir::Expr::unbox(new_inner)
        }
        mir::Expr::RawI64(_) => expr,
        mir::Expr::Nop => expr,
        mir::Expr::NullPtr => expr,
        mir::Expr::ClassVTable(_) => expr,
    }
}
