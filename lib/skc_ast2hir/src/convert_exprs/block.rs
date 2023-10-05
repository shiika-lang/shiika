use crate::convert_exprs::params;
use crate::hir_maker::{extract_lvars, HirMaker};
use crate::hir_maker_context::HirMakerContext;
use crate::type_system::type_checking;
use anyhow::Result;
use shiika_ast::{AstExpression, AstExpressionBody, LocationSpan};
use shiika_core::ty::{self, TermTy, TyParam};
use skc_hir::{Hir, HirExpression, MethodParam, MethodSignature};
use std::fmt;

/// Type information of the method or fn which takes the block.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum BlockTaker<'hir_maker> {
    Method {
        locs: &'hir_maker LocationSpan,
        sig: MethodSignature,
    },
    Function {
        locs: &'hir_maker LocationSpan,
        fn_ty: &'hir_maker TermTy,
    },
}

// For error message
impl<'hir_maker> fmt::Display for BlockTaker<'hir_maker> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockTaker::Method { sig, .. } => write!(f, "{}", sig),
            BlockTaker::Function { fn_ty, .. } => write!(f, "fn {}", fn_ty),
        }
    }
}

impl<'hir_maker> BlockTaker<'hir_maker> {
    pub fn typarams(&self) -> &[TyParam] {
        match self {
            BlockTaker::Method { sig, .. } => &sig.typarams,
            BlockTaker::Function { .. } => &[],
        }
    }

    pub fn param_tys(&self) -> Vec<TermTy> {
        match self {
            BlockTaker::Method { sig, .. } => sig
                .params
                .iter()
                .map(|param| param.ty.clone())
                .collect::<Vec<_>>(),
            BlockTaker::Function { fn_ty, .. } => {
                let mut tys = fn_ty.fn_x_info().unwrap().to_vec();
                // Drop the last ty (i.e. the return type)
                tys.pop();
                tys
            }
        }
    }

    pub fn ret_ty(&self) -> &TermTy {
        match self {
            BlockTaker::Method { sig, .. } => &sig.ret_ty,
            BlockTaker::Function { fn_ty, .. } => {
                let tys = fn_ty.fn_x_info().unwrap();
                tys.last().unwrap()
            }
        }
    }

    pub fn locs(&self) -> &LocationSpan {
        match self {
            BlockTaker::Method { locs, .. } => locs,
            BlockTaker::Function { locs, .. } => locs,
        }
    }
}

/// Convert a block to HirLambdaExpr.
pub fn convert_block(
    mk: &mut HirMaker,
    block_taker: &BlockTaker,
    inferred_block_param_tys: &[TermTy],
    arg: &AstExpression,
) -> Result<HirExpression> {
    match &arg.body {
        AstExpressionBody::LambdaExpr {
            params,
            exprs,
            is_fn,
        } => {
            debug_assert!(!is_fn);
            _convert_block(
                mk,
                block_taker,
                inferred_block_param_tys,
                params,
                exprs,
                arg.locs.clone(),
            )
        }
        _ => panic!("expected LambdaExpr but got {:?}", arg),
    }
}

/// Convert a block to HirLambdaExpr
/// Types of block parameters are inferred from `block_ty` (arg_ty1, arg_ty2, ..., ret_ty) if not
/// specified.
fn _convert_block(
    mk: &mut HirMaker,
    block_taker: &BlockTaker,
    inferred_block_param_tys: &[TermTy],
    params: &[shiika_ast::BlockParam],
    body_exprs: &[AstExpression],
    locs: LocationSpan,
) -> Result<HirExpression> {
    type_checking::check_block_arity(block_taker, inferred_block_param_tys.len(), params)?;

    let namespace = mk.ctx_stack.const_scopes().next().unwrap();
    let hir_params = params::convert_block_params(
        &mk.class_dict,
        &namespace,
        params,
        &mk.ctx_stack.current_class_typarams(),
        &mk.ctx_stack.current_method_typarams(),
        inferred_block_param_tys,
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
        locs,
    ))
}

pub fn lambda_ty(params: &[MethodParam], ret_ty: &TermTy) -> TermTy {
    let mut tyargs = params.iter().map(|x| x.ty.clone()).collect::<Vec<_>>();
    tyargs.push(ret_ty.clone());
    ty::spe(&format!("Fn{}", params.len()), tyargs)
}
