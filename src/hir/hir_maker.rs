use crate::ast;
use crate::error;
use crate::error::Error;
use crate::hir::*;
use crate::type_checking;

#[derive(Debug, PartialEq)]
pub struct HirMaker {
    pub index: crate::hir::index::Index
}

impl HirMaker {
    pub fn new(index: crate::hir::index::Index) -> HirMaker {
        HirMaker { index }
    }

    pub fn convert_program(&mut self, prog: ast::Program) -> Result<Hir, Error> {
        let sk_classes = self.convert_toplevel_defs(&prog.toplevel_defs)?;

        let main_stmts = prog.stmts.iter().map(|stmt| {
            self.convert_stmt(&stmt)
        }).collect::<Result<Vec<_>, _>>()?;

        Ok(Hir { sk_classes, main_stmts } )
    }

    fn convert_toplevel_defs(&self, toplevel_defs: &Vec<ast::Definition>) -> Result<Vec<SkClass>, Error> {
        toplevel_defs.iter().map(|def| {
            match def {
                ast::Definition::ClassDefinition { name, defs } => {
                    self.convert_class_def(&name, &defs)
                },
                _ => panic!("should be checked in hir::index")
            }
        }).collect::<Result<Vec<_>, _>>()
    }

    fn convert_class_def(&self, name: &str, defs: &Vec<ast::Definition>) -> Result<SkClass, Error> {
        let fullname = name.to_string();
        let methods = defs.iter().map(|def| {
            match def {
                ast::Definition::InstanceMethodDefinition { name, body_stmts, .. } => {
                    self.convert_method_def(&fullname, &name, &body_stmts)
                },
                _ => panic!("TODO")
            }
        }).collect::<Result<Vec<_>, _>>()?;

        Ok(SkClass { fullname, methods })
    }

    fn convert_method_def(&self,
                          class_fullname: &str,
                          name: &str,
                          body_stmts: &Vec<ast::Statement>) -> Result<SkMethod, Error> {
        // MethodSignature is built beforehand by index::new
        let signature = self.index.find_method(class_fullname, name).unwrap().clone();
        let body = Some(SkMethodBody::ShiikaMethodBody {
            stmts: self.convert_stmts(body_stmts)?
        });

        Ok(SkMethod { signature, body })
    }

    fn convert_stmts(&self, stmts: &Vec<ast::Statement>) -> Result<Vec<HirStatement>, Error> {
        stmts.iter().map(|stmt|
            self.convert_stmt(stmt)
        ).collect::<Result<Vec<_>, _>>()
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

            _ => panic!("TODO: convert_expr for {:?}", expr)
        }
    }

    fn make_method_call(&self, receiver_hir: HirExpression, method_name: &str, arg_hirs: Vec<HirExpression>) -> Result<HirExpression, Error> {
        let sig = self.lookup_method(&receiver_hir.ty, method_name)?;

        let param_tys = arg_hirs.iter().map(|expr| &expr.ty).collect();
        type_checking::check_method_args(&sig, &param_tys)?;

        Ok(Hir::method_call(sig.ret_ty.clone(), receiver_hir, sig.fullname.clone(), arg_hirs))
    }

    fn lookup_method(&self, receiver_ty: &TermTy, method_name: &str) -> Result<&MethodSignature, Error> {
        let class_fullname = &receiver_ty.fullname;
        self.index.get(class_fullname)
            .and_then(|sk_methods| sk_methods.get(method_name))
            .ok_or(error::program_error(&format!("method {:?} not found on {:?}", method_name, class_fullname)))
    }
}
