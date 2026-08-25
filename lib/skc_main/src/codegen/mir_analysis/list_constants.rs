use crate::mir;
use crate::mir::visitor::MirVisitor;
use anyhow::Result;

/// Returns a list of constants defined
pub fn run(funcs: &[mir::Function]) -> Vec<(String, mir::Ty)> {
    let mut visitor = ListConstants::new();
    visitor.walk_funcs(funcs).unwrap();
    visitor.get()
}

pub struct ListConstants(Vec<(String, mir::Ty)>);
impl ListConstants {
    fn new() -> Self {
        ListConstants(vec![])
    }

    fn get(self) -> Vec<(String, mir::Ty)> {
        self.0
    }
}
impl mir::visitor::MirVisitor for ListConstants {
    fn visit_expr(&mut self, texpr: &mir::TypedExpr) -> Result<()> {
        match texpr {
            (mir::Expr::ConstSet(name, _), ty) => {
                self.0.push((name.clone(), ty.clone()));
            }
            _ => {}
        }
        Ok(())
    }
}
