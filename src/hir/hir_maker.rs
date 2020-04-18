use std::rc::Rc;
use crate::ast::*;
use crate::error;
use crate::error::Error;
use crate::hir;
use crate::hir::*;
use crate::hir::index::Index;
use crate::hir::hir_maker_context::*;
use crate::names;
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
    // List of ivars of the classes
    class_ivars: HashMap<ClassFullname, Rc<HashMap<String, SkIVar>>>
}

pub fn convert_program(index: index::Index, prog: ast::Program) -> Result<Hir, Error> {
    let mut hir_maker = HirMaker::new(&index);
    hir_maker.init_class_ivars();
    hir_maker.register_class_consts();
    let sk_methods =
        hir_maker.convert_toplevel_defs(&prog.toplevel_defs)?;
    let main_exprs =
        hir_maker.convert_exprs(&mut HirMakerContext::toplevel(), &prog.exprs)?;
    Ok(hir_maker.to_hir(sk_methods, main_exprs))
}

impl<'a> HirMaker<'a> {
    fn new(index: &'a crate::hir::index::Index) -> HirMaker<'a> {
        HirMaker {
            index: index,
            constants: HashMap::new(),
            const_inits: vec![],
            str_literals: vec![],
            class_ivars: HashMap::new(),
        }
    }

    fn init_class_ivars(&mut self) {
        for (name, idx_class) in self.index.classes.iter() {
            self.class_ivars.insert(
                name.clone(),
                Rc::clone(&idx_class.ivars),
            );
        }
    }

    /// Destructively convert self to Hir
    fn to_hir(&mut self,
           sk_methods: HashMap<ClassFullname, Vec<SkMethod>>,
           main_exprs: HirExpressions) -> Hir {
        let sk_classes = self.extract_classes();

        // Extract data from self
        let mut constants = HashMap::new();
        std::mem::swap(&mut constants, &mut self.constants);
        let mut str_literals = vec![];
        std::mem::swap(&mut str_literals, &mut self.str_literals);
        let mut const_inits = vec![];
        std::mem::swap(&mut const_inits, &mut self.const_inits);

        Hir {
            sk_classes,
            sk_methods,
            constants,
            str_literals,
            const_inits,
            main_exprs,
        }
    }

    fn extract_classes(&mut self) -> HashMap<ClassFullname, SkClass> {
        // TODO: Extract index
        //let mut index = Index::new();
        //std::mem::swap(&mut index, &mut self.index);

        let mut sk_classes = HashMap::new();
        self.index.classes.iter().for_each(|(name, c)| {
            let ivars = self.class_ivars.get(name)
                .expect(&format!("[BUG] ivars for class {} not found", name));
            // PERF: How to avoid these clone's? Use Rc?
            sk_classes.insert(name.clone(), SkClass {
                fullname: c.fullname.clone(),
                superclass_fullname: c.superclass_fullname.clone(),
                instance_ty: c.instance_ty.clone(),
                ivars: Rc::clone(&ivars),
                method_sigs: c.method_sigs.clone()
            });
        });
        sk_classes
    }

    fn register_class_consts(&mut self) {
        for (name, _idxclass) in &self.index.classes {
            if !name.is_meta() {
                self.register_class_const(&name);
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
                    self.collect_sk_methods(name, defs, &mut sk_methods)?;
                    Ok(())
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

    fn collect_sk_methods(&mut self,
                          firstname: &ClassFirstname,
                          defs: &Vec<ast::Definition>,
                          sk_methods: &mut HashMap<ClassFullname, Vec<SkMethod>>)
                         -> Result<(), Error> {
        let (fullname, mut instance_methods, meta_name, mut class_methods) =
            self.convert_class_def(firstname, defs)?;
        match sk_methods.get_mut(&fullname) {
            Some(imethods) => {
                // Merge methods to existing class (Class is reopened)
                imethods.append(&mut instance_methods);
                let cmethods = sk_methods.get_mut(&meta_name).expect("[BUG] meta not found");
                cmethods.append(&mut class_methods);
            },
            None => {
                sk_methods.insert(fullname, instance_methods);
                sk_methods.insert(meta_name, class_methods);
            }
        }
        Ok(())
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

        self.register_meta_ivar(&fullname);

        let mut instance_methods = vec![];
        let mut class_methods = vec![];
        let mut ctx = HirMakerContext::class_ctx(&fullname);
        let mut initialize_params = &vec![];

        match defs.iter().find(|d| d.is_initializer()) {
            Some(ast::Definition::InstanceMethodDefinition { sig, body_exprs, .. }) => {
                let method = self.convert_method_def(&mut ctx, &fullname, &sig.name, &body_exprs, true)?;
                ctx.ivars = Rc::new(collect_ivars(&method));
                self.class_ivars.insert(fullname.clone(), Rc::clone(&ctx.ivars));
                instance_methods.push(method);

                initialize_params = &sig.params;
            },
            _ => {
                // TODO: it may inherit `initialize`
                ()
            }
        };

        defs.iter().filter(|d| !d.is_initializer()).try_for_each(|def| {
            match def {
                ast::Definition::InstanceMethodDefinition { sig, body_exprs, .. } => {
                    match self.convert_method_def(&mut ctx, &fullname, &sig.name, &body_exprs, false) {
                        Ok(method) => { instance_methods.push(method); Ok(()) },
                        Err(err) => Err(err)
                    }
                },
                ast::Definition::ClassMethodDefinition { sig, body_exprs, .. } => {
                    match self.convert_method_def(&mut ctx, &meta_name, &sig.name, &body_exprs, false) {
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

        class_methods.push(self.create_new(&fullname, initialize_params)?);
        Ok((fullname, instance_methods, meta_name, class_methods))
    }

    /// Register a constant that holds a class
    fn register_class_const(&mut self, fullname: &ClassFullname) {
        let instance_ty = ty::raw(&fullname.0);
        let class_ty = instance_ty.meta_ty();
        let const_name = ConstFullname("::".to_string() + &fullname.0);

        // eg. Constant `A` holds the class A
        self.constants.insert(const_name.clone(), class_ty.clone());
        // eg. "A"
        let idx = self.register_string_literal(&fullname.0);
        // eg. A = Meta:A.new
        let op = Hir::assign_const(const_name, Hir::class_literal(fullname.clone(), idx));
        self.const_inits.push(op);
    }

    fn register_meta_ivar(&mut self, name: &ClassFullname) {
        let meta_name = name.meta_name();
        let mut meta_ivars = HashMap::new();
        meta_ivars.insert("@name".to_string(), SkIVar {
            name: "@name".to_string(),
            idx: 0,
            ty: ty::raw("String"),
            readonly: true,
        });
        self.class_ivars.insert(meta_name, Rc::new(meta_ivars));
    }

    /// Create .new
    fn create_new(&self,
                  fullname: &ClassFullname,
                  initialize_params: &Vec<ast::Param>) -> Result<SkMethod, Error> {
        let class_fullname = fullname.clone();
        let instance_ty = ty::raw(&class_fullname.0);
        let meta_name = class_fullname.meta_name();
        let (initialize_name, init_cls_name) = self.find_initialize(&fullname)?;
        let need_bitcast = init_cls_name != *fullname;
        let arity = initialize_params.len();

        Ok(SkMethod {
            signature: hir::signature_of_new(&meta_name, initialize_params, &instance_ty),
            body: SkMethodBody::RustClosureMethodBody {
                boxed_gen: Box::new(move |code_gen, function| {
                    // Allocate memory 
                    let obj = code_gen.allocate_sk_obj(&class_fullname, "addr");

                    // Call initialize
                    let initialize = code_gen.module.get_function(&initialize_name.full_name)
                        .expect(&format!("[BUG] function `{}' not found", &initialize_name));
                    let mut addr = obj;
                    if need_bitcast {
                        let ances_type = code_gen.llvm_struct_types.get(&init_cls_name)
                            .expect("ances_type not found")
                            .ptr_type(inkwell::AddressSpace::Generic);
                        addr = code_gen.builder.build_bitcast(addr, ances_type, "obj_as_super");
                    }
                    let args = (0..=arity).map(|i| {
                        if i == 0 { 
                            addr
                        }
                        else {
                            function.get_params()[i]
                        }
                    }).collect::<Vec<_>>();
                    code_gen.builder.build_call(initialize, &args, "");

                    code_gen.builder.build_return(Some(&obj));
                    Ok(())
                })
            }
        })
    }

    fn find_initialize(&self, class_fullname: &ClassFullname)
                       -> Result<(MethodFullname, ClassFullname), Error> {
        let (_sig, found_cls) =
            self.lookup_method(&class_fullname, &class_fullname, 
                               &MethodFirstname("initialize".to_string()))?;
        Ok((names::method_fullname(&found_cls, "initialize"), found_cls))
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
                          body_exprs: &Vec<AstExpression>,
                          is_initializer: bool) -> Result<SkMethod, Error> {
        // MethodSignature is built beforehand by index::new
        let err = format!("[BUG] signature not found ({}/{}/{:?})", class_fullname, name, self.index);
        let signature = self.index.find_method(class_fullname, name).expect(&err).clone();

        let mut method_ctx = HirMakerContext::method_ctx(ctx, &signature, is_initializer);
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
        if ctx.is_initializer {
            // TODO: check duplicates
            let idx = ctx.iivars.len();
            ctx.iivars.insert(name.to_string(), SkIVar {
                idx: idx,
                name: name.to_string(),
                ty: expr.ty.clone(),
                readonly: true,  // TODO: `var @foo`
            });
            return Ok(Hir::assign_ivar(name, idx, expr))
        }
        match ctx.ivars.get(name) {
            Some(ivar) => {
                if ivar.ty.equals_to(&expr.ty) {
                    Ok(Hir::assign_ivar(name, ivar.idx, expr))
                }
                else {
                    // TODO: Subtype (@obj = 1, etc.)
                    Err(error::type_error(&format!("instance variable `{}' has type {:?} but tried to assign a {:?}", name, ivar.ty, expr.ty)))
                }
            },
            None => {
                Err(error::program_error(&format!("instance variable `{}' not found", name)))
            }
        }
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
        let (sig, found_class_name) = self.lookup_method(class_fullname, class_fullname, method_name)?;

        let param_tys = arg_hirs.iter().map(|expr| &expr.ty).collect();
        type_checking::check_method_args(&sig, &param_tys)?;

        let receiver = 
            if &found_class_name != class_fullname {
                // Upcast needed
                Hir::bit_cast(found_class_name.instance_ty(), receiver_hir)
            }
            else {
                receiver_hir
            };
        Ok(Hir::method_call(sig.ret_ty.clone(), receiver, sig.fullname.clone(), arg_hirs))
    }

    fn lookup_method(&self, 
                     receiver_class_fullname: &ClassFullname,
                     class_fullname: &ClassFullname,
                     method_name: &MethodFirstname) -> Result<(&MethodSignature, ClassFullname), Error> {
        let found = self.index.find_method(class_fullname, method_name);
        if let Some(sig) = found {
            Ok((sig, class_fullname.clone()))
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
        match ctx.ivars.get(name) {
            Some(ivar) => {
                Ok(Hir::ivar_ref(ivar.ty.clone(), name.to_string(), ivar.idx))
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
                let c = ClassFullname(names.join("::"));
                if self.index.class_exists(&c.0) {
                    Ok(Hir::const_ref(c.class_ty(), fullname))
                }
                else {
                    Err(error::program_error(&format!("constant `{:?}' was not found", fullname)))
                }
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
        let idx = self.register_string_literal(content);
        Ok(Hir::string_literal(idx))
    }

    fn register_string_literal(&mut self, content: &str) -> usize {
        let idx = self.str_literals.len();
        self.str_literals.push(content.to_string());
        idx
    }
}

fn collect_ivars(method: &SkMethod) -> HashMap<String, SkIVar>
{
    let mut ivars = HashMap::new();
    match &method.body {
        SkMethodBody::ShiikaMethodBody { exprs } => {
            exprs.exprs.iter().for_each(|expr| {
                match &expr.node {
                    HirExpressionBase::HirIVarAssign { name, idx, rhs } => {
                        ivars.insert(name.to_string(), SkIVar {
                            idx: *idx,
                            name: name.to_string(),
                            ty: rhs.ty.clone(),
                            readonly: true,  // TODO: `var @foo`
                        });
                    },
                    // TODO: IVarAssign in `if'
                    _ => (),
                }
            });
            ivars
        },
        _ => HashMap::new(),
    }
}
