use crate::class_dict::FoundMethod;
use crate::convert_exprs::{block, block::BlockTaker};
use crate::error;
use crate::hir_maker::HirMaker;
use crate::type_inference::Infer;
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
    let receiver_ty = &receiver_hir.ty;

    let found = mk
        .class_dict
        .lookup_method(receiver_ty, method_name, locs)?;
    let arranged = arrange_named_args(&found.sig, args, locs)?;

    validate_method_tyargs(&found, type_args)?;
    let method_tyargs = if found.sig.has_typarams() && type_args.is_empty() {
        if found.is_new(&receiver_ty) && receiver_ty.has_type_args() {
            // Special handling for `Foo<Bar>.new`
            Some(receiver_ty.type_args().to_vec())
        } else {
            // The method has typarams but not specified; need to infer them.
            None
        }
    } else {
        Some(
            type_args
                .iter()
                .map(|tyarg| resolve_method_tyarg(mk, tyarg))
                .collect::<Result<Vec<_>>>()?,
        )
    };

    let block_taker = BlockTaker::Method {
        sig: found.sig.clone(),
        locs,
    };
    let class_typarams = &mk
        .class_dict
        .get_type(&found.owner.to_type_fullname())
        .base()
        .typarams;
    let class_tyargs = receiver_ty.type_args();
    let mut inf = Infer::new(&block_taker, class_typarams, class_tyargs, method_tyargs);
    let mut arg_hirs = convert_method_args(mk, &mut inf, &block_taker, &arranged, &args.block)?;

    let tyargs = inf
        .method_tyargs()
        .with_context(|| error(&block_taker, locs))?;
    let updated_param_types = inf.param_tys().with_context(|| error(&block_taker, locs))?;

    check_argument_types(
        mk,
        &found.sig,
        &receiver_hir,
        &mut arg_hirs,
        &updated_param_types,
    )?;

    // Special handling for `Foo.new(x)` where `Foo<T>` is a generic class and
    // `T` is inferred from `x`.
    if found.is_generic_new(&receiver_ty) {
        return Ok(call_specialized_new(
            mk,
            &receiver_ty,
            arg_hirs,
            tyargs,
            locs,
        ));
    }

    let ret_ty = inf.ret_ty().with_context(|| error(&block_taker, locs))?;

    let receiver = Hir::bit_cast(found.owner.to_term_ty(), receiver_hir);
    let first_arg_ty = arg_hirs.first().map(|arg| arg.ty.clone());
    let hir = build_hir(mk, &found, receiver, arg_hirs, tyargs, ret_ty);
    if found.sig.fullname.full_name == "Object#unsafe_cast" {
        Ok(Hir::bit_cast(first_arg_ty.unwrap().instance_ty(), hir))
    } else {
        Ok(hir)
    }
}

/// Arrange named and unnamed arguments into a Vec which corresponds to `sig.params`.
/// Also, put the default expression if necessary.
pub fn arrange_named_args<'a>(
    sig: &'a MethodSignature,
    args: &'a AstCallArgs,
    method_span: &'a LocationSpan,
) -> Result<Vec<ArrangedArg<'a>>> {
    let n_unnamed = args.unnamed.len();
    let mut named = args.named.iter().collect::<Vec<_>>();
    let mut block_seen = false;
    let mut v = vec![];
    let locs = args.locs();
    let error_locs = match &locs {
        Some(loc) => loc,
        None => method_span,
    };
    for (i, param) in sig.params.iter().enumerate() {
        // 1. Take unnamed arguments
        if i < n_unnamed {
            v.push(ArrangedArg::Expr(args.unnamed.get(i).unwrap()));
            continue;
        }
        // 2. Take named arguments
        if let Some(j) = named.iter().position(|(name, _)| name == &param.name) {
            let (_, expr) = named.remove(j);
            v.push(ArrangedArg::Expr(expr));
            continue;
        }
        // 3. Take the block
        if args.block.is_some() && !block_seen {
            block_seen = true;
            continue;
        }
        // 4. If there are more parameters, the arg may be omitted
        if param.has_default {
            v.push(ArrangedArg::Default(&param.ty));
            continue;
        }
        return Err(error::unspecified_arg(&param.name, sig, error_locs));
    }
    if let Some((name, _)) = named.first() {
        return Err(error::extranous_arg(name, sig, error_locs));
    }
    Ok(v)
}

/// Check if number of type arguments matches to the typarams.
pub fn validate_method_tyargs(found: &FoundMethod, type_args: &[AstExpression]) -> Result<()> {
    if type_args.is_empty() {
        // If the method has no typarams, this is just ok.
        // If the method has typarams, it is inferred later.
        return Ok(());
    }
    let expected = found.sig.typarams.len();
    let given = type_args.len();
    if given != expected {
        return Err(error::type_error(format!(
            "wrong number of method-wise type arguments ({} for {:?}",
            given, &found.sig,
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

    let block_taker = BlockTaker::Function {
        fn_ty: &fn_expr.ty,
        locs,
    };
    let mut inf = Infer::new(
        &block_taker,
        Default::default(),
        Default::default(),
        Default::default(),
    );

    let arg_hirs = convert_method_args(mk, &mut inf, &block_taker, &arg_exprs, &args.block)?;
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
    mk.assert_class_expr(&e, &arg.locs)?;
    Ok(e.ty.instance_ty())
}

/// Convert method call arguments to HirExpression's
fn convert_method_args(
    mk: &mut HirMaker,
    inf: &mut Infer,
    // The method or lambda to be called.
    // (TODO: this name is odd when has_block=false...)
    block_taker: &BlockTaker,
    // Note: this does not contain `block`.
    arg_exprs: &[ArrangedArg],
    opt_block: &Option<Box<AstExpression>>,
) -> Result<Vec<HirExpression>> {
    let mut arg_hirs = arg_exprs
        .iter()
        .map(|expr| match expr {
            ArrangedArg::Expr(e) => mk.convert_expr(e),
            ArrangedArg::Default(ty) => Ok(Hir::default_expression((*ty).clone())),
        })
        .collect::<Result<Vec<_>>>()?;
    let arg_tys = arg_hirs.iter().map(|arg| &arg.ty).collect::<Vec<_>>();
    inf.set_arg_tys(&arg_tys)?;

    // Convert the block (if any)
    if let Some(block) = opt_block {
        let block_param_tys = inf.block_param_tys()?;
        let block_hir = block::convert_block(mk, block_taker, &block_param_tys, &block)?;
        inf.set_block_ty(&block_hir.ty)?;
        arg_hirs.push(block_hir);
    }

    Ok(arg_hirs)
}

fn check_argument_types(
    mk: &HirMaker,
    sig: &MethodSignature,
    receiver_hir: &HirExpression,
    arg_hirs: &mut [HirExpression],
    arg_types: &[TermTy],
) -> Result<()> {
    type_checking::check_method_args(&mk.class_dict, sig, receiver_hir, arg_hirs, arg_types)?;
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
    mk: &mut HirMaker,
    // The method
    found: &FoundMethod,
    receiver_hir: HirExpression,
    arg_hirs: Vec<HirExpression>,
    tyargs: Vec<TermTy>,
    ret_ty: TermTy,
) -> HirExpression {
    debug_assert!(tyargs.len() == found.sig.typarams.len());
    let tyarg_hirs = tyargs
        .iter()
        .map(|t| mk.get_class_object(&t.meta_ty(), &receiver_hir.locs))
        .collect();

    match mk.class_dict.get_type(&found.owner.to_type_fullname()) {
        SkType::Class(_) => Hir::method_call(
            ret_ty,
            receiver_hir,
            found.sig.fullname.clone(),
            arg_hirs,
            tyarg_hirs,
        ),
        SkType::Module(sk_module) => Hir::module_method_call(
            ret_ty,
            receiver_hir,
            sk_module.fullname(),
            found.sig.fullname.first_name.clone(),
            found.method_idx.unwrap(),
            arg_hirs,
            tyarg_hirs,
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
    let tyarg_hirs = tyargs
        .iter()
        .map(|t| mk.get_class_object(&t.meta_ty(), locs))
        .collect();
    let meta_spe_ty = receiver_ty.specialized_ty(tyargs);
    let spe_cls = mk.get_class_object(&meta_spe_ty, locs);
    Hir::method_call(
        meta_spe_ty.instance_ty(),
        spe_cls,
        method_fullname(receiver_ty.erasure().to_type_fullname(), "new"),
        arg_hirs,
        tyarg_hirs,
    )
}

fn error(block_taker: &BlockTaker, locs: &LocationSpan) -> String {
    let detail = match block_taker {
        BlockTaker::Method { sig, .. } => format!("{}", sig),
        _ => "here".to_string(),
    };
    error::method_call_tyinf_failed(detail, locs).to_string()
}
