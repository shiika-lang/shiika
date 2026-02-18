use crate::sk_method::SkMethodBody;
use crate::HirExpressionBase::*;
use crate::{Hir, HirExpression};
use anyhow::Result;

pub trait HirVisitor<'hir> {
    fn visit_expr(&mut self, expr: &'hir HirExpression) -> Result<()>;
}

pub fn walk_hir<'hir, V: HirVisitor<'hir>>(v: &mut V, hir: &'hir Hir) -> Result<()> {
    for methods in hir.sk_methods.values() {
        for method in methods {
            if let SkMethodBody::Normal { exprs } = &method.body {
                walk_expr(v, exprs)?;
            }
        }
    }

    for expr in &hir.const_inits {
        walk_expr(v, expr)?;
    }

    for expr in &hir.main_exprs {
        walk_expr(v, expr)?;
    }
    Ok(())
}

pub fn walk_expr<'hir, V: HirVisitor<'hir>>(v: &mut V, expr: &'hir HirExpression) -> Result<()> {
    v.visit_expr(expr)?;
    match &expr.node {
        HirLogicalNot { expr } => walk_expr(v, expr)?,
        HirLogicalAnd { left, right } => {
            walk_expr(v, left)?;
            walk_expr(v, right)?;
        }
        HirLogicalOr { left, right } => {
            walk_expr(v, left)?;
            walk_expr(v, right)?;
        }
        HirIfExpression {
            cond_expr,
            then_exprs,
            else_exprs,
            ..
        } => {
            walk_expr(v, cond_expr)?;
            walk_expr(v, then_exprs)?;
            walk_expr(v, else_exprs)?;
        }
        HirMatchExpression {
            cond_assign_expr,
            clauses,
        } => {
            walk_expr(v, cond_assign_expr)?;
            for clause in clauses {
                walk_expr(v, &clause.body_hir)?;
            }
        }
        HirWhileExpression {
            cond_expr,
            body_exprs,
            ..
        } => {
            walk_expr(v, cond_expr)?;
            walk_expr(v, body_exprs)?;
        }
        HirBreakExpression { .. } => (),
        HirReturnExpression { arg, .. } => walk_expr(v, arg)?,
        HirLVarDecl { rhs, .. } | HirLVarAssign { rhs, .. } => walk_expr(v, rhs)?,
        HirIVarAssign { rhs, .. } => walk_expr(v, rhs)?,
        HirConstAssign { rhs, .. } => walk_expr(v, rhs)?,
        HirMethodCall {
            receiver_expr,
            arg_exprs,
            ..
        } => {
            walk_expr(v, receiver_expr)?;
            for expr in arg_exprs {
                walk_expr(v, expr)?;
            }
        }
        HirModuleMethodCall {
            receiver_expr,
            arg_exprs,
            ..
        } => {
            walk_expr(v, receiver_expr)?;
            for expr in arg_exprs {
                walk_expr(v, expr)?;
            }
        }
        HirLambdaInvocation {
            lambda_expr,
            arg_exprs,
        } => {
            walk_expr(v, lambda_expr)?;
            for expr in arg_exprs {
                walk_expr(v, expr)?;
            }
        }
        HirArgRef { .. } => (),
        HirLVarRef { .. } => (),
        HirIVarRef { .. } => (),
        HirClassTVarRef { .. } => (),
        HirMethodTVarRef { .. } => (),
        HirConstRef { .. } => (),
        HirLambdaExpr { exprs, .. } => {
            walk_expr(v, exprs)?;
        }
        HirSelfExpression => (),
        HirArrayLiteral { elem_exprs } => {
            for expr in elem_exprs {
                walk_expr(v, expr)?;
            }
        }
        HirFloatLiteral { .. } => (),
        HirDecimalLiteral { .. } => (),
        HirStringLiteral { .. } => (),
        HirBooleanLiteral { .. } => (),

        HirLambdaCaptureRef { .. } => (),
        HirLambdaCaptureWrite { rhs, .. } => walk_expr(v, rhs)?,
        HirBitCast { expr } => walk_expr(v, expr)?,
        HirClassLiteral { .. } => (),
        HirParenthesizedExpr { exprs } => {
            for expr in exprs {
                walk_expr(v, expr)?;
            }
        }
        HirDefaultExpr { .. } => (),
        HirIsOmittedValue { expr, .. } => walk_expr(v, expr)?,
    }
    Ok(())
}
