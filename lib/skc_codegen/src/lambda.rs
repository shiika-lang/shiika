use crate::utils::{llvm_func_name, LlvmFuncName};
use crate::CodeGen;
use anyhow::Result;
use either::Either::*;
use shiika_core::ty::*;
use skc_hir::HirExpressionBase::*;
use skc_hir::*;

impl<'hir: 'ictx, 'run, 'ictx: 'run> CodeGen<'hir, 'run, 'ictx> {
    /// Find all lambdas in a hir and create the body of the corresponding llvm function
    /// PERF: Ideally they should be created during gen_methods but I couldn't
    /// avoid borrow checker errors.
    pub(super) fn gen_lambda_funcs(&self, hir: &'hir Hir) -> Result<()> {
        for methods in hir.sk_methods.values() {
            for method in methods {
                if let SkMethodBody::Normal { exprs } = &method.body {
                    self.gen_lambda_funcs_in_exprs(&exprs.exprs)?;
                }
            }
        }

        for expr in &hir.const_inits {
            self.gen_lambda_funcs_in_expr(expr)?;
        }

        self.gen_lambda_funcs_in_exprs(&hir.main_exprs.exprs)?;
        Ok(())
    }

    fn gen_lambda_funcs_in_exprs(&self, exprs: &'hir [HirExpression]) -> Result<()> {
        for expr in exprs {
            self.gen_lambda_funcs_in_expr(expr)?;
        }
        Ok(())
    }

    fn gen_lambda_funcs_in_expr(&self, expr: &'hir HirExpression) -> Result<()> {
        match &expr.node {
            HirLogicalNot { expr } => self.gen_lambda_funcs_in_expr(expr)?,
            HirLogicalAnd { left, right } => {
                self.gen_lambda_funcs_in_expr(left)?;
                self.gen_lambda_funcs_in_expr(right)?;
            }
            HirLogicalOr { left, right } => {
                self.gen_lambda_funcs_in_expr(left)?;
                self.gen_lambda_funcs_in_expr(right)?;
            }
            HirIfExpression {
                cond_expr,
                then_exprs,
                else_exprs,
            } => {
                self.gen_lambda_funcs_in_expr(cond_expr)?;
                self.gen_lambda_funcs_in_exprs(&then_exprs.exprs)?;
                self.gen_lambda_funcs_in_exprs(&else_exprs.exprs)?;
            }
            HirMatchExpression {
                cond_assign_expr,
                clauses,
            } => {
                self.gen_lambda_funcs_in_expr(cond_assign_expr)?;
                for clause in clauses {
                    self.gen_lambda_funcs_in_exprs(&clause.body_hir.exprs)?;
                }
            }
            HirWhileExpression {
                cond_expr,
                body_exprs,
            } => {
                self.gen_lambda_funcs_in_expr(cond_expr)?;
                self.gen_lambda_funcs_in_exprs(&body_exprs.exprs)?;
            }
            HirBreakExpression { .. } => (),
            HirReturnExpression { arg, .. } => self.gen_lambda_funcs_in_expr(arg)?,
            HirLVarAssign { rhs, .. } => self.gen_lambda_funcs_in_expr(rhs)?,
            HirIVarAssign { rhs, .. } => self.gen_lambda_funcs_in_expr(rhs)?,
            HirConstAssign { rhs, .. } => self.gen_lambda_funcs_in_expr(rhs)?,
            HirMethodCall {
                receiver_expr,
                arg_exprs,
                ..
            } => {
                self.gen_lambda_funcs_in_expr(receiver_expr)?;
                for expr in arg_exprs {
                    self.gen_lambda_funcs_in_expr(expr)?;
                }
            }
            HirLambdaInvocation {
                lambda_expr,
                arg_exprs,
            } => {
                self.gen_lambda_funcs_in_expr(lambda_expr)?;
                for expr in arg_exprs {
                    self.gen_lambda_funcs_in_expr(expr)?;
                }
            }
            HirArgRef { .. } => (),
            HirLVarRef { .. } => (),
            HirIVarRef { .. } => (),
            HirConstRef { .. } => (),
            HirLambdaExpr {
                name,
                params,
                exprs,
                ret_ty,
                lvars,
                ..
            } => {
                self.gen_lambda_func(&llvm_func_name(name), params, exprs, ret_ty, lvars)?;
                self.gen_lambda_funcs_in_exprs(&exprs.exprs)?;
            }
            HirSelfExpression => (),
            HirArrayLiteral { exprs } => self.gen_lambda_funcs_in_exprs(exprs)?,
            HirFloatLiteral { .. } => (),
            HirDecimalLiteral { .. } => (),
            HirStringLiteral { .. } => (),
            HirBooleanLiteral { .. } => (),

            HirLambdaCaptureRef { .. } => (),
            HirLambdaCaptureWrite { rhs, .. } => self.gen_lambda_funcs_in_expr(rhs)?,
            HirBitCast { expr } => self.gen_lambda_funcs_in_expr(expr)?,
            HirClassLiteral { .. } => (),
            HirParenthesizedExpr { exprs } => self.gen_lambda_funcs_in_exprs(&exprs.exprs)?,
        }
        Ok(())
    }

    fn gen_lambda_func(
        &self,
        func_name: &LlvmFuncName,
        params: &'hir [MethodParam],
        exprs: &'hir HirExpressions,
        ret_ty: &TermTy,
        lvars: &[(String, TermTy)],
    ) -> Result<()> {
        self.gen_llvm_func_body(func_name, params, Right(exprs), lvars, ret_ty, true)
    }
}
