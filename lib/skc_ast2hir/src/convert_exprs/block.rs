use crate::convert_exprs::params;
use crate::hir_maker::{extract_lvars, HirMaker};
use crate::hir_maker_context::HirMakerContext;
use crate::type_system::type_checking;
use anyhow::Result;
use shiika_ast::{AstExpression, AstExpressionBody, LocationSpan};
use shiika_core::ty::{self, TermTy};
use skc_hir::{Hir, HirExpression, MethodParam, MethodSignature};
use std::fmt;

/// Type information of the method or fn which takes the block.
#[derive(Debug)]
pub enum BlockTaker<'hir_maker> {
    Method(MethodSignature),
    Function(&'hir_maker TermTy),
}

// For error message
impl<'hir_maker> fmt::Display for BlockTaker<'hir_maker> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockTaker::Method(sig) => write!(f, "{}", sig),
            BlockTaker::Function(fn_ty) => write!(f, "fn {}", fn_ty),
        }
    }
}

/// Convert a block to HirLambdaExpr.
/// `arg_expr` must be a LambdaExpr.
pub fn convert_block(
    mk: &mut HirMaker,
    block_taker: &BlockTaker,
    arg_expr: &AstExpression,
) -> Result<HirExpression> {
    match &arg_expr.body {
        AstExpressionBody::LambdaExpr {
            params,
            exprs,
            is_fn,
        } => {
            debug_assert!(!is_fn);
            _convert_block(mk, block_taker, params, exprs)
        }
        _ => panic!("expected LambdaExpr but got {:?}", arg_expr),
    }
}

/// Convert a block to HirLambdaExpr
/// Types of block parameters are inferred from `block_ty` (arg_ty1, arg_ty2, ..., ret_ty)
fn _convert_block(
    mk: &mut HirMaker,
    block_taker: &BlockTaker,
    params: &[shiika_ast::BlockParam],
    body_exprs: &[AstExpression],
) -> Result<HirExpression> {
    type_checking::check_block_arity(block_taker, params)?;

    let namespace = mk.ctx_stack.const_scopes().next().unwrap();
    let hir_params = params::convert_block_params(
        &mk.class_dict,
        &namespace,
        params,
        &mk.ctx_stack.current_class_typarams(),
        &mk.ctx_stack.current_method_typarams(),
        block_ty_of(block_taker),
    )?;

    // Convert lambda body
    mk.ctx_stack
        .push(HirMakerContext::lambda(false, hir_params.clone()));
    let hir_exprs = mk.convert_exprs(body_exprs)?;
    let mut lambda_ctx = mk.ctx_stack.pop_lambda_ctx();
    Ok(Hir::lambda_expr(
        lambda_ty(&hir_params, &hir_exprs.ty),
        mk.create_lambda_name(),
        hir_params,
        hir_exprs,
        mk._resolve_lambda_captures(lambda_ctx.captures), // hir_captures
        extract_lvars(&mut lambda_ctx.lvars),             // lvars
        lambda_ctx.has_break,
        LocationSpan::todo(),
    ))
}

pub fn lambda_ty(params: &[MethodParam], ret_ty: &TermTy) -> TermTy {
    let mut tyargs = params.iter().map(|x| x.ty.clone()).collect::<Vec<_>>();
    tyargs.push(ret_ty.clone());
    ty::spe(&format!("Fn{}", params.len()), tyargs)
}

/// Returns the type of block accepted by the method or fn.
fn block_ty_of<'a>(block_taker: &'a BlockTaker) -> &'a [TermTy] {
    match block_taker {
        BlockTaker::Method(sig) => sig.block_ty().unwrap(),
        BlockTaker::Function(fn_ty) => {
            let tys = fn_ty.fn_x_info().unwrap();
            let last_arg_ty = tys.get(tys.len() - 2).unwrap();
            last_arg_ty.fn_x_info().unwrap()
        }
    }
}
