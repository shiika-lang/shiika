use crate::class_dict::FoundMethod;
use crate::convert_exprs::{block, LVarInfo};
use crate::error;
use crate::hir_maker::HirMaker;
use crate::type_system::type_checking;
use anyhow::Result;
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
            if let Some(hir) = convert_lambda_invocation(mk, arg_exprs, has_block, locs, lvar)? {
                return Ok(hir);
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
    let arg_hirs = convert_method_args(
        mk,
        &block::BlockTaker::Method {
            sig: found.sig.clone(),
            locs,
        },
        arg_exprs,
        has_block,
    )?;
    build(mk, found, receiver_hir, arg_hirs)
}

/// Returns `Some` if the method call is a lambda invocation.
fn convert_lambda_invocation(
    mk: &mut HirMaker,
    arg_exprs: &[AstExpression],
    has_block: &bool,
    locs: &LocationSpan,
    lvar: LVarInfo,
) -> Result<Option<HirExpression>> {
    let tys = if let Some(tys) = lvar.ty.fn_x_info() {
        tys
    } else {
        return Ok(None);
    };
    let arg_hirs = convert_method_args(
        mk,
        &block::BlockTaker::Function {
            fn_ty: &lvar.ty,
            locs,
        },
        arg_exprs,
        has_block, // true if `f(){ ... }`, for example.
    )?;
    let ret_ty = tys.last().unwrap();
    Ok(Some(Hir::lambda_invocation(
        ret_ty.clone(),
        lvar.ref_expr(),
        arg_hirs,
        locs.clone(),
    )))
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
pub fn convert_method_args(
    mk: &mut HirMaker,
    block_taker: &block::BlockTaker,
    arg_exprs: &[AstExpression],
    has_block: &bool,
) -> Result<Vec<HirExpression>> {
    let n = arg_exprs.len();
    let mut arg_hirs = vec![];
    for i in 0..n {
        let arg_hir = if *has_block && i == n - 1 {
            block::convert_block(mk, block_taker, &arg_exprs[i])?
        } else {
            mk.convert_expr(&arg_exprs[i])?
        };
        arg_hirs.push(arg_hir);
    }
    Ok(arg_hirs)
}

/// Check the arguments and create HirMethodCall or HirModuleMethodCall
pub fn build(
    mk: &HirMaker,
    found: FoundMethod,
    receiver_hir: HirExpression,
    mut arg_hirs: Vec<HirExpression>,
) -> Result<HirExpression> {
    check_argument_types(mk, &found.sig, &receiver_hir, &mut arg_hirs)?;
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
) -> Result<()> {
    type_checking::check_method_args(&mk.class_dict, sig, receiver_hir, arg_hirs)?;
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
    found: &FoundMethod,
    owner: &SkType,
    receiver_hir: HirExpression,
    arg_hirs: Vec<HirExpression>,
) -> HirExpression {
    match owner {
        SkType::Class(_) => Hir::method_call(
            found.sig.ret_ty.clone(),
            receiver_hir,
            found.sig.fullname.clone(),
            arg_hirs,
        ),
        SkType::Module(sk_module) => Hir::module_method_call(
            found.sig.ret_ty.clone(),
            receiver_hir,
            sk_module.fullname(),
            found.sig.fullname.first_name.clone(),
            found.method_idx.unwrap(),
            arg_hirs,
        ),
    }
}
