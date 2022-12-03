use crate::class_dict::FoundMethod;
use crate::convert_exprs::{block, block::BlockTaker};
use crate::error;
use crate::hir_maker::HirMaker;
use crate::type_inference::method_call_inf;
use crate::type_system::type_checking;
use anyhow::{Context, Result};
use shiika_ast::{AstExpression, LocationSpan};
use shiika_core::{names::MethodFirstname, ty, ty::TermTy};
use skc_hir::*;

pub fn convert_method_call(
    mk: &mut HirMaker,
    receiver_expr: &Option<Box<AstExpression>>,
    method_name: &MethodFirstname,
    arg_exprs: &[AstExpression],
    has_block: &bool,
    type_args: &[AstExpression],
    locs: &LocationSpan,
) -> Result<HirExpression> {
    // Check if this is a lambda invocation
    if receiver_expr.is_none() {
        if let Some(lvar) = mk._lookup_var(&method_name.0, locs.clone()) {
            if lvar.ty.fn_x_info().is_some() {
                return convert_lambda_invocation(mk, lvar.ref_expr(), arg_exprs, has_block, locs);
            }
        }
    }

    let receiver_hir = match receiver_expr {
        Some(expr) => mk.convert_expr(expr)?,
        // Implicit self
        _ => mk.convert_self_expr(&LocationSpan::todo()),
    };

    let mut method_tyargs = vec![];
    for tyarg in type_args {
        method_tyargs.push(resolve_method_tyarg(mk, tyarg)?);
    }

    let found = mk
        .class_dict
        .lookup_method(&receiver_hir.ty, method_name, method_tyargs.as_slice())?
        .clone();
    if type_args.len() > 0 && type_args.len() != found.sig.typarams.len() {
        return Err(error::type_error(format!(
            "wrong number of method-wise type arguments ({} for {:?}",
            type_args.len(),
            &found.sig,
        )));
    }

    let inf1 = if found.sig.typarams.len() > 0 && type_args.is_empty() {
        let sig = &found.sig; //.specialize(class_tyargs, method_tyargs);
        Some(method_call_inf::MethodCallInf1::new(sig, *has_block))
    } else if *has_block {
        type_checking::check_takes_block(&found.sig, locs)?;
        Some(method_call_inf::MethodCallInf1::infer_block(&found.sig))
    } else {
        None
    };
    let msg = format!("Type inferrence failed: {:?}", inf1);
    let (arg_hirs, inf3) = convert_method_args(
        mk,
        inf1,
        &BlockTaker::Method {
            sig: found.sig.clone(),
            locs,
        },
        arg_exprs,
        has_block,
    )
    .context(msg)?;
    build(mk, found, receiver_hir, arg_hirs, inf3)
}

/// Returns `Some` if the method call is a lambda invocation.
pub fn convert_lambda_invocation(
    mk: &mut HirMaker,
    fn_expr: HirExpression,
    arg_exprs: &[AstExpression],
    has_block: &bool,
    locs: &LocationSpan,
) -> Result<HirExpression> {
    let (arg_hirs, _) = convert_method_args(
        mk,
        None,
        &BlockTaker::Function {
            fn_ty: &fn_expr.ty,
            locs,
        },
        arg_exprs,
        has_block, // true if `f(){ ... }`, for example.
    )?;
    let ret_ty = fn_expr.ty.fn_x_info().unwrap().last().unwrap().clone();
    Ok(Hir::lambda_invocation(
        ret_ty,
        fn_expr,
        arg_hirs,
        locs.clone(),
    ))
}

/// Resolve a method tyarg (a ConstName) into a TermTy
/// eg.
///     ary.map<Array<T>>(f)
///             ~~~~~~~~
///             => TermTy(Array<TyParamRef(T)>)
fn resolve_method_tyarg(mk: &mut HirMaker, arg: &AstExpression) -> Result<TermTy> {
    let e = mk.convert_expr(arg)?;
    mk.assert_class_expr(&e)?;
    Ok(e.ty.instance_ty())
}

/// Convert method call arguments to HirExpression's
/// Also returns inferred type of this method call.
fn convert_method_args(
    mk: &mut HirMaker,
    inf: Option<method_call_inf::MethodCallInf1>,
    block_taker: &BlockTaker,
    arg_exprs: &[AstExpression],
    has_block: &bool,
) -> Result<(Vec<HirExpression>, Option<method_call_inf::MethodCallInf3>)> {
    let n = arg_exprs.len();
    let mut arg_hirs = vec![];
    if *has_block && inf.is_some() {
        if n > 1 {
            for i in 0..n - 1 {
                arg_hirs.push(mk.convert_expr(&arg_exprs[i])?);
            }
        }
        let last_arg = &arg_exprs.last().unwrap();

        let arg_tys = arg_hirs.iter().map(|x| &x.ty).collect::<Vec<_>>();
        let inf2 = method_call_inf::infer_block_param(inf.unwrap(), &arg_tys)?;
        let block_hir = block::convert_block(mk, block_taker, &inf2, &last_arg)?;
        let inf3 = method_call_inf::infer_result_ty_with_block(inf2, &block_hir.ty)?;

        arg_hirs.push(block_hir);
        Ok((arg_hirs, Some(inf3)))
    } else {
        for expr in arg_exprs {
            arg_hirs.push(mk.convert_expr(&expr)?);
        }
        Ok((arg_hirs, None))
    }
}

/// For method calls without any arguments.
pub fn build_simple(
    mk: &HirMaker,
    found: FoundMethod,
    receiver_hir: HirExpression,
) -> Result<HirExpression> {
    build(
        mk,
        found,
        receiver_hir,
        Default::default(),
        Default::default(),
    )
}

/// Check the arguments and create HirMethodCall or HirModuleMethodCall
pub fn build(
    mk: &HirMaker,
    found: FoundMethod,
    receiver_hir: HirExpression,
    mut arg_hirs: Vec<HirExpression>,
    inf: Option<method_call_inf::MethodCallInf3>,
) -> Result<HirExpression> {
    check_argument_types(mk, &found.sig, &receiver_hir, &mut arg_hirs, inf)?;
    let specialized = receiver_hir.ty.is_specialized();
    let first_arg_ty = arg_hirs.get(0).map(|x| x.ty.clone());

    let owner = mk.class_dict.get_type(&found.owner);
    let receiver = Hir::bit_cast(owner.erasure().to_term_ty(), receiver_hir);
    let args = if specialized {
        arg_hirs
            .into_iter()
            .map(|expr| Hir::bit_cast(ty::raw("Object"), expr))
            .collect::<Vec<_>>()
    } else {
        arg_hirs
    };

    let hir = build_hir(&found, &owner, receiver, args);
    if found.sig.fullname.full_name == "Object#unsafe_cast" {
        Ok(Hir::bit_cast(first_arg_ty.unwrap().instance_ty(), hir))
    } else if specialized {
        Ok(Hir::bit_cast(found.sig.ret_ty, hir))
    } else {
        Ok(hir)
    }
}

fn check_argument_types(
    mk: &HirMaker,
    sig: &MethodSignature,
    receiver_hir: &HirExpression,
    arg_hirs: &mut [HirExpression],
    inf: Option<method_call_inf::MethodCallInf3>,
) -> Result<()> {
    type_checking::check_method_args(&mk.class_dict, sig, receiver_hir, arg_hirs, inf)?;
    if let Some(last_arg) = arg_hirs.last_mut() {
        check_break_in_block(sig, last_arg)?;
    }
    Ok(())
}

/// Check if `break` in block is valid
fn check_break_in_block(sig: &MethodSignature, last_arg: &mut HirExpression) -> Result<()> {
    if let HirExpressionBase::HirLambdaExpr { has_break, .. } = last_arg.node {
        if has_break {
            if sig.ret_ty == ty::raw("Void") {
                match &mut last_arg.node {
                    HirExpressionBase::HirLambdaExpr { ret_ty, .. } => {
                        std::mem::swap(ret_ty, &mut ty::raw("Void"));
                    }
                    _ => panic!("[BUG] unexpected type"),
                }
            } else {
                return Err(error::program_error(
                    "`break' not allowed because this block is expected to return a value",
                ));
            }
        }
    }
    Ok(())
}

fn build_hir(
    // The method
    found: &FoundMethod,
    // The class/module which has the method
    owner: &SkType,
    receiver_hir: HirExpression,
    arg_hirs: Vec<HirExpression>,
) -> HirExpression {
    let ret_ty = found.sig.ret_ty.clone(); //substitute(class_tyargs, method_tyargs);
    match owner {
        SkType::Class(_) => {
            Hir::method_call(ret_ty, receiver_hir, found.sig.fullname.clone(), arg_hirs)
        }
        SkType::Module(sk_module) => Hir::module_method_call(
            ret_ty,
            receiver_hir,
            sk_module.fullname(),
            found.sig.fullname.first_name.clone(),
            found.method_idx.unwrap(),
            arg_hirs,
        ),
    }
}
