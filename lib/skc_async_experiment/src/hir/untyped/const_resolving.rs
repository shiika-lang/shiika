use anyhow::Result;
use shiika_ast::{self, AstExpression, AstVisitor};

#[derive(Debug)]
pub struct ConstDefinition {
    pub name: ResolvedConstName,
    pub namespace: Namespace,
    pub const_init_expr: AstExpression,
}


pub fn run(ast: &shiika_ast::Program) -> Vec<ConstDefinition> {
    let mut visitor = Visitor(vec![]);
    visitor.walk_program(ast).unwrap();
    visitor.0
}

pub struct Visitor {
}
impl AstVisitor for Visitor {
    fn visit_const_definition(
        &mut self,
        namespace: &shiika_core::names::Namespace,
        name: &str,
        expr: &AstExpression,
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
        self.0.push(ConstDefinition {
            name: shiika_core::names::ResolvedConstName::new(names),
            namespace: namespace.clone(),
            const_init_expr,
        });
        Ok(())
    }
}
