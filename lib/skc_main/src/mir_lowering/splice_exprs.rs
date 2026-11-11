//! Splice mir::Expr::Exprs into its body.
//!
//! ## Example
//!
//! ```text
//! // Before
//! Exprs([f(), Exprs[g(), h()]]));
//! // After
//! Exprs([f(), g(), h()]);
//! ```
use crate::{gensym, mir};

pub fn run(mir: mir::Program) -> mir::Program {
    let funcs = mir.funcs.into_iter().map(compile_func).collect();
    mir::Program::new(mir.classes, mir.externs, funcs, mir.constants)
}

fn compile_func(orig_func: mir::Function) -> mir::Function {
    let mut splicer = Splicer {
        gensym: gensym::Gensym::new(gensym::PREFIX_SPLICE_EXPRS),
    };
    let new_body_stmts = splicer.splice_exprs(orig_func.body_stmts);
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

struct Splicer {
    gensym: gensym::Gensym,
}

impl Splicer {
    fn splice_exprs(&mut self, exprs: mir::TypedExpr) -> mir::TypedExpr {
        let mut new_exprs = vec![];
        for expr in mir::expr::into_exprs(exprs) {
            let new_expr = self.splice(expr, &mut new_exprs);
            new_exprs.push(new_expr);
        }
        mir::Expr::exprs(new_exprs)
    }

    fn splice(
        &mut self,
        expr: mir::TypedExpr,
        new_exprs: &mut Vec<mir::TypedExpr>,
    ) -> mir::TypedExpr {
        match expr.0 {
            mir::Expr::Number(_) => expr,
            mir::Expr::Float(_) => expr,
            mir::Expr::PseudoVar(_) => expr,
            mir::Expr::StringLiteral(_) => expr,
            mir::Expr::LVarRef(_) => expr,
            mir::Expr::IVarRef(obj_expr, idx, name) => {
                let new_obj = self.splice(*obj_expr, new_exprs);
                mir::Expr::ivar_ref(new_obj, idx, name, expr.1)
            }
            mir::Expr::ArgRef(_, _) => expr,
            mir::Expr::EnvRef(_, _) => expr,
            mir::Expr::EnvSet(idx, value_expr, name) => {
                let new_value = self.splice(*value_expr, new_exprs);
                mir::Expr::env_set(idx, new_value, name)
            }
            mir::Expr::ConstRef(_) => expr,
            mir::Expr::FuncRef(_) => expr,
            mir::Expr::FunCall(func_expr, args) => {
                let new_func = self.splice(*func_expr, new_exprs);
                let new_args = args
                    .into_iter()
                    .map(|a| self.splice(a, new_exprs))
                    .collect();
                mir::Expr::fun_call(new_func, new_args)
            }
            mir::Expr::VTableRef(receiver, idx, name) => {
                let new_receiver = self.splice(*receiver, new_exprs);
                let mir::Ty::Fun(fun_ty) = expr.1 else {
                    panic!("[BUG] VTableRef must have Fun type");
                };
                mir::Expr::vtable_ref(new_receiver, idx, name, fun_ty)
            }
            mir::Expr::GetVTable(obj_expr) => {
                let new_obj = self.splice(*obj_expr, new_exprs);
                mir::Expr::get_vtable(new_obj)
            }
            mir::Expr::WTableKey(_) => expr,
            mir::Expr::WTableRow(_, _) => expr,
            mir::Expr::WTableRef(receiver, module, idx, name) => {
                let new_receiver = self.splice(*receiver, new_exprs);
                let mir::Ty::Fun(fun_ty) = expr.1 else {
                    panic!("[BUG] WTableRef must have Fun type");
                };
                mir::Expr::wtable_ref(new_receiver, module, idx, name, fun_ty)
            }
            mir::Expr::If(cond, then, else_) => {
                let new_cond = self.splice(*cond, new_exprs);
                let new_then = self.splice_exprs(*then);
                let new_else = self.splice_exprs(*else_);
                mir::Expr::if_(new_cond, new_then, new_else)
            }
            mir::Expr::While(cond, body) => {
                // A condition's prelude must run on every iteration.
                let new_cond = self.splice_exprs(*cond);
                let new_body = self.splice_exprs(*body);
                mir::Expr::while_(new_cond, new_body)
            }
            mir::Expr::Spawn(e) => {
                let new_e = self.splice(*e, new_exprs);
                mir::Expr::spawn(new_e)
            }
            mir::Expr::Alloc(_, _) => expr,
            mir::Expr::LVarDecl(name, rhs, writable) => {
                let new_rhs = self.splice(*rhs, new_exprs);
                mir::Expr::lvar_decl(name, new_rhs, writable)
            }
            mir::Expr::LVarSet(name, rhs) => {
                let new_rhs = self.splice(*rhs, new_exprs);
                mir::Expr::lvar_set(name, new_rhs)
            }
            mir::Expr::IVarSet(obj_expr, idx, value_expr, name) => {
                let new_obj = self.splice(*obj_expr, new_exprs);
                let new_value = self.splice(*value_expr, new_exprs);
                mir::Expr::ivar_set(new_obj, idx, new_value, name)
            }
            mir::Expr::ConstSet(fullname, value_expr) => {
                let new_value = self.splice(*value_expr, new_exprs);
                mir::Expr::const_set(fullname, new_value)
            }
            mir::Expr::Return(opt_inner) => match opt_inner {
                Some(inner) => {
                    let last = self.splice(*inner, new_exprs);
                    mir::Expr::return_(last)
                }
                None => mir::Expr::return_cvoid(),
            },
            mir::Expr::Break => mir::Expr::break_(),
            mir::Expr::Exprs(inner_exprs) => {
                for ie in inner_exprs {
                    let new_ie = self.splice(ie, new_exprs);
                    new_exprs.push(new_ie);
                }
                new_exprs.pop().unwrap()
            }
            mir::Expr::Cast(cast_type, inner) => {
                let new_inner = self.splice(*inner, new_exprs);
                mir::Expr::cast(cast_type, new_inner)
            }
            mir::Expr::CreateObject(_) => expr,
            mir::Expr::CreateTypeObject(_) => expr,
            mir::Expr::SetClassObj(obj_expr, class_obj_expr) => {
                let new_obj = self.splice(*obj_expr, new_exprs);
                let new_class_obj = self.splice(*class_obj_expr, new_exprs);
                mir::Expr::set_class_obj(new_obj, new_class_obj)
            }
            mir::Expr::CreateNativeArray(elems) => {
                let new_elems = elems
                    .into_iter()
                    .map(|e| self.splice(e, new_exprs))
                    .collect();
                mir::Expr::create_native_array(new_elems)
            }
            mir::Expr::NativeArrayRef(arr, idx) => {
                let new_arr = self.splice(*arr, new_exprs);
                mir::Expr::native_array_ref(new_arr, idx, expr.1)
            }
            mir::Expr::CellNew(value) => {
                let new_value = self.splice(*value, new_exprs);
                mir::Expr::cell_new(new_value)
            }
            mir::Expr::CellGet(cell) => {
                let new_cell = self.splice(*cell, new_exprs);
                mir::Expr::cell_get(new_cell, expr.1)
            }
            mir::Expr::CellSet(cell, value) => {
                let new_cell = self.splice(*cell, new_exprs);
                let new_value = self.splice(*value, new_exprs);
                mir::Expr::cell_set(new_cell, new_value)
            }
            mir::Expr::Box(inner) => {
                let new_inner = self.splice(*inner, new_exprs);
                mir::Expr::box_(new_inner)
            }
            mir::Expr::Unbox(inner) => {
                let new_inner = self.splice(*inner, new_exprs);
                mir::Expr::unbox(new_inner)
            }
            mir::Expr::RawI64(_) => expr,
            mir::Expr::Nop => expr,
            mir::Expr::NullPtr => expr,
            mir::Expr::IsNull(inner) => {
                let new_inner = self.splice(*inner, new_exprs);
                mir::Expr::is_null(new_inner)
            }
            mir::Expr::BitIsZero(inner, bits) => {
                let new_inner = self.splice(*inner, new_exprs);
                mir::Expr::bit_is_zero(new_inner, bits)
            }
            mir::Expr::PrimBinOp(op, left, right) => {
                let mut left = self.splice(*left, new_exprs);
                let mut right_stmts = vec![];
                let right = self.splice(*right, &mut right_stmts);
                if !right_stmts.is_empty()
                    || !matches!(
                        right.0,
                        mir::Expr::Number(_)
                            | mir::Expr::RawI64(_)
                            | mir::Expr::PseudoVar(_)
                            | mir::Expr::LVarRef(_)
                            | mir::Expr::ArgRef(_, _)
                            | mir::Expr::EnvRef(_, _)
                            | mir::Expr::ConstRef(_)
                    )
                {
                    // Save even a variable read: the right operand may mutate it.
                    // Besides hoisted statements and async calls, an If nested
                    // in a sync operand can create chapters in async_splitter.
                    // Only leaves are guaranteed not to move evaluation earlier.
                    let name = self.gensym.new_name();
                    let ty = left.1.clone();
                    new_exprs.push(mir::Expr::lvar_decl(name.clone(), left, false));
                    left = mir::Expr::lvar_ref(name, ty);
                }
                new_exprs.extend(right_stmts);
                mir::Expr::prim_binop(op, left, right)
            }
            mir::Expr::ClassVTable(_) => expr,
        }
    }
}
