use anyhow::Result;
use shiika_ast::{self, AstVisitor};

pub fn run(ast: &shiika_ast::Program) -> Vec<shiika_ast::AstExpression> {
    let mut visitor = Visitor(vec![]);
    visitor.walk_program(ast).unwrap();
    visitor.0
}

pub struct Visitor(Vec<shiika_ast::AstExpression>);
impl AstVisitor for Visitor {
    fn visit_toplevel_expr(&mut self, expr: &shiika_ast::AstExpression) -> Result<()> {
        self.0.push(expr.clone());
        Ok(())
    }
}
