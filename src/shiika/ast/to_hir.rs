use backtrace::Backtrace;
use crate::shiika::ast;
use crate::shiika::hir::*;

#[derive(Debug)]
pub struct Error {
    pub msg: String,
    pub backtrace: Backtrace
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}
impl std::error::Error for Error {}

impl ast::Program {
    pub fn to_hir(&self) -> Result<Hir, Error> {
        let hir_expr = self.expr.to_hir()?;
        Ok(Hir::new(hir_expr))
    }
}

impl ast::Statement {
//    fn to_hir(&self) -> Result<HirStatement, Error> {
//        match self {
//            ExpressionStatement => {
//                HirStatement::HirExpressionStatement {
//                }
//            }
//        }
//    }
}

impl ast::Expression {
    fn to_hir(&self) -> Result<HirExpression, Error> {
        match self {
            ast::Expression::If { cond_expr, then_expr, else_expr } => {
                let cond_hir = cond_expr.to_hir()?;
                let then_hir = then_expr.to_hir()?;
                let else_hir = match else_expr {
                    Some(expr) => expr.to_hir()?,
                    None => Hir::nop(),
                };
                // TODO: then and else must have conpatible type
                Ok(Hir::if_expression(
                        then_hir.ty.clone(),
                        cond_hir,
                        then_hir,
                        else_hir))
            },
            ast::Expression::FloatLiteral {value} => {
                Ok(Hir::float_literal(*value))
            },

            ast::Expression::DecimalLiteral {value} => {
                // TODO: Support Integer 
                Ok(Hir::float_literal(*value as f32))
            },

            _ => panic!("TODO: ast.to_hir for {:?}", self)
        }
    }
}

