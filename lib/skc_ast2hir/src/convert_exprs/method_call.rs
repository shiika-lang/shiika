use crate::class_dict::FoundMethod;
use crate::convert_exprs::{block, block::BlockTaker};
use crate::error;
use crate::hir_maker::HirMaker;
use crate::type_inference::{self, method_call_inf};
use crate::type_system::type_checking;
use anyhow::{Context, Result};
use shiika_ast::{AstCallArgs, AstExpression, LocationSpan};
use shiika_core::names::{method_fullname, MethodFirstname};
use shiika_core::{ty, ty::TermTy};
use skc_hir::*;

pub enum ArrangedArg<'ast> {
    Expr(&'ast AstExpression),
    Default(&'ast TermTy),
}

/// Entry point of Converting `AstMethodCall` into `HirMethodCall`.
pub fn convert_method_call(
    mk: &mut HirMaker,
    receiver_expr: &Option<Box<AstExpression>>,
    method_name: &MethodFirstname,
    args: &AstCallArgs,
    type_args: &[AstExpression],
    locs: &LocationSpan,
) -> Result<HirExpression> {
    // Check if this is a lambda invocation
    if receiver_expr.is_none() {
        if let Some(lvar) = mk._lookup_var(&method_name.0, locs.clone())? {
            if lvar.ty.fn_x_info().is_some() {
                return convert_lambda_invocation(mk, lvar.ref_expr(), args, locs);
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
    let arranged = arrange_named_args(&found.sig, args)?;
    validate_method_tyargs(&found, type_args)?;

    let inf1 = if found.sig.typarams.len() > 0 && type_args.is_empty() {
        let sig = &found.sig; //.specialize(class_tyargs, method_tyargs);
        Some(method_call_inf::MethodCallInf1::new(sig, args.has_block()))
    } else if args.has_block() {
        type_checking::check_takes_block(&found.sig, locs)?;
        Some(method_call_inf::MethodCallInf1::infer_block(&found.sig))
    } else {
        None
    };
    let (arg_hirs, inf3) = convert_method_args(
        mk,
        inf1,
        &BlockTaker::Method {
            sig: found.sig.clone(),
            locs,
        },
        &arranged,
        args.has_block(),
    )?;

    build(mk, found, receiver_hir, arg_hirs, inf3, method_tyargs, locs)
}

/// Arrange named and unnamed arguments into a Vec which corresponds to `sig.params`.
/// Also, put the default expression if necessary.
pub fn arrange_named_args<'a>(
    sig: &'a MethodSignature,
    args: &'a AstCallArgs,
) -> Result<Vec<ArrangedArg<'a>>> {
    let n_params = sig.params.len();
    let n_unnamed = args.unnamed.len();
    let mut named = args.named.iter().collect::<Vec<_>>();
    let mut v = vec![];
    for (i, param) in sig.params.iter().enumerate() {
        if i < n_unnamed {
            v.push(ArrangedArg::Expr(args.unnamed.get(i).unwrap()));
            continue;
        }
        if let Some(j) = named.iter().position(|(name, _)| name == &param.name) {
            let (_, expr) = named.remove(j);
            v.push(ArrangedArg::Expr(expr));
            continue;
        }
        if let Some(expr) = &args.block {
            if i == n_params - 1 {
                v.push(ArrangedArg::Expr(expr));
                continue;
            }
        }
        if param.has_default {
            v.push(ArrangedArg::Default(&param.ty));
            continue;
        }
        return Err(error::unspecified_arg(
            &param.name,
            sig,
            &args.locs().unwrap(),
        ));
    }
    if let Some((name, _)) = named.first() {
        return Err(error::extranous_arg(name, sig, &args.locs().unwrap()));
    }
    Ok(v)
}

/// Check if number of type arguments matches to the typarams.
/// If no tyargs are given, check is skipped (it will be inferred instead.)
pub fn validate_method_tyargs(found: &FoundMethod, type_args: &[AstExpression]) -> Result<()> {
    if type_args.len() > 0 && type_args.len() != found.sig.typarams.len() {
        return Err(error::type_error(format!(
            "wrong number of method-wise type arguments ({} for {:?}",
            type_args.len(),
            &found.sig,
        )));
    }
    Ok(())
}

/// Returns `Some` if the method call is a lambda invocation.
pub fn convert_lambda_invocation(
    mk: &mut HirMaker,
    fn_expr: HirExpression,
    args: &AstCallArgs,
    locs: &LocationSpan,
) -> Result<HirExpression> {
    if let Some((name, _)) = args.named.first() {
        return Err(error::named_arg_for_lambda(name, locs));
    }
    let arg_exprs = args
        .all_exprs()
        .iter()
        .map(|e| ArrangedArg::Expr(e))
        .collect::<Vec<_>>();

    let (arg_hirs, _) = convert_method_args(
        mk,
        None,
        &BlockTaker::Function {
            fn_ty: &fn_expr.ty,
            locs,
        },
        &arg_exprs,
        args.has_block(),
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
    // The method or lambda to be called.
    // (TODO: this name is odd when has_block=false...)
    block_taker: &BlockTaker,
    arg_exprs: &[ArrangedArg],
    has_block: bool,
) -> Result<(Vec<HirExpression>, Option<method_call_inf::MethodCallInf3>)> {
    let infer_block = has_block && inf.is_some();
    let mut n = arg_exprs.len();
    if infer_block {
        n -= 1;
    }
    let mut arg_hirs = (0..n)
        .map(|i| match &arg_exprs[i] {
            ArrangedArg::Expr(e) => mk.convert_expr(e),
            ArrangedArg::Default(ty) => Ok(Hir::default_expression((*ty).clone())),
        })
        .collect::<Result<Vec<_>>>()?;

    if infer_block {
        let arg_tys = arg_hirs.iter().map(|x| &x.ty).collect::<Vec<_>>();
        let inf2 = method_call_inf::infer_block_param(inf.unwrap(), &arg_tys).context(format!(
            "failed to infer block parameter of {}",
            block_taker
        ))?;
        let block_hir = block::convert_block(mk, block_taker, &inf2, &arg_exprs.last().unwrap())?;
        let inf3 = method_call_inf::infer_result_ty_with_block(inf2, &block_hir.ty)
            .context(format!("failed to infer result type of {}", block_taker))?;

        arg_hirs.push(block_hir);
        Ok((arg_hirs, Some(inf3)))
    } else {
        Ok((arg_hirs, None))
    }
}

/// For method calls without any arguments.
pub fn build_simple(
    mk: &mut HirMaker,
    found: FoundMethod,
    receiver_hir: HirExpression,
    locs: &LocationSpan,
) -> Result<HirExpression> {
    build(
        mk,
        found,
        receiver_hir,
        Default::default(),
        Default::default(),
        Default::default(),
        locs,
    )
}

/// Check the arguments and create HirMethodCall or HirModuleMethodCall
pub fn build(
    mk: &mut HirMaker,
    found: FoundMethod,
    receiver_hir: HirExpression,
    mut arg_hirs: Vec<HirExpression>,
    inf: Option<method_call_inf::MethodCallInf3>,
    method_tyargs: Vec<TermTy>,
    locs: &LocationSpan,
) -> Result<HirExpression> {
    check_argument_types(mk, &found.sig, &receiver_hir, &mut arg_hirs, &inf)?;
    let receiver_ty = receiver_hir.ty.clone();
    let specialized = receiver_hir.ty.is_specialized();
    let first_arg_ty = arg_hirs.get(0).map(|x| x.ty.clone());
    let arg_types = arg_hirs.iter().map(|x| x.ty.clone()).collect::<Vec<_>>();

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

    // Special handling for `Foo<X>.new`, or
    // `Foo.new(x)`, in which case `X` is inferred from the type of `x`.
    if found.is_generic_new(&receiver_ty) {
        let tyargs = if method_tyargs.is_empty() {
            let err = error::method_tyarg_inference_failed(
                format!("Could not infer type arg(s) of {}", found.sig),
                locs,
            );
            type_inference::generic_new::infer_tyargs(&found.sig, &arg_types).context(err)?
        } else {
            method_tyargs
        };
        return Ok(call_specialized_new(mk, &receiver_ty, args, tyargs, locs));
    }

    let hir = build_hir(&found, &owner, receiver, args, &inf);
    if found.sig.fullname.full_name == "Object#unsafe_cast" {
        Ok(Hir::bit_cast(first_arg_ty.unwrap().instance_ty(), hir))
    } else {
        Ok(hir)
    }
}

fn check_argument_types(
    mk: &HirMaker,
    sig: &MethodSignature,
    receiver_hir: &HirExpression,
    arg_hirs: &mut [HirExpression],
    inf: &Option<method_call_inf::MethodCallInf3>,
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
    inf: &Option<method_call_inf::MethodCallInf3>,
) -> HirExpression {
    let ret_ty = match inf {
        Some(inf_) => inf_.solved_method_ret_ty.clone(),
        None => found.sig.ret_ty.clone(), //.substitute(class_tyargs, method_tyargs);
    };
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

/// Build HIR for `Foo<Bar>.new(x)`
fn call_specialized_new(
    mk: &mut HirMaker,
    // `TermTy(Meta:Foo)`
    receiver_ty: &TermTy,
    // Args for .new
    arg_hirs: Vec<HirExpression>,
    tyargs: Vec<TermTy>,
    locs: &LocationSpan,
) -> HirExpression {
    let meta_spe_ty = receiver_ty.specialized_ty(tyargs);
    let spe_cls = mk.get_class_object(&meta_spe_ty, locs);
    Hir::method_call(
        meta_spe_ty.instance_ty(),
        spe_cls,
        method_fullname(receiver_ty.erasure().to_type_fullname(), "new"),
        arg_hirs,
    )
}
