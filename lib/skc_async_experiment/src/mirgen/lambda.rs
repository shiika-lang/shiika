use crate::mir;
use crate::names::FunctionName;
use shiika_core::names::MethodFullname;
use shiika_core::ty::TermTy;
use skc_hir::visitor::{walk_expr, HirVisitor};
use skc_hir::{HirExpression, HirExpressionBase, HirLVars, MethodParam};
use skc_hir::{HirLambdaCapture, HirLambdaCaptureDetail};
use std::collections::HashSet;

/// Lambda-related state held by the compiler
pub struct LambdaContext {
    /// Collects generated lambda functions
    pub lambda_funcs: Vec<mir::Function>,
    /// Names of local variables that need Cell wrapping (captured as non-readonly)
    pub cell_vars: HashSet<String>,
    /// The Fn class name for the current lambda being compiled (e.g. "Fn1")
    pub current_fn_class: Option<String>,
}

impl LambdaContext {
    pub fn new() -> Self {
        Self {
            lambda_funcs: vec![],
            cell_vars: HashSet::new(),
            current_fn_class: None,
        }
    }
}

/// Build a lambda invocation from pre-converted MIR expressions.
/// `mir_args` should NOT include fn_obj; it will be prepended automatically.
pub fn compile_lambda_invocation(
    lambda_ty: &TermTy,
    fn_obj: mir::TypedExpr,
    mir_args: Vec<mir::TypedExpr>,
) -> mir::TypedExpr {
    let lambda_fun_ty = mir::FunTy::lambda_fun(lambda_ty);
    // param_tys[0] is the fn_obj itself; the rest are the explicit params.
    let expected_param_tys: Vec<mir::Ty> = lambda_fun_ty.param_tys[1..].to_vec();
    let func_ptr =
        mir::Expr::ivar_ref(fn_obj.clone(), 0, "@func".to_string(), lambda_fun_ty.into());
    let mut all_args = vec![fn_obj];
    for (arg, expected_ty) in mir_args.into_iter().zip(expected_param_tys.iter()) {
        let casted = if arg.1.same(expected_ty) {
            arg
        } else {
            mir::Expr::cast(mir::CastType::Upcast(expected_ty.clone()), arg)
        };
        all_args.push(casted);
    }
    mir::Expr::fun_call(func_ptr, all_args)
}

/// Build the lambda `mir::Function` from a pre-converted body.
pub fn build_lambda_function(
    name: &str,
    params: &[MethodParam],
    body: mir::TypedExpr,
    lvars: &HirLVars,
    ret_ty: &TermTy,
    fn_class: &str,
) -> mir::Function {
    // First param is the Fn object
    let mut mir_params: Vec<mir::Param> = vec![mir::Param::new(mir::Ty::raw(fn_class), "$fn")];
    // Then explicit params
    mir_params.extend(params.iter().map(|p| mir::Param {
        ty: p.ty.clone().into(),
        name: p.name.clone(),
    }));

    // Alloc stmts for local variables
    let mut stmts: Vec<mir::TypedExpr> = lvars
        .iter()
        .map(|lvar| mir::Expr::alloc(lvar.name.clone(), lvar.ty.clone().into()))
        .collect();

    let body_with_return = if ret_ty.is_void_type() {
        mir::Expr::exprs(vec![body, mir::Expr::return_(mir::Expr::void_const_ref())])
    } else {
        mir::Expr::return_(body)
    };

    stmts.extend(mir::expr::into_exprs(body_with_return));
    let final_body = mir::Expr::exprs(stmts);

    mir::Function {
        // All lambdas are treated as async now
        asyncness: mir::Asyncness::Async,
        name: FunctionName::Generated(name.to_string()),
        params: mir_params,
        ret_ty: ret_ty.clone().into(),
        body_stmts: final_body,
        sig: None,
        lvar_count: None,
    }
}

/// Build the `Meta:FnN#new(...)` call that creates the Fn object.
/// `capture_values` are the already-evaluated capture expressions.
/// `tyarg_class_objs` are runtime `Class` objects for each type argument of `fn_ty`,
/// pre-computed by the caller (so it can resolve method/class typaram refs using
/// surrounding-method context).
pub fn build_fn_object(
    func_name: FunctionName,
    params: &[MethodParam],
    capture_values: Vec<mir::TypedExpr>,
    ret_ty: TermTy,
    fn_ty: TermTy,
    tyarg_class_objs: Vec<mir::TypedExpr>,
) -> mir::TypedExpr {
    let captures_ptr = mir::Expr::create_native_array(capture_values);

    // Build function type for the lambda (fn_obj + explicit params)
    let mut lambda_param_tys: Vec<mir::Ty> = vec![fn_ty.clone().into()];
    lambda_param_tys.extend(params.iter().map(|p| p.ty.clone().into()));
    let lambda_fun_ty = mir::FunTy::new(mir::Asyncness::Async, lambda_param_tys, ret_ty.into());

    // Call Meta:FnN#new(Meta:FnN, Ptr, Ptr, Int, [Class...]) -> FnN
    //
    // Per `signature_of_new`, `Meta:FnN#new` carries the class typarams of FnN
    // (e.g. `A1, R` for `Fn1<A1, R>`) as method typarams, and the mirgen pass
    // for method calls passes them as additional `Class` arguments after the
    // explicit ones. We must do the same here.
    let meta_fn_ty = fn_ty.meta_ty();
    let meta_erasure = meta_fn_ty.erasure();
    let new_func_ref = {
        let new_func_name: FunctionName =
            MethodFullname::new(meta_erasure.to_type_fullname(), "new").into();
        let mut param_tys = vec![
            meta_fn_ty.clone().into(),
            mir::Ty::Ptr,
            mir::Ty::Ptr,
            mir::Ty::raw("Int"),
        ];
        for _ in &tyarg_class_objs {
            param_tys.push(mir::Ty::raw("Class"));
        }
        let new_fun_ty = mir::FunTy::new(mir::Asyncness::Unknown, param_tys, fn_ty.clone().into());
        mir::Expr::func_ref(new_func_name, new_fun_ty)
    };
    // Use base_type_name() for const ref too (e.g., "Fn1" instead of "Fn1<Int,Int>")
    let meta_fn_obj = mir::Expr::const_ref(meta_erasure.to_const_fullname(), meta_fn_ty.into());

    let func_ptr = mir::Expr::func_ref(func_name, lambda_fun_ty);
    // Cast to Ptr for storing in Fn object's @func ivar
    let func_ptr_as_ptr = mir::Expr::cast(mir::CastType::Force(mir::Ty::Ptr), func_ptr);

    let zero = mir::Expr::number(0);
    let mut args = vec![meta_fn_obj, func_ptr_as_ptr, captures_ptr, zero];
    args.extend(tyarg_class_objs);
    mir::Expr::fun_call(new_func_ref, args)
}

/// Build the expression that reads a captured variable inside a lambda body.
pub fn compile_lambda_capture_ref(
    current_fn_class: &Option<String>,
    idx: usize,
    readonly: bool,
    ty: TermTy,
) -> mir::TypedExpr {
    // $fn is first arg (index 0)
    let fn_class = current_fn_class
        .as_ref()
        .expect("[BUG] HirLambdaCaptureRef outside lambda");
    let fn_obj = mir::Expr::arg_ref(0, "$fn", mir::Ty::raw(fn_class));
    // @captures is ivar index 1
    let captures = mir::Expr::ivar_ref(fn_obj, 1, "@captures".to_string(), mir::Ty::Ptr);

    if !readonly {
        // var capture: read through cell
        let cell = mir::Expr::native_array_ref(captures, idx, mir::Ty::Ptr);
        mir::Expr::cell_get(cell, ty.into())
    } else {
        // let capture: direct value
        mir::Expr::native_array_ref(captures, idx, ty.into())
    }
}

/// Build the expression that writes a captured variable inside a lambda body.
/// `value` is the already-converted RHS expression.
pub fn compile_lambda_capture_write(
    fn_class: &str,
    cidx: usize,
    value: mir::TypedExpr,
    result_ty: mir::Ty,
) -> mir::TypedExpr {
    // $fn is first arg (index 0)
    let fn_obj = mir::Expr::arg_ref(0, "$fn", mir::Ty::raw(fn_class));
    // @captures is ivar index 1
    let captures = mir::Expr::ivar_ref(fn_obj, 1, "@captures".to_string(), mir::Ty::Ptr);
    let cell = mir::Expr::native_array_ref(captures, cidx, mir::Ty::Ptr);
    // cell_set returns Void, but assignment in Shiika returns the value.
    // Re-read the cell to produce the assigned value.
    let fn_obj2 = mir::Expr::arg_ref(0, "$fn", mir::Ty::raw(fn_class));
    let captures2 = mir::Expr::ivar_ref(fn_obj2, 1, "@captures".to_string(), mir::Ty::Ptr);
    let cell2 = mir::Expr::native_array_ref(captures2, cidx, mir::Ty::Ptr);
    mir::Expr::exprs(vec![
        mir::Expr::cell_set(cell, value),
        mir::Expr::cell_get(cell2, result_ty),
    ])
}

/// Build the expression that evaluates to the captured value at the call site.
/// `current_fn_class` is the enclosing lambda's Fn class (if any) — needed
/// for `CaptureFwd`, which forwards from the enclosing lambda's `@captures`.
pub fn get_capture_value(
    cap: &HirLambdaCapture,
    current_fn_class: &Option<String>,
) -> mir::TypedExpr {
    match &cap.detail {
        HirLambdaCaptureDetail::CaptureLVar { name } => {
            if !cap.readonly {
                // var lvar: pass cell pointer (lvar already holds a cell due to cell_vars)
                mir::Expr::lvar_ref(name.clone(), mir::Ty::Ptr)
            } else {
                // let lvar: pass value directly
                mir::Expr::lvar_ref(name.clone(), cap.ty.clone().into())
            }
        }
        HirLambdaCaptureDetail::CaptureArg { idx } => {
            // Args are captured by value (+1 for self receiver)
            mir::Expr::arg_ref(idx + 1, "captured_arg", cap.ty.clone().into())
        }
        HirLambdaCaptureDetail::CaptureMethodTyArg { idx, n_params } => {
            // Method tyargs are passed as additional Class args after self
            // and explicit params, matching HirMethodTVarRef in mirgen.rs.
            mir::Expr::arg_ref(
                1 + n_params + idx,
                "captured_method_tyarg",
                mir::Ty::raw("Class"),
            )
        }
        HirLambdaCaptureDetail::CaptureSelf => {
            // The enclosing scope's `self` is `arg_ref(0)` there.
            mir::Expr::arg_ref(0, "self", cap.ty.clone().into())
        }
        HirLambdaCaptureDetail::CaptureFwd { cidx } => {
            // Forward from the enclosing lambda's @captures: the value is
            // already stored there in the right form (cell-Ptr for var,
            // value for let), so pass it through without dereferencing.
            let fn_class = current_fn_class
                .as_ref()
                .expect("[BUG] CaptureFwd outside lambda");
            let fn_obj = mir::Expr::arg_ref(0, "$fn", mir::Ty::raw(fn_class));
            let captures = mir::Expr::ivar_ref(fn_obj, 1, "@captures".to_string(), mir::Ty::Ptr);
            let elem_ty: mir::Ty = if cap.readonly {
                cap.ty.clone().into()
            } else {
                mir::Ty::Ptr
            };
            mir::Expr::native_array_ref(captures, *cidx, elem_ty)
        }
    }
}

/// Scan HIR expressions for HirLambdaExpr nodes and collect
/// lvar names that need Cell wrapping (captured && writable)
pub fn collect_cell_vars(expr: &HirExpression) -> HashSet<String> {
    struct CellVarCollector(HashSet<String>);
    impl<'hir> HirVisitor<'hir> for CellVarCollector {
        fn visit_expr(&mut self, expr: &'hir HirExpression) -> anyhow::Result<()> {
            if let HirExpressionBase::HirLambdaExpr { captures, .. } = &expr.node {
                for cap in captures {
                    if !cap.readonly {
                        if let HirLambdaCaptureDetail::CaptureLVar { name } = &cap.detail {
                            self.0.insert(name.clone());
                        }
                    }
                }
            }
            Ok(())
        }
    }
    let mut collector = CellVarCollector(HashSet::new());
    walk_expr(&mut collector, expr).unwrap();
    collector.0
}
