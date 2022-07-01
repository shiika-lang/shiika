use crate::class_dict::FoundMethod;
use crate::error;
use crate::hir_maker::HirMaker;
use crate::type_system::type_checking;
use anyhow::Result;
use shiika_core::ty;
use skc_hir::*;

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

    let receiver = Hir::bit_cast(found.owner.erasure().to_term_ty(), receiver_hir);
    let args = if specialized {
        arg_hirs
            .into_iter()
            .map(|expr| Hir::bit_cast(ty::raw("Object"), expr))
            .collect::<Vec<_>>()
    } else {
        arg_hirs
    };

    let hir = build_hir(&found, receiver, args);
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
    let arg_tys = arg_hirs.iter().map(|expr| &expr.ty).collect::<Vec<_>>();
    type_checking::check_method_args(&mk.class_dict, sig, &arg_tys, receiver_hir, arg_hirs)?;
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
    receiver_hir: HirExpression,
    arg_hirs: Vec<HirExpression>,
) -> HirExpression {
    match found.owner {
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
