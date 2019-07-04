use std::collections::HashMap;
use crate::ast;
use crate::error;
use crate::error::Error;
use crate::hir::*;
use crate::type_checking;

#[derive(Debug, PartialEq)]
pub struct HirMaker<'a> {
    pub sk_classes: &'a HashMap<String, SkClass>,
}

impl<'a> HirMaker<'a> {
    pub fn new(stdlib: &HashMap<String, SkClass>) -> HirMaker {
        HirMaker {
            sk_classes: stdlib,
        }
    }

    pub fn convert_program(&self, prog: ast::Program) -> Result<Hir, Error> {
        let hir_stmts = prog.stmts.iter().map(|stmt| {
            self.convert_stmt(&stmt)
        }).collect::<Result<Vec<_>, _>>()?;
        Ok(Hir::new(hir_stmts))
    }

    fn convert_stmt(&self, stmt: &ast::Statement) -> Result<HirStatement, Error> {
        match stmt {
            ast::Statement::ExpressionStatement { expr } => {
                Ok(self.convert_expr(&expr)?.to_hir_statement())
            }
        }
    }

    fn convert_expr(&self, expr: &ast::Expression) -> Result<HirExpression, Error> {
        match expr {
            ast::Expression::If { cond_expr, then_expr, else_expr } => {
                let cond_hir = self.convert_expr(cond_expr)?;
                type_checking::check_if_condition_ty(&cond_hir.ty)?;

                let then_hir = self.convert_expr(then_expr)?;
                let else_hir = match else_expr {
                    Some(expr) => self.convert_expr(expr)?,
                    None => Hir::nop(),
                };
                // TODO: then and else must have conpatible type
                Ok(Hir::if_expression(
                        then_hir.ty.clone(),
                        cond_hir,
                        then_hir,
                        else_hir))
            },

            ast::Expression::MethodCall {receiver_expr, method_name, arg_exprs} => {
                let receiver_hir =
                    match receiver_expr {
                        Some(expr) => self.convert_expr(&expr)?,
                        // Implicit self
                        _ => Hir::self_expression(), // TODO: pass current self
                    };
                // TODO: arg types must match with method signature
                let arg_hirs = arg_exprs.iter().map(|arg_expr| self.convert_expr(arg_expr)).collect::<Result<Vec<_>,_>>()?;

                self.make_method_call(receiver_hir, &method_name, arg_hirs)
            },

            ast::Expression::BinOpExpression {left, op, right} => {
                self.make_method_call(self.convert_expr(left)?, &op.method_name(), vec!(self.convert_expr(right)?))
            },

            ast::Expression::FloatLiteral {value} => {
                Ok(Hir::float_literal(*value))
            },

            ast::Expression::DecimalLiteral {value} => {
                Ok(Hir::decimal_literal(*value))
            },

            _ => panic!("TODO: convert_expr for {:?}", self)
        }
    }

    fn make_method_call(&self, receiver_hir: HirExpression, method_name: &str, arg_hirs: Vec<HirExpression>) -> Result<HirExpression, Error> {
        let method = self.lookup_method(&receiver_hir.ty, method_name)
            .ok_or(error::program_error(&format!("method {:?} not found on {:?}", method_name, receiver_hir.ty.class_fullname())))?;
        Ok(Hir::method_call(method.signature.ret_ty.clone(), receiver_hir, method.id.clone(), arg_hirs))
    }

    fn lookup_method(&self, receiver_ty: &TermTy, method_name: &str) -> Option<&SkMethod> {
        self.find_class(receiver_ty.class_fullname())
            .and_then(|sk_class| sk_class.find_method(method_name))
    }

    fn find_class(&self, fullname: &str) -> Option<&SkClass> {
        self.sk_classes.get(fullname)
    }
}
