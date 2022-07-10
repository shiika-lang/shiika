use shiika_ast::{AstExpression, AstExpressionBody, Location, LocationSpan};
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub struct AstBuilder {
    filepath: Rc<PathBuf>,
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
