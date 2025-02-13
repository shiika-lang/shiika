//! Convert Hir to JSON (for debugging use)
use crate::{HirExpression, HirExpressionBase::*, HirExpressions};
use serde_json::{json, Value};
use shiika_core::ty::TermTy;

pub fn exprs(x: &HirExpressions) -> Value {
    let exprs = x.exprs.iter().map(|e| expr(e)).collect::<Vec<_>>();
    json!({"ty": ty(&x.ty), "exprs": exprs})
}

pub fn expr(x: &HirExpression) -> Value {
    let t = ty(&x.ty);
    match &x.node {
        HirLogicalNot { expr: e } => json!({"ty": t, "node": "not", "expr": expr(e)}),
        HirLogicalAnd { left, right } => {
            json!({"ty": t, "node": "and", "left": expr(left), "right": expr(right) })
        }
        HirLogicalOr { left, right } => {
            json!({"ty": t, "node": "or", "left": expr(left), "right": expr(right) })
        }
        HirIfExpression {
            cond_expr,
            then_exprs,
            else_exprs,
        } => {
            json!({"ty": t, "node": "if", "cond": expr(cond_expr), "then": exprs(then_exprs), "else": exprs(else_exprs) })
        }
        HirMatchExpression {
            cond_assign_expr,
            clauses,
        } => json!({"ty": t, "node": "match(TODO)" }),
        HirWhileExpression {
            cond_expr,
            body_exprs,
        } => json!("TODO"),
        HirBreakExpression { from } => json!("TODO"),
        HirReturnExpression { arg, .. } => json!("TODO"),
        HirLVarAssign { name, rhs } => json!("TODO"),
        HirIVarAssign {
            name,
            idx,
            rhs,
            self_ty,
            ..
        } => json!("TODO"),
        HirConstAssign { fullname, rhs } => json!("TODO"),
        HirMethodCall {
            receiver_expr,
            method_fullname,
            arg_exprs,
        } => json!("TODO"),
        HirModuleMethodCall {
            receiver_expr,
            module_fullname,
            method_name,
            method_idx,
            arg_exprs,
        } => json!("TODO"),
        HirLambdaInvocation {
            lambda_expr,
            arg_exprs,
        } => json!("TODO"),
        HirArgRef { idx } => json!("TODO"),
        HirLVarRef { name } => json!("TODO"),
        HirIVarRef { name, idx, self_ty } => json!("TODO"),
        HirTVarRef {
            typaram_ref,
            self_ty,
        } => json!("TODO"),
        HirConstRef { fullname } => json!("TODO"),
        HirLambdaExpr {
            name,
            params,
            captures,
            ret_ty,
            ..
        } => json!("TODO"),
        HirSelfExpression => json!("TODO"),
        HirFloatLiteral { value } => json!("TODO"),
        HirDecimalLiteral { value } => json!("TODO"),
        HirStringLiteral { idx } => json!("TODO"),
        HirBooleanLiteral { value } => json!("TODO"),

        HirLambdaCaptureRef { idx, readonly } => json!("TODO"),
        HirLambdaCaptureWrite { cidx, rhs } => json!("TODO"),
        HirBitCast { expr: target } => json!("TODO"),
        HirClassLiteral {
            fullname,
            str_literal_idx,
            includes_modules,
            initialize_name,
            init_cls_name,
        } => json!("TODO"),
        HirParenthesizedExpr { exprs } => json!("TODO"),
    }
}

fn ty(x: &TermTy) -> Value {
    json!(format!("{}", x))
}
