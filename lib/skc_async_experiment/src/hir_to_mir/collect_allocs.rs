use crate::hir;
use crate::hir::visitor::HirVisitor;
use anyhow::Result;
use shiika_core::ty::TermTy;

pub fn run(body_stmts: &hir::TypedExpr<TermTy>) -> Vec<(String, TermTy)> {
    Allocs::collect(body_stmts)
}

pub struct Allocs(Vec<(String, TermTy)>);
impl Allocs {
    /// Collects `alloc`ed variable names and their types.
    pub fn collect(body_stmts: &hir::TypedExpr<TermTy>) -> Vec<(String, TermTy)> {
        let mut a = Allocs(vec![]);
        a.walk_expr(body_stmts).unwrap();
        a.0
    }
}
impl HirVisitor<TermTy> for Allocs {
    fn visit_expr(&mut self, texpr: &hir::TypedExpr<TermTy>) -> Result<()> {
        match texpr {
            (hir::Expr::LVarDecl(name, rhs), _) => {
                self.0.push((name.clone(), rhs.1.clone()));
            }
            _ => {}
        }
        Ok(())
    }
}
