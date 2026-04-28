//! Lowering of `HirMatchExpression` to MIR.
//!
//! The HIR provides a scrutinee assignment plus a list of `MatchClause`s,
//! each made of a sequence of `Test`/`Bind` components and a body. We lower
//! to a chain of nested `If`s; each clause's components fold into a single
//! Bool-typed test chain that interleaves binds (as `LVarDecl`) and tests
//! (as `If(t, rest, false)`), so no expression is duplicated.
use crate::mir;
use skc_hir::pattern_match::{Component, MatchClause};
use skc_hir::{HirExpression, HirExpressionBase};

impl<'a> super::Compiler<'a> {
    pub(super) fn convert_match_expr(
        &mut self,
        cond_assign_expr: HirExpression,
        clauses: Vec<MatchClause>,
    ) -> mir::TypedExpr {
        // The cond_assign_expr is always a HirLVarAssign($exprN, scrutinee)
        // produced by skc_ast2hir. Convert it to LVarDecl so insert_allocs
        // emits the alloca for the temp.
        let scrutinee_decl = match cond_assign_expr.node {
            HirExpressionBase::HirLVarAssign { name, rhs } => {
                let mir_rhs = self.convert_expr(*rhs);
                mir::Expr::lvar_decl(name, mir_rhs, false)
            }
            other => panic!(
                "[BUG] expected HirLVarAssign as match cond_assign_expr, got {:?}",
                other
            ),
        };
        let body = self.build_match_clauses(clauses);
        mir::Expr::exprs(vec![scrutinee_decl, body])
    }

    fn build_match_clauses(&mut self, clauses: Vec<MatchClause>) -> mir::TypedExpr {
        // skc_ast2hir always appends a final unconditional clause whose body
        // panics. Recurse so it sits at the innermost else.
        let mut iter = clauses.into_iter();
        let first = iter.next().expect("[BUG] match expression with no clauses");
        if iter.len() == 0 {
            // Just the panic clause; emit its body unconditionally.
            return self.convert_expr(first.body_hir);
        }
        let rest: Vec<_> = iter.collect();
        let cond = self.build_test_chain(first.components);
        let then_ = self.convert_expr(first.body_hir);
        let else_ = self.build_match_clauses(rest);
        mir::Expr::if_(cond, then_, else_)
    }

    fn build_test_chain(&mut self, components: Vec<Component>) -> mir::TypedExpr {
        let mut iter = components.into_iter();
        match iter.next() {
            None => mir::Expr::pseudo_var(mir::PseudoVar::True),
            Some(Component::Test(t)) => {
                let mir_t = self.convert_expr(t);
                let rest = self.build_test_chain(iter.collect());
                mir::Expr::if_(mir_t, rest, mir::Expr::pseudo_var(mir::PseudoVar::False))
            }
            Some(Component::Bind(name, e)) => {
                let mir_e = self.convert_expr(e);
                let decl = mir::Expr::lvar_decl(name, mir_e, false);
                let rest = self.build_test_chain(iter.collect());
                mir::Expr::exprs(vec![decl, rest])
            }
        }
    }
}
