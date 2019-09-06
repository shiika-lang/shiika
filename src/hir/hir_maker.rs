use crate::ast;
use crate::error;
use crate::error::Error;
use crate::hir::*;
use crate::hir::index::Index;
use crate::hir::hir_maker_context::HirMakerContext;
use crate::type_checking;
use crate::parser::token::Token;

#[derive(Debug, PartialEq)]
pub struct HirMaker<'a> {
    pub index: &'a Index
}

impl<'a> HirMaker<'a> {
    fn new(index: &'a crate::hir::index::Index) -> HirMaker<'a> {
        HirMaker { index }
    }

    pub fn convert_program(index: index::Index, prog: ast::Program) -> Result<Hir, Error> {
        let hir_maker = HirMaker::new(&index);

        let sk_methods =
            hir_maker.convert_toplevel_defs(&prog.toplevel_defs)?;
        let main_exprs =
            hir_maker.convert_exprs(&HirMakerContext::toplevel(), &prog.exprs)?;
        Ok(Hir {
            sk_classes: index.sk_classes(),
            sk_methods,
            main_exprs,
        })
    }

    fn convert_toplevel_defs(&self, toplevel_defs: &Vec<ast::Definition>)
                            -> Result<HashMap<ClassFullname, Vec<SkMethod>>, Error> {
        let mut ret = HashMap::new();

        let results = toplevel_defs.iter().map(|def| {
            match def {
                ast::Definition::ClassDefinition { name, defs } => {
                    self.convert_class_def(&name, &defs)
                },
                _ => panic!("should be checked in hir::index")
            }
        }).collect::<Result<Vec<_>, _>>()?;

        results.into_iter().for_each(|methods| {
            ret.extend(methods);
        });
        Ok(ret)
    }

    /// Create SkClass and its metaclass
    fn convert_class_def(&self, name: &ClassName, defs: &Vec<ast::Definition>)
                        -> Result<HashMap<ClassFullname, Vec<SkMethod>>, Error> {
        // TODO: nested class
        let fullname = name.to_class_fullname();
        let instance_ty = ty::raw(&fullname.0);

        let instance_methods = defs.iter().filter_map(|def| {
            match def {
                ast::Definition::InstanceMethodDefinition { sig, body_exprs, .. } => {
                    Some(self.convert_method_def(&fullname, &sig.name, &body_exprs))
                },
                _ => None,
            }
        }).collect::<Result<Vec<_>, _>>()?;

        // Metaclass
        let class_ty = instance_ty.meta_ty();
        let meta_name = class_ty.fullname.clone();
        let class_methods = defs.iter().filter_map(|def| {
            match def {
                ast::Definition::ClassMethodDefinition { sig, body_exprs, .. } => {
                    Some(self.convert_method_def(&meta_name, &sig.name, &body_exprs))
                },
                _ => None,
            }
        }).collect::<Result<Vec<_>, _>>()?;

        let mut ret = HashMap::new();
        ret.insert(fullname, instance_methods);
        ret.insert(meta_name, class_methods);
        Ok(ret)
    }

    fn convert_method_def(&self,
                          class_fullname: &ClassFullname,
                          name: &MethodName,
                          body_exprs: &Vec<ast::Expression>) -> Result<SkMethod, Error> {
        // MethodSignature is built beforehand by index::new
        let err = format!("[BUG] signature not found ({}/{}/{:?})", class_fullname, name, self.index);
        let signature = self.index.find_method(class_fullname, name).expect(&err).clone();

        let ctx = HirMakerContext {
            method_sig: signature.clone(),
            self_ty: ty::raw(&class_fullname.0),
        };
        let body_exprs = self.convert_exprs(&ctx, body_exprs)?;
        type_checking::check_return_value(&signature, &body_exprs.ty)?;

        let body = SkMethodBody::ShiikaMethodBody { exprs: body_exprs };

        Ok(SkMethod { signature, body })
    }

    fn convert_exprs(&self,
                     ctx: &HirMakerContext,
                     exprs: &Vec<ast::Expression>) -> Result<HirExpressions, Error> {
        let hir_exprs = exprs.iter().map(|expr|
            self.convert_expr(ctx, expr)
        ).collect::<Result<Vec<_>, _>>()?;

        let ty = match hir_exprs.last() {
                   Some(hir_expr) => hir_expr.ty.clone(),
                   None => ty::raw("Void"),
                 };

        Ok(HirExpressions { ty: ty, exprs: hir_exprs })
    }

    fn convert_expr(&self,
                    ctx: &HirMakerContext,
                    expr: &ast::Expression) -> Result<HirExpression, Error> {
        match &expr.body {
            ast::ExpressionBody::If { cond_expr, then_expr, else_expr } => {
                let cond_hir = self.convert_expr(ctx, cond_expr)?;
                type_checking::check_if_condition_ty(&cond_hir.ty)?;

                let then_hir = self.convert_expr(ctx, then_expr)?;
                let else_hir = match else_expr {
                    Some(expr) => self.convert_expr(ctx, expr)?,
                    None => Hir::nop(),
                };
                // TODO: then and else must have conpatible type
                Ok(Hir::if_expression(
                        then_hir.ty.clone(),
                        cond_hir,
                        then_hir,
                        else_hir))
            },

            ast::ExpressionBody::MethodCall {receiver_expr, method_name, arg_exprs, .. } => {
                let receiver_hir =
                    match receiver_expr {
                        Some(expr) => self.convert_expr(ctx, &expr)?,
                        // Implicit self
                        _ => self.convert_self_expr(ctx)?,
                    };
                // TODO: arg types must match with method signature
                let arg_hirs = arg_exprs.iter().map(|arg_expr| self.convert_expr(ctx, arg_expr)).collect::<Result<Vec<_>,_>>()?;

                self.make_method_call(receiver_hir, &method_name, arg_hirs)
            },

            ast::ExpressionBody::BareName(name) => {
                self.convert_bare_name(ctx, name)
            },

            ast::ExpressionBody::ConstRef(name) => {
                self.convert_const_ref(ctx, name)
            },

            ast::ExpressionBody::PseudoVariable(token) => {
                self.convert_pseudo_variable(ctx, token)
            },

            ast::ExpressionBody::FloatLiteral {value} => {
                Ok(Hir::float_literal(*value))
            },

            ast::ExpressionBody::DecimalLiteral {value} => {
                Ok(Hir::decimal_literal(*value))
            },

            x => panic!("TODO: {:?}", x)
        }
    }

    fn convert_pseudo_variable(&self,
                               ctx: &HirMakerContext,
                               token: &Token) -> Result<HirExpression, Error> {
        match token {
            Token::KwSelf => {
                self.convert_self_expr(ctx)
            },
            Token::KwTrue => {
                Ok(Hir::boolean_literal(true))
            },
            Token::KwFalse => {
                Ok(Hir::boolean_literal(false))
            },
            _ => panic!("[BUG] not a pseudo variable token: {:?}", token)
        }
    }

    fn convert_self_expr(&self, ctx: &HirMakerContext) -> Result<HirExpression, Error> {
        Ok(Hir::self_expression(ctx.self_ty.clone()))
    }

    /// Generate local variable reference or method call with implicit receiver(self)
    fn convert_bare_name(&self,
                         ctx: &HirMakerContext,
                         name: &str) -> Result<HirExpression, Error> {
        match &ctx.method_sig.find_param(name) {
            Some((idx, param)) => {
                Ok(Hir::hir_arg_ref(param.ty.clone(), *idx))
            },
            None => {
                Err(error::program_error(&format!("variable `{}' was not found", name)))
            }
        }
    }

    /// Resolve constant name
    fn convert_const_ref(&self,
                         _ctx: &HirMakerContext,
                         name: &str) -> Result<HirExpression, Error> {
        // TODO: nested class, constants
        if self.index.class_exists(&name) {
            let ty = ty::meta(name);
            Ok(Hir::const_ref(ty, ConstFullname(name.to_string())))
        }
        else {
            Err(error::program_error(&format!("constant `{}' was not found", name)))
        }
    }

    fn make_method_call(&self, receiver_hir: HirExpression, method_name: &MethodName, arg_hirs: Vec<HirExpression>) -> Result<HirExpression, Error> {
        let sig = self.lookup_method(&receiver_hir.ty, method_name)?;

        let param_tys = arg_hirs.iter().map(|expr| &expr.ty).collect();
        type_checking::check_method_args(&sig, &param_tys)?;

        Ok(Hir::method_call(sig.ret_ty.clone(), receiver_hir, sig.fullname.clone(), arg_hirs))
    }

    fn lookup_method(&self, receiver_ty: &TermTy, method_name: &MethodName) -> Result<&MethodSignature, Error> {
        let class_fullname = &receiver_ty.fullname;
        self.index.find_method(class_fullname, method_name)
            .ok_or(error::program_error(&format!("method {:?} not found on {:?}", method_name, class_fullname)))
    }
}
