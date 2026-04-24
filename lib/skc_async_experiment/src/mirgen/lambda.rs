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
    let func_ptr =
        mir::Expr::ivar_ref(fn_obj.clone(), 0, "@func".to_string(), lambda_fun_ty.into());
    let mut all_args = vec![fn_obj];
    all_args.extend(mir_args);
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
pub fn build_fn_object(
    func_name: FunctionName,
    params: &[MethodParam],
    capture_values: Vec<mir::TypedExpr>,
    ret_ty: TermTy,
    fn_ty: TermTy,
) -> mir::TypedExpr {
    let captures_ptr = mir::Expr::create_native_array(capture_values);

    // Build function type for the lambda (fn_obj + explicit params)
    let mut lambda_param_tys: Vec<mir::Ty> = vec![fn_ty.clone().into()];
    lambda_param_tys.extend(params.iter().map(|p| p.ty.clone().into()));
    let lambda_fun_ty = mir::FunTy::new(mir::Asyncness::Async, lambda_param_tys, ret_ty.into());

    // Call Meta:FnN#new(Meta:FnN, Ptr, Ptr, Int) -> FnN
    let meta_fn_ty = fn_ty.meta_ty();
    let meta_erasure = meta_fn_ty.erasure();
    let new_func_ref = {
        let new_func_name: FunctionName =
            MethodFullname::new(meta_erasure.to_type_fullname(), "new").into();
        let new_fun_ty = mir::FunTy::new(
            mir::Asyncness::Unknown,
            vec![
                meta_fn_ty.clone().into(),
                mir::Ty::Ptr,
                mir::Ty::Ptr,
                mir::Ty::raw("Int"),
            ],
            fn_ty.clone().into(),
        );
        mir::Expr::func_ref(new_func_name, new_fun_ty)
    };
    // Use base_type_name() for const ref too (e.g., "Fn1" instead of "Fn1<Int,Int>")
    let meta_fn_obj = mir::Expr::const_ref(meta_erasure.to_const_fullname(), meta_fn_ty.into());

    let func_ptr = mir::Expr::func_ref(func_name, lambda_fun_ty);
    // Cast to Ptr for storing in Fn object's @func ivar
    let func_ptr_as_ptr = mir::Expr::cast(mir::CastType::Force(mir::Ty::Ptr), func_ptr);

    let zero = mir::Expr::number(0);
    mir::Expr::fun_call(
        new_func_ref,
        vec![meta_fn_obj, func_ptr_as_ptr, captures_ptr, zero],
    )
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
pub fn get_capture_value(cap: &HirLambdaCapture) -> mir::TypedExpr {
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
        _ => todo!("Unsupported capture type: {:?}", cap.detail),
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
