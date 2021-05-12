use crate::ast::*;
use crate::code_gen::CodeGen;
use crate::error::Error;
use crate::hir::class_dict::ClassDict;
use crate::hir::hir_maker_context::*;
use crate::hir::method_dict::MethodDict;
use crate::hir::*;
use crate::library::LibraryExports;
use crate::names;
use crate::type_checking;

#[derive(Debug)]
pub struct HirMaker<'hir_maker> {
    /// List of classes found so far
    pub(super) class_dict: ClassDict<'hir_maker>,
    /// List of methods found so far
    pub(super) method_dict: MethodDict,
    /// List of constants found so far
    pub(super) constants: HashMap<ConstFullname, TermTy>,
    /// Constants defined from other library
    pub(super) imported_constants: &'hir_maker HashMap<ConstFullname, TermTy>,
    /// Expressions that initialize constants
    pub(super) const_inits: Vec<HirExpression>,
    /// List of string literals found so far
    pub(super) str_literals: Vec<String>,
    /// Contextual information
    pub(super) ctx: HirMakerContext,
    /// Counter to give unique name for lambdas
    pub(super) lambda_ct: usize,
}

pub fn make_hir(
    ast: ast::Program,
    corelib: Option<Corelib>,
    imports: &LibraryExports,
) -> Result<Hir, Error> {
    let (core_classes, core_methods) = if let Some(c) = corelib {
        (c.sk_classes, c.sk_methods)
    } else {
        (Default::default(), Default::default())
    };
    let class_dict = class_dict::create(&ast, core_classes, &imports.sk_classes)?;
    let mut hir = convert_program(class_dict, &imports.constants, ast)?;

    // While corelib classes are included in `class_dict`,
    // corelib methods are not. Here we need to add them manually
    hir.add_methods(core_methods);

    Ok(hir)
}

fn convert_program(
    class_dict: ClassDict,
    imported_constants: &HashMap<ConstFullname, TermTy>,
    prog: ast::Program,
) -> Result<Hir, Error> {
    let mut hir_maker = HirMaker::new(class_dict, imported_constants);
    let (main_exprs, main_lvars) = hir_maker.convert_toplevel_items(&prog.toplevel_items)?;
    Ok(hir_maker.extract_hir(main_exprs, main_lvars))
}

impl<'hir_maker> HirMaker<'hir_maker> {
    fn new(
        class_dict: ClassDict<'hir_maker>,
        imported_constants: &'hir_maker HashMap<ConstFullname, TermTy>,
    ) -> HirMaker<'hir_maker> {
        HirMaker {
            class_dict,
            method_dict: MethodDict::new(),
            constants: HashMap::new(),
            imported_constants,
            const_inits: vec![],
            str_literals: vec![],
            ctx: HirMakerContext::new(),
            lambda_ct: 0,
        }
    }

    /// Destructively convert self to Hir
    fn extract_hir(&mut self, main_exprs: HirExpressions, main_lvars: HirLVars) -> Hir {
        // Extract data from self
        let sk_classes = std::mem::replace(&mut self.class_dict.sk_classes, HashMap::new());
        let sk_methods = std::mem::take(&mut self.method_dict.sk_methods);
        let mut constants = HashMap::new();
        std::mem::swap(&mut constants, &mut self.constants);
        let mut str_literals = vec![];
        std::mem::swap(&mut str_literals, &mut self.str_literals);
        let mut const_inits = vec![];
        std::mem::swap(&mut const_inits, &mut self.const_inits);

        // Register void
        constants.insert(toplevel_const("Void"), ty::raw("Void"));

        Hir {
            sk_classes,
            sk_methods,
            constants,
            str_literals,
            const_inits,
            main_exprs,
            main_lvars,
        }
    }

    fn convert_toplevel_items(
        &mut self,
        items: &[ast::TopLevelItem],
    ) -> Result<(HirExpressions, HirLVars), Error> {
        let mut main_exprs = vec![];
        for item in items {
            match item {
                ast::TopLevelItem::Def(def) => {
                    self.process_toplevel_def(&def)?;
                }
                ast::TopLevelItem::Expr(expr) => {
                    main_exprs.push(self.convert_expr(&expr)?);
                }
            }
        }
        Ok((
            HirExpressions::new(main_exprs),
            extract_lvars(&mut self.ctx.toplevel.lvars),
        ))
    }

    fn process_toplevel_def(&mut self, def: &ast::Definition) -> Result<(), Error> {
        match def {
            // Extract instance/class methods
            ast::Definition::ClassDefinition {
                name,
                typarams,
                defs,
                ..
            } => {
                let full = name.add_namespace("");
                self.process_defs_in_class(&full, typarams.clone(), defs)?;
            }
            ast::Definition::ConstDefinition { name, expr } => {
                self.register_toplevel_const(name, expr)?;
            }
            _ => panic!("should be checked in hir::class_dict"),
        }
        Ok(())
    }

    /// Process each method def and const def
    fn process_defs_in_class(
        &mut self,
        fullname: &ClassFullname,
        typarams: Vec<String>,
        defs: &[ast::Definition],
    ) -> Result<(), Error> {
        let meta_name = fullname.meta_name();
        let mut current = CtxKind::Class;
        self.ctx.swap_current(&mut current);
        self.ctx
            .classes
            .push(ClassCtx::new(fullname.clone(), typarams));

        // Register constants before processing #initialize
        self._process_const_defs_in_class(defs, fullname)?;

        // Register #initialize and ivars
        let own_ivars =
            self._process_initialize(fullname, defs.iter().find(|d| d.is_initializer()))?;
        if !own_ivars.is_empty() {
            // Be careful not to reset ivars of corelib/* by builtin/*
            self.class_dict.define_ivars(fullname, own_ivars.clone())?;
            self.define_accessors(fullname, own_ivars, defs);
        }

        // Register .new
        self.method_dict
            .add_method(&meta_name, self.create_new(&fullname)?);

        for def in defs {
            match def {
                ast::Definition::InstanceMethodDefinition {
                    sig, body_exprs, ..
                } => {
                    if def.is_initializer() {
                        // Already processed above
                    } else {
                        log::trace!("method {}#{}", &fullname, &sig.name);
                        let method = self.convert_method_def(&fullname, &sig.name, &body_exprs)?;
                        self.method_dict.add_method(&fullname, method);
                    }
                }
                ast::Definition::ClassMethodDefinition {
                    sig, body_exprs, ..
                } => {
                    log::trace!("method {}.{}", &fullname, &sig.name);
                    let method = self.convert_method_def(&meta_name, &sig.name, &body_exprs)?;
                    self.method_dict.add_method(&meta_name, method);
                }
                ast::Definition::ConstDefinition { .. } => {
                    // Already processed above
                }
                ast::Definition::ClassDefinition {
                    name,
                    defs,
                    typarams,
                    ..
                } => {
                    let full = name.add_namespace(&fullname.0);
                    self.process_defs_in_class(&full, typarams.clone(), defs)?;
                }
            }
        }
        self.ctx.classes.pop();
        self.ctx.swap_current(&mut current);
        Ok(())
    }

    /// Add `#initialize` and return defined ivars
    fn _process_initialize(
        &mut self,
        fullname: &ClassFullname,
        initialize: Option<&ast::Definition>,
    ) -> Result<SkIVars, Error> {
        let mut own_ivars = HashMap::default();
        if let Some(ast::Definition::InstanceMethodDefinition {
            sig, body_exprs, ..
        }) = initialize
        {
            log::trace!("method {}#initialize", &fullname);
            let (sk_method, found_ivars) =
                self.create_initialize(&fullname, &sig.name, &body_exprs)?;
            self.method_dict.add_method(&fullname, sk_method);
            own_ivars = found_ivars;
        }
        Ok(own_ivars)
    }

    /// Register constants defined in a class
    fn _process_const_defs_in_class(
        &mut self,
        defs: &[ast::Definition],
        fullname: &ClassFullname,
    ) -> Result<(), Error> {
        for def in defs {
            if let ast::Definition::ConstDefinition { name, expr } = def {
                // FIXME: works for A::B but not for A::B::C
                let full = const_fullname(&format!("::{}::{}", fullname.0, name));
                let hir_expr = self.convert_expr(expr)?;
                self.register_const_full(full, hir_expr);
            }
        }
        Ok(())
    }

    /// Create the `initialize` method
    /// Also, define ivars
    fn create_initialize(
        &mut self,
        class_fullname: &ClassFullname,
        name: &MethodFirstname,
        body_exprs: &[AstExpression],
    ) -> Result<(SkMethod, SkIVars), Error> {
        let super_ivars = self
            .class_dict
            .get_superclass(class_fullname)
            .map(|super_cls| super_cls.ivars.clone());
        self.convert_method_def_(class_fullname, name, body_exprs, super_ivars)
    }

    /// Create .new
    fn create_new(&self, class_fullname: &ClassFullname) -> Result<SkMethod, Error> {
        let (initialize_name, init_cls_name) =
            self._find_initialize(&class_fullname.instance_ty())?;
        let (signature, _) = self.class_dict.lookup_method(
            &class_fullname.class_ty(),
            &method_firstname("new"),
            &[],
        )?;
        let arity = signature.params.len();
        let classname = class_fullname.clone();

        let new_body = move |code_gen: &CodeGen, function: &inkwell::values::FunctionValue| {
            code_gen.gen_body_of_new(
                function,
                &classname,
                &initialize_name,
                &init_cls_name,
                arity,
            );
            Ok(())
        };

        Ok(SkMethod {
            signature,
            body: SkMethodBody::RustClosureMethodBody {
                boxed_gen: Box::new(new_body),
            },
            lvars: vec![],
        })
    }

    /// Find actual `initialize` func to call from `.new`
    fn _find_initialize(&self, class: &TermTy) -> Result<(MethodFullname, ClassFullname), Error> {
        let (_, found_cls) =
            self.class_dict
                .lookup_method(&class, &method_firstname("initialize"), &[])?;
        Ok((names::method_fullname(&found_cls, "initialize"), found_cls))
    }

    /// Register a constant defined in the toplevel
    pub(super) fn register_toplevel_const(
        &mut self,
        name: &str,
        expr: &AstExpression,
    ) -> Result<(), Error> {
        let hir_expr = self.convert_expr(expr)?;
        self.constants.insert(toplevel_const(name), hir_expr.ty.clone());
        let op = Hir::const_assign(toplevel_const(name), hir_expr);
        self.const_inits.push(op);
        Ok(())
    }

    /// Register a constant
    pub(super) fn register_const_full(
        &mut self,
        fullname: ConstFullname,
        hir_expr: HirExpression,
    ) {
        self.constants.insert(fullname.clone(), hir_expr.ty.clone());
        let op = Hir::const_assign(fullname.clone(), hir_expr);
        self.const_inits.push(op);
    }

    fn convert_method_def(
        &mut self,
        class_fullname: &ClassFullname,
        name: &MethodFirstname,
        body_exprs: &[AstExpression],
    ) -> Result<SkMethod, Error> {
        let (sk_method, _ivars) =
            self.convert_method_def_(class_fullname, name, body_exprs, None)?;
        Ok(sk_method)
    }

    /// Create a SkMethod and return it with ctx.iivars
    fn convert_method_def_(
        &mut self,
        class_fullname: &ClassFullname,
        name: &MethodFirstname,
        body_exprs: &[AstExpression],
        super_ivars: Option<SkIVars>,
    ) -> Result<(SkMethod, HashMap<String, SkIVar>), Error> {
        // MethodSignature is built beforehand by class_dict::new
        let err = format!(
            "[BUG] signature not found ({}/{}/{:?})",
            class_fullname, name, self.class_dict
        );
        let signature = self
            .class_dict
            .find_method(class_fullname, name)
            .expect(&err)
            .clone();

        self.ctx.method = Some(MethodCtx::new(signature.clone(), super_ivars));

        let mut current = CtxKind::Method;
        self.ctx.swap_current(&mut current);
        let mut hir_exprs = self.convert_exprs(body_exprs)?;
        // Insert ::Void so that last expr always matches to ret_ty
        if signature.ret_ty.is_void_type() {
            hir_exprs.voidify();
        }
        self.ctx.swap_current(&mut current);

        let mut method_ctx = self.ctx.method.take().unwrap();
        let lvars = extract_lvars(&mut method_ctx.lvars);
        type_checking::check_return_value(&self.class_dict, &signature, &hir_exprs.ty)?;

        let body = SkMethodBody::ShiikaMethodBody { exprs: hir_exprs };
        Ok((
            SkMethod {
                signature,
                body,
                lvars,
            },
            method_ctx.iivars,
        ))
    }
}
