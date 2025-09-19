use crate::mir;
use crate::mir::visitor::MirVisitor;
use anyhow::Result;

pub fn run(body_stmts: &mir::TypedExpr) -> Vec<(String, mir::Ty)> {
    Allocs::collect(body_stmts)
}

pub struct Allocs(Vec<(String, mir::Ty)>);
impl Allocs {
    /// Collects `alloc`ed variable names and their types.
    pub fn collect(body_stmts: &mir::TypedExpr) -> Vec<(String, mir::Ty)> {
        let mut a = Allocs(vec![]);
        a.walk_expr(body_stmts).unwrap();
        a.0
    }
}
impl MirVisitor for Allocs {
    fn visit_expr(&mut self, texpr: &mir::TypedExpr) -> Result<()> {
        match texpr {
            (mir::Expr::Assign(name, rhs), _) => {
                self.0.push((name.clone(), rhs.1.clone()));
            }
            _ => {}
        }
        Ok(())
    }
}
