use crate::mir::{PrimBinOp, Ty};
use shiika_core::names::MethodFullname;
use skc_hir::{HirExpression, HirExpressionBase};

/// Only exact, concrete core-method calls can bypass the call ABI.
/// Module/erased dispatch keeps using its async implementation and wrappers.
pub(super) fn recognize(
    method: &MethodFullname,
    receiver_ty: &Ty,
    args: &[HirExpression],
    tyargs: &[HirExpression],
    result_ty: &Ty,
) -> Option<PrimBinOp> {
    let op = match (method.type_name.0.as_str(), method.first_name.0.as_str()) {
        ("Int", "<") => PrimBinOp::IntLt,
        ("Int", "<=") => PrimBinOp::IntLe,
        ("Int", ">") => PrimBinOp::IntGt,
        ("Int", ">=") => PrimBinOp::IntGe,
        ("Int", "==") => PrimBinOp::IntEq,
        ("Bool", "==") => PrimBinOp::BoolEq,
        ("Float", "<") => PrimBinOp::FloatLt,
        ("Float", "<=") => PrimBinOp::FloatLe,
        ("Float", ">") => PrimBinOp::FloatGt,
        ("Float", ">=") => PrimBinOp::FloatGe,
        ("Float", "==") => PrimBinOp::FloatEq,
        _ => return None,
    };
    let [right] = args else { return None };
    if !tyargs.is_empty()
        || matches!(right.node, HirExpressionBase::HirDefaultExpr)
        || *receiver_ty != op.operand_ty()
        || Ty::from(right.ty.clone()) != op.operand_ty()
        || *result_ty != Ty::I1
    {
        return None;
    }
    Some(op)
}
