//! Handles method parameters and block parameters.
//!
//! - Method parameters always has type annotations.
//! - Block parameters may not have type annotations. In this case, the type
//!   is inferred from the method signature.
//!   eg:
//!
//! ```sk
//! [1,2,3].each{|i| p i}
//! # `i` is inferred as `Int` from the signature of `Array<Int>#each`.
//! ```
use crate::class_dict::ClassDict;
use crate::convert_exprs::MethodParam;
use anyhow::Result;
use shiika_core::names::Namespace;
use shiika_core::ty::{self, TermTy};

/// Convert `shiika_ast::Param`s to hir params.
pub fn convert_params(
    class_dict: &ClassDict,
    namespace: &Namespace,
    ast_params: &[shiika_ast::Param],
    class_typarams: &[ty::TyParam],
    method_typarams: &[ty::TyParam],
) -> Result<Vec<MethodParam>> {
    let mut hir_params = vec![];
    for param in ast_params {
        let ty =
            class_dict._resolve_typename(namespace, class_typarams, method_typarams, &param.typ)?;
        hir_params.push(MethodParam {
            name: param.name.to_string(),
            ty: ty.clone(),
        });
    }
    Ok(hir_params)
}

/// Convert `shiika_ast::BlockParam`s to hir params.
/// Type annotation is optional for block parameters. If not provided, it will
/// be inferred from the signature of the method that takes the block.
pub fn convert_block_params(
    class_dict: &ClassDict,
    namespace: &Namespace,
    ast_params: &[shiika_ast::BlockParam],
    class_typarams: &[ty::TyParam],
    // Blocks cannot have type parameters. However, it is allowed to refer to
    // the typarams of the current method.
    method_typarams: &[ty::TyParam],
    // `[arg1_ty, arg2_ty, ..., ret_ty]`
    type_hint: &[TermTy],
) -> Result<Vec<MethodParam>> {
    let mut hir_params = vec![];
    for (i, param) in ast_params.iter().enumerate() {
        let hir_param = if let Some(typ) = &param.opt_typ {
            // Has type annotation `typ`
            let ty =
                class_dict._resolve_typename(namespace, class_typarams, method_typarams, typ)?;
            MethodParam {
                name: param.name.to_string(),
                ty: ty.clone(),
            }
        } else {
            // Infer from hint
            let ty = type_hint.get(i).expect("type hint not found");
            MethodParam {
                name: param.name.to_string(),
                ty: ty.clone(),
            }
        };
        hir_params.push(hir_param);
    }
    Ok(hir_params)
}
