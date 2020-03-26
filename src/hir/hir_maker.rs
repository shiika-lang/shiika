use crate::ast::*;
use crate::error;
use crate::error::Error;
use crate::hir::*;
use crate::hir::index::Index;
use crate::hir::hir_maker_context::*;
use crate::type_checking;
use crate::parser::token::Token;

#[derive(Debug, PartialEq)]
pub struct HirMaker<'a> {
    pub index: &'a Index,
    // List of constants found so far
    pub constants: HashMap<ConstFullname, TermTy>,
    pub const_inits: Vec<HirExpression>,
    // List of string literals found so far
    pub str_literals: Vec<String>,
}

impl<'a> HirMaker<'a> {
    fn new(index: &'a crate::hir::index::Index) -> HirMaker<'a> {
        let mut constants = HashMap::new();
        constants.insert(ConstFullname("::Void".to_string()), ty::raw("Void"));
        HirMaker {
            index: index,
            constants: constants,
            const_inits: vec![],
            str_literals: vec![],
        }
    }

    pub fn convert_program(index: index::Index, prog: ast::Program) -> Result<Hir, Error> {
        let mut hir_maker = HirMaker::new(&index);

        let sk_methods =
            hir_maker.convert_toplevel_defs(&prog.toplevel_defs)?;
        let mut main_exprs =
            hir_maker.convert_exprs(&mut HirMakerContext::toplevel(), &prog.exprs)?;
        match hir_maker {
            HirMaker { index, constants, mut const_inits, str_literals } => {
                let sk_classes = HashMap::new();
                index.classes.into_iter().for_each(|(name, c)| {
                    sk_classes.insert(name, SkClass {
                        fullname: c.fullname,
                        superclass_fullname: c.superclass_fullname,
                        instance_ty: c.instance_ty,
                        ivars: HashMap::new(), // TODO
                        method_sigs: c.method_sigs
                    });
                });

                const_inits.append(&mut main_exprs.exprs);
                Ok(Hir {
                    sk_classes,
                    sk_methods,
                    constants,
                    str_literals,
                    //str_literals,
                    main_exprs:  HirExpressions {
                        ty: main_exprs.ty,
                        exprs: const_inits,
                    }
                })
            }
        }
    }

    fn convert_toplevel_defs(&mut self, toplevel_defs: &Vec<ast::Definition>)
                            -> Result<HashMap<ClassFullname, Vec<SkMethod>>, Error> {
        let mut sk_methods = HashMap::new();
        let mut ctx = HirMakerContext::toplevel();

        toplevel_defs.iter().try_for_each(|def|
            match def {
                // Extract instance/class methods
                ast::Definition::ClassDefinition { name, defs } => {
                    match self.convert_class_def(&name, &defs) {
                        Ok((fullname, instance_methods, meta_name, class_methods)) => {
                            sk_methods.insert(fullname, instance_methods);
                            sk_methods.insert(meta_name, class_methods);
                            Ok(())
                        },
                        Err(err) => Err(err)
                    }
                },
                ast::Definition::ConstDefinition { name, expr } => {
                    self.register_const(&mut ctx, name, expr)?;
                    Ok(())
                }
                _ => panic!("should be checked in hir::index")
            }
        )?;

        Ok(sk_methods)
    }

    /// Extract instance/class methods and constants
    fn convert_class_def(&mut self, name: &ClassFirstname, defs: &Vec<ast::Definition>)
                        -> Result<(ClassFullname, Vec<SkMethod>,
                                   ClassFullname, Vec<SkMethod>), Error> {
        // TODO: nested class
        let fullname = name.to_class_fullname();
        let instance_ty = ty::raw(&fullname.0);
        let class_ty = instance_ty.meta_ty();
        let meta_name = class_ty.fullname.clone();

        self.register_class_const(&fullname);

        let mut instance_methods = vec![];
        let mut class_methods = vec![];
        let mut ctx = HirMakerContext::class_ctx(&fullname);

        defs.iter().try_for_each(|def| {
            match def {
                ast::Definition::InstanceMethodDefinition { sig, body_exprs, .. } => {
                    match self.convert_method_def(&mut ctx, &fullname, &sig.name, &body_exprs) {
                        Ok(method) => { instance_methods.push(method); Ok(()) },
                        Err(err) => Err(err)
                    }
                },
                ast::Definition::ClassMethodDefinition { sig, body_exprs, .. } => {
                    match self.convert_method_def(&mut ctx, &meta_name, &sig.name, &body_exprs) {
                        Ok(method) => { class_methods.push(method); Ok(()) },
                        Err(err) => Err(err)
                    }
                },
                ast::Definition::ConstDefinition { name, expr } => {
                    self.register_const(&mut ctx, name, expr)?;
                    Ok(())
                }
                _ => Ok(()),
            }
        })?;

        class_methods.push(self.create_new(&fullname));
        Ok((fullname, instance_methods, meta_name, class_methods))
    }

    /// Register a constant that holds a class
    fn register_class_const(&mut self, fullname: &ClassFullname) {
        let instance_ty = ty::raw(&fullname.0);
        let class_ty = instance_ty.meta_ty();
        let const_name = ConstFullname("::".to_string() + &fullname.0);

        // eg. Constant `A` holds the class A
        self.constants.insert(const_name.clone(), class_ty.clone());
        // eg. A = Meta:A.new
        let op = Hir::assign_const(const_name, Hir::class_literal(fullname.clone()));
        self.const_inits.push(op);
    }

    /// Create .new
    fn create_new(&self, fullname: &ClassFullname) -> SkMethod {
        let class_fullname = fullname.clone();
        let instance_ty = ty::raw(&class_fullname.0);
        let class_ty = instance_ty.meta_ty();
        let meta_name = class_ty.fullname;

        SkMethod {
            signature: signature_of_new(&meta_name, &instance_ty),
            body: SkMethodBody::RustClosureMethodBody {
                boxed_gen: Box::new(move |code_gen, _| {
                    let addr = code_gen.allocate_sk_obj(&class_fullname, "obj");
                    code_gen.builder.build_return(Some(&addr));
                    Ok(())
                })
            }
        }
    }

    /// Register a constant
    fn register_const(&mut self,
                      ctx: &mut HirMakerContext,
                      name: &ConstFirstname,
                      expr: &AstExpression) -> Result<ConstFullname, Error> {
        // TODO: resolve name using ctx
        let fullname = ConstFullname(ctx.namespace.0.clone() + "::" + &name.0);
        let hir_expr = self.convert_expr(ctx, expr)?;
        self.constants.insert(fullname.clone(), hir_expr.ty.clone());
        let op = Hir::assign_const(fullname.clone(), hir_expr);
        self.const_inits.push(op);
        Ok(fullname)
    }


    fn convert_method_def(&mut self,
                          ctx: &HirMakerContext,
                          class_fullname: &ClassFullname,
                          name: &MethodFirstname,
                          body_exprs: &Vec<AstExpression>) -> Result<SkMethod, Error> {
        // MethodSignature is built beforehand by index::new
        let err = format!("[BUG] signature not found ({}/{}/{:?})", class_fullname, name, self.index);
        let signature = self.index.find_method(class_fullname, name).expect(&err).clone();

        let mut method_ctx = HirMakerContext::method_ctx(ctx, &signature);
        let body_exprs = self.convert_exprs(&mut method_ctx, body_exprs)?;
        type_checking::check_return_value(&signature, &body_exprs.ty)?;

        let body = SkMethodBody::ShiikaMethodBody { exprs: body_exprs };

        Ok(SkMethod { signature, body })
    }

    fn convert_exprs(&mut self,
                     ctx: &mut HirMakerContext,
                     exprs: &Vec<AstExpression>) -> Result<HirExpressions, Error> {
        let mut hir_exprs = exprs.iter().map(|expr|
            self.convert_expr(ctx, expr)
        ).collect::<Result<Vec<_>, _>>()?;

        if hir_exprs.is_empty() {
            hir_exprs.push(Hir::const_ref(ty::raw("Void"), ConstFullname("::Void".to_string())))
        }

        let last_expr = hir_exprs.last().unwrap();
        let ty = last_expr.ty.clone();

        Ok(HirExpressions { ty: ty, exprs: hir_exprs })
    }

    fn convert_expr(&mut self,
                    ctx: &mut HirMakerContext,
                    expr: &AstExpression) -> Result<HirExpression, Error> {
        match &expr.body {
            AstExpressionBody::If { cond_expr, then_exprs, else_exprs } => {
                self.convert_if_expr(ctx, cond_expr, then_exprs, else_exprs)
            },

            AstExpressionBody::While { cond_expr, body_exprs } => {
                self.convert_while_expr(ctx, cond_expr, body_exprs)
            },

            AstExpressionBody::Break => {
                self.convert_break_expr()
            },

            AstExpressionBody::LVarAssign { name, rhs, is_var } => {
                self.convert_lvar_assign(ctx, name, &*rhs, is_var)
            }

            AstExpressionBody::IVarAssign { name, rhs, is_var } => {
                self.convert_ivar_assign(ctx, name, &*rhs, is_var)
            }

            AstExpressionBody::ConstAssign { names, rhs } => {
                self.convert_const_assign(ctx, names, &*rhs)
            },

            AstExpressionBody::MethodCall {receiver_expr, method_name, arg_exprs, .. } => {
                self.convert_method_call(ctx, receiver_expr, method_name, arg_exprs)
            },

            AstExpressionBody::BareName(name) => {
                self.convert_bare_name(ctx, name)
            },

            AstExpressionBody::IVarRef(names) => {
                self.convert_ivar_ref(ctx, names)
            },

            AstExpressionBody::ConstRef(names) => {
                self.convert_const_ref(ctx, names)
            },

            AstExpressionBody::PseudoVariable(token) => {
                self.convert_pseudo_variable(ctx, token)
            },

            AstExpressionBody::FloatLiteral {value} => {
                Ok(Hir::float_literal(*value))
            },

            AstExpressionBody::DecimalLiteral {value} => {
                Ok(Hir::decimal_literal(*value))
            },

            AstExpressionBody::StringLiteral {content} => {
                self.convert_string_literal(content)
            },

            x => panic!("TODO: {:?}", x)
        }
    }

    fn convert_if_expr(&mut self,
                       ctx: &mut HirMakerContext,
                       cond_expr: &AstExpression,
                       then_exprs: &Vec<AstExpression>,
                       else_exprs: &Option<Vec<AstExpression>>) -> Result<HirExpression, Error> {
        let cond_hir = self.convert_expr(ctx, cond_expr)?;
        type_checking::check_condition_ty(&cond_hir.ty, "if")?;

        let then_hirs = self.convert_exprs(ctx, then_exprs)?;
        let else_hirs = match else_exprs {
            Some(exprs) => Some(self.convert_exprs(ctx, exprs)?),
            None => None,
        };
        // TODO: then and else must have conpatible type
        Ok(Hir::if_expression(
                then_hirs.ty.clone(),
                cond_hir,
                then_hirs,
                else_hirs))
    }

    fn convert_while_expr(&mut self,
                          ctx: &mut HirMakerContext,
                          cond_expr: &AstExpression,
                          body_exprs: &Vec<AstExpression>) -> Result<HirExpression, Error> {
        let cond_hir = self.convert_expr(ctx, cond_expr)?;
        type_checking::check_condition_ty(&cond_hir.ty, "while")?;

        let body_hirs = self.convert_exprs(ctx, body_exprs)?;
        Ok(Hir::while_expression(cond_hir, body_hirs))
    }

    fn convert_break_expr(&mut self) -> Result<HirExpression, Error> {
        Ok(Hir::break_expression())
    }

    fn convert_lvar_assign(&mut self,
                            ctx: &mut HirMakerContext,
                            name: &str,
                            rhs: &AstExpression,
                            is_var: &bool) -> Result<HirExpression, Error> {
        let expr = self.convert_expr(ctx, rhs)?;
        match ctx.lvars.get(name) {
            Some(lvar) => {
                // Reassigning
                if lvar.readonly {
                    return Err(error::program_error(&format!(
                      "cannot reassign to {} (Hint: declare it with `var')", name)))
                }
                else {
                    if *is_var {
                        return Err(error::program_error(&format!("variable `{}' already exists", name)))
                    }
                    else {
                        type_checking::check_reassign_var(&lvar.ty, &expr.ty, name)?;
                    }
                }
            },
            None => {
                // Newly introduced lvar
                ctx.lvars.insert(name.to_string(), CtxLVar {
                    name: name.to_string(),
                    ty: expr.ty.clone(),
                    readonly: !is_var,
                });
            }
        }

        Ok(Hir::assign_lvar(name, expr))
    }

    fn convert_ivar_assign(&mut self,
                            ctx: &mut HirMakerContext,
                            name: &str,
                            rhs: &AstExpression,
                            _is_var: &bool) -> Result<HirExpression, Error> {
        let expr = self.convert_expr(ctx, rhs)?;
        if self.index.find_ivar(&ctx.self_ty, name).is_none() {
            return Err(error::program_error(&format!("instance variable `{}' not found", name)))
        }

        Ok(Hir::assign_ivar(name, expr))
    }

    fn convert_const_assign(&mut self,
                            ctx: &mut HirMakerContext,
                            names: &Vec<String>,
                            rhs: &AstExpression) -> Result<HirExpression, Error> {
        let name = ConstFirstname(names.join("::")); // TODO: pass entire `names` rather than ConstFirstname?
        let fullname = self.register_const(ctx, &name, &rhs)?;
        Ok(Hir::assign_const(fullname, self.convert_expr(ctx, rhs)?))
    }

    fn convert_method_call(&mut self,
                            ctx: &mut HirMakerContext,
                            receiver_expr: &Option<Box<AstExpression>>,
                            method_name: &MethodFirstname,
                            arg_exprs: &Vec<AstExpression>) -> Result<HirExpression, Error> {
        let receiver_hir =
            match receiver_expr {
                Some(expr) => self.convert_expr(ctx, &expr)?,
                // Implicit self
                _ => self.convert_self_expr(ctx)?,
            };
        // TODO: arg types must match with method signature
        let arg_hirs = arg_exprs.iter().map(|arg_expr| self.convert_expr(ctx, arg_expr)).collect::<Result<Vec<_>,_>>()?;

        self.make_method_call(receiver_hir, &method_name, arg_hirs)
    }

    fn make_method_call(&self, receiver_hir: HirExpression, method_name: &MethodFirstname, arg_hirs: Vec<HirExpression>) -> Result<HirExpression, Error> {
        let class_fullname = &receiver_hir.ty.fullname;
        let sig = self.lookup_method(class_fullname, class_fullname, method_name)?;

        let param_tys = arg_hirs.iter().map(|expr| &expr.ty).collect();
        type_checking::check_method_args(&sig, &param_tys)?;

        Ok(Hir::method_call(sig.ret_ty.clone(), receiver_hir, sig.fullname.clone(), arg_hirs))
    }

    fn lookup_method(&self, 
                     receiver_class_fullname: &ClassFullname,
                     class_fullname: &ClassFullname,
                     method_name: &MethodFirstname) -> Result<&MethodSignature, Error> {
        let found = self.index.find_method(class_fullname, method_name);
        if let Some(sig) = found {
            Ok(sig)
        }
        else {
            // Look up in superclass
            let sk_class = self.index.find_class(class_fullname)
                .expect("[BUG] lookup_method: class not found");
            if let Some(super_name) = &sk_class.superclass_fullname {
                self.lookup_method(receiver_class_fullname, super_name, method_name)
            }
            else {
                Err(error::program_error(&format!("method {:?} not found on {:?}", method_name, receiver_class_fullname)))
            }
        }
    }

    /// Generate local variable reference or method call with implicit receiver(self)
    fn convert_bare_name(&self,
                         ctx: &HirMakerContext,
                         name: &str) -> Result<HirExpression, Error> {
        // It is a local variable
        if let Some(lvar) = ctx.lvars.get(name) {
            return Ok(Hir::lvar_ref(lvar.ty.clone(), name.to_string()))
        }
        // It is a method parameter
        let method_sig = match &ctx.method_sig {
            Some(x) => x,
            None => return Err(error::program_error(&format!("variable not found: `{}'", name)))
        };
        match &method_sig.find_param(name) {
            Some((idx, param)) => {
                Ok(Hir::hir_arg_ref(param.ty.clone(), *idx))
            },
            None => {
                Err(error::program_error(&format!("variable `{}' was not found", name)))
            }
        }
        // TODO: It may be a nullary method call
    }

    fn convert_ivar_ref(&self,
                        ctx: &HirMakerContext,
                        name: &str) -> Result<HirExpression, Error> {
        match self.index.find_ivar(&ctx.self_ty, name) {
            Some(ivar) => {
                Ok(Hir::ivar_ref(ivar.ty.clone(), name.to_string()))
            },
            None => {
                Err(error::program_error(&format!("ivar `{}' was not found", name)))
            }
        }
    }

    /// Resolve constant name
    fn convert_const_ref(&self,
                         _ctx: &HirMakerContext,
                         names: &Vec<String>) -> Result<HirExpression, Error> {
        // TODO: Resolve using ctx
        let fullname = ConstFullname("::".to_string() + &names.join("::"));
        match self.constants.get(&fullname) {
            Some(ty) => {
                Ok(Hir::const_ref(ty.clone(), fullname))
            },
            None => {
                Err(error::program_error(&format!("constant `{:?}' was not found", names)))
            }
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

    fn convert_string_literal(&mut self, content: &str) -> Result<HirExpression, Error> {
        let idx = self.str_literals.len();
        self.str_literals.push(content.to_string());
        Ok(Hir::string_literal(idx))
    }
}
