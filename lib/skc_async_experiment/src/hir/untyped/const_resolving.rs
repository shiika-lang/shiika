use anyhow::Result;
use shiika_ast::{self, AstVisitor};
//use shiika_core::names::ResolvedConstName;

pub fn run(ast: &shiika_ast::Program) -> Vec<shiika_ast::AstExpression> {
    let mut visitor = Visitor(vec![]);
    visitor.walk_program(ast).unwrap();
    visitor.0
}

pub struct Visitor(Vec<shiika_ast::AstExpression>);
impl AstVisitor for Visitor {
    fn visit_const_definition(
        &mut self,
        namespace: &shiika_core::names::Namespace,
        name: &str,
        expr: &shiika_ast::AstExpression,
    ) -> Result<()> {
        let mut names = namespace.0.clone();
        names.push(name.to_string());

        let const_init_expr = shiika_ast::AstExpression {
            body: shiika_ast::AstExpressionBody::ConstAssign {
                names,
                rhs: Box::new(expr.clone()),
            },
            primary: false,
            locs: expr.locs.clone(),
        };
        self.0.push(const_init_expr);
        Ok(())
    }
}
