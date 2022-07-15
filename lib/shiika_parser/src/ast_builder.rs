use shiika_ast::{AstExpression, AstExpressionBody, Location, LocationSpan, Token};
use shiika_core::names::UnresolvedConstName;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub struct AstBuilder {
    pub filepath: Rc<PathBuf>,
}

impl AstBuilder {
    pub fn new(filepath: &Rc<PathBuf>) -> AstBuilder {
        AstBuilder {
            filepath: filepath.clone(),
        }
    }

    pub fn empty() -> AstBuilder {
        AstBuilder {
            filepath: Rc::new(Path::new("").to_path_buf()),
        }
    }

    // TODO
    // LogicalNot {
    // LogicalAnd {
    // LogicalOr {
    // If {
    // Match {
    // While {
    // Break,
    // Return {
    // LVarAssign {
    // IVarAssign {
    // ConstAssign {
    // MethodCall {
    // LambdaExpr {
    // BareName(String),

    pub fn ivar_ref(&self, name: String, begin: Location, end: Location) -> AstExpression {
        self.primary_expression(begin, end, AstExpressionBody::IVarRef(name))
    }

    pub fn capitalized_name(
        &self,
        name: Vec<String>,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        self.primary_expression(
            begin,
            end,
            AstExpressionBody::CapitalizedName(UnresolvedConstName(name)),
        )
    }

    pub fn specialize_expr(
        &self,
        base_name: Vec<String>,
        args: Vec<AstExpression>,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        self.primary_expression(
            begin,
            end,
            AstExpressionBody::SpecializeExpression {
                base_name: UnresolvedConstName(base_name),
                args,
            },
        )
    }

    pub fn pseudo_variable(&self, token: Token, begin: Location, end: Location) -> AstExpression {
        self.primary_expression(begin, end, AstExpressionBody::PseudoVariable(token))
    }

    pub fn array_literal(
        &self,
        exprs: Vec<AstExpression>,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        self.primary_expression(begin, end, AstExpressionBody::ArrayLiteral(exprs))
    }

    pub fn float_literal(&self, value: f64, begin: Location, end: Location) -> AstExpression {
        self.primary_expression(begin, end, AstExpressionBody::FloatLiteral { value })
    }

    pub fn string_literal(&self, content: String, begin: Location, end: Location) -> AstExpression {
        self.primary_expression(begin, end, AstExpressionBody::StringLiteral { content })
    }

    pub fn decimal_literal(&self, value: i64, begin: Location, end: Location) -> AstExpression {
        self.primary_expression(begin, end, AstExpressionBody::DecimalLiteral { value })
    }

    fn primary_expression(
        &self,
        begin: Location,
        end: Location,
        body: AstExpressionBody,
    ) -> AstExpression {
        AstExpression {
            primary: true,
            body,
            locs: LocationSpan::new(&self.filepath, begin, end),
        }
    }
}
