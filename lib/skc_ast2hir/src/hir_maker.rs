use crate::class_dict::ClassDict;
use crate::ctx_stack::CtxStack;
use crate::error;
use crate::hir_maker_context::*;
use crate::method_dict::MethodDict;
use crate::parse_typarams;
use crate::type_system::type_checking;
use anyhow::Result;
use shiika_ast::*;
use shiika_core::{names::*, ty, ty::*};
use skc_hir::*;
use std::collections::HashMap;

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
    pub(super) ctx_stack: CtxStack,
    /// Counter to give unique name for lambdas
    pub(super) lambda_ct: usize,
    /// Counter for unique name
    pub(super) gensym_ct: usize,
}

impl<'hir_maker> HirMaker<'hir_maker> {
    pub fn new(
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
            ctx_stack: CtxStack::new(vec![HirMakerContext::toplevel()]),
            lambda_ct: 0,
            gensym_ct: 0,
        }
    }

    /// Destructively convert self to Hir
    pub fn extract_hir(&mut self, main_exprs: HirExpressions, main_lvars: HirLVars) -> Hir {
        // Extract data from self
        let sk_types = std::mem::take(&mut self.class_dict.sk_types);
        let sk_methods = std::mem::take(&mut self.method_dict.0);
        let mut constants = HashMap::new();
        std::mem::swap(&mut constants, &mut self.constants);
        let mut str_literals = vec![];
        std::mem::swap(&mut str_literals, &mut self.str_literals);
        let mut const_inits = vec![];
        std::mem::swap(&mut const_inits, &mut self.const_inits);

        Hir {
            sk_types,
            sk_methods,
            constants,
            str_literals,
            const_inits,
            main_exprs,
            main_lvars,
        }
    }

    /// Register constants which has the same as the class
    /// eg.
    /// - ::Int (#<class Int>)
    /// - ::Array (#<class Array>)
    /// - ::Void (the only instance of the class Void)
    /// - ::Maybe::None (the only instance of the class Maybe::None)
    pub fn define_class_constants(&mut self) -> Result<()> {
        let nonmeta = self
            .class_dict
            .sk_types
            .0
            .iter()
            .filter(|(_, sk_type)| !sk_type.fullname().is_meta());
        let v = nonmeta
            .map(|(name, sk_type)| {
                let const_is_obj = sk_type.class().map(|c| c.const_is_obj).unwrap_or(false);
                let includes_modules = sk_type
                    .class()
                    .map(|c| !c.includes.is_empty())
                    .unwrap_or(false);
                (name.clone(), const_is_obj, includes_modules)
            })
            .collect::<Vec<_>>();
        for (name, const_is_obj, includes_modules) in v {
            let expr = if const_is_obj {
                // Create constant like `Void`, `Maybe::None`.
                let ty = ty::raw(&name.0);
                // The class
                let cls_obj = self.create_class_literal(&name, includes_modules)?;
                // The instance
                Hir::method_call(
                    ty,
                    cls_obj,
                    method_fullname(metaclass_fullname(&name.0).into(), "new"),
                    vec![],
                )
            } else {
                self.create_class_literal(&name, includes_modules)?
            };
            self.register_const_full(name.to_const_fullname(), expr);
        }
        Ok(())
    }

    fn create_class_literal(
        &mut self,
        name: &TypeFullname,
        includes_modules: bool,
    ) -> Result<HirExpression> {
        let ty = ty::meta(&name.0);
        let str_idx = self.register_string_literal(&name.0);
        // These two are for calling class-level initialize.
        let (initialize_name, init_cls_name) = self._find_initialize(&ty)?;
        Ok(Hir::class_literal(
            ty,
            name.clone(),
            str_idx,
            includes_modules,
            initialize_name,
            init_cls_name,
        ))
    }

    pub fn convert_toplevel_items(
        &mut self,
        items: Vec<shiika_ast::TopLevelItem>,
    ) -> Result<(HirExpressions, HirLVars)> {
        let mut defs = vec![];
        let mut top_exprs = vec![];
        for item in items {
            match item {
                shiika_ast::TopLevelItem::Def(def) => {
                    defs.push(def);
                }
                shiika_ast::TopLevelItem::Expr(expr) => {
                    top_exprs.push(expr);
                }
            }
        }
        self.process_defs(&Namespace::root(), None, &defs)?;

        let mut main_exprs = vec![];
        for expr in top_exprs {
            main_exprs.push(self.convert_expr(&expr)?);
        }

        debug_assert!(self.ctx_stack.len() == 1);
        let mut toplevel_ctx = self.ctx_stack.pop_toplevel_ctx();
        Ok((
            HirExpressions::new(main_exprs),
            extract_lvars(&mut toplevel_ctx.lvars),
        ))
    }

    // Process definitions in a class or the toplevel.
    fn process_defs(
        &mut self,
        namespace: &Namespace,
        opt_fullname: Option<&ClassFullname>,
        defs: &[shiika_ast::Definition],
    ) -> Result<()> {
        for def in defs {
            match def {
                shiika_ast::Definition::InstanceMethodDefinition { sig, body_exprs } => {
                    if let Some(fullname) = opt_fullname {
                        log::trace!("method {}#{}", &fullname, &sig.name);
                        let method =
                            self.convert_method_def(&fullname.to_type_fullname(), sig, body_exprs)?;
                        self.method_dict
                            .add_method(fullname.to_type_fullname(), method);
                    } else {
                        return Err(error::program_error(
                            "you cannot define methods at toplevel",
                        ));
                    }
                }
                shiika_ast::Definition::InitializerDefinition { .. } => {
                    // Already processed in process_class_def
                }
                shiika_ast::Definition::ClassMethodDefinition {
                    sig, body_exprs, ..
                } => {
                    if let Some(fullname) = opt_fullname {
                        let meta_name = fullname.meta_name();
                        log::trace!("method {}.{}", &fullname, &sig.name);
                        let method = self.convert_method_def(
                            &meta_name.to_type_fullname(),
                            &sig,
                            body_exprs,
                        )?;
                        self.method_dict
                            .add_method(meta_name.to_type_fullname(), method);
                    } else {
                        return Err(error::program_error(
                            "you cannot define methods at toplevel",
                        ));
                    }
                }
                shiika_ast::Definition::ClassInitializerDefinition { .. } => {
                    // Already processed in process_class_def
                }
                shiika_ast::Definition::ConstDefinition { name, expr } => {
                    if opt_fullname.is_some() {
                        // Already processed
                    } else {
                        self.register_toplevel_const(name, expr)?;
                    }
                }
                shiika_ast::Definition::ClassDefinition {
                    name,
                    defs,
                    typarams,
                    ..
                } => self.process_class_def(namespace, name, parse_typarams(typarams), defs)?,
                shiika_ast::Definition::ModuleDefinition {
                    name,
                    typarams,
                    defs,
                    ..
                } => {
                    self.process_module_def(namespace, name, parse_typarams(typarams), defs)?;
                }
                shiika_ast::Definition::EnumDefinition {
                    name,
                    typarams,
                    cases,
                    defs,
                } => {
                    self.process_enum_def(namespace, name, parse_typarams(typarams), cases, defs)?
                }
                shiika_ast::Definition::MethodRequirementDefinition { .. } => {
                    // Already processed in class_dict/indexing.rs
                }
            }
        }
        Ok(())
    }

    /// Process a class definition and its inner defs
    fn process_class_def(
        &mut self,
        namespace: &Namespace,
        firstname: &ClassFirstname,
        typarams: Vec<TyParam>,
        defs: &[shiika_ast::Definition],
    ) -> Result<()> {
        let fullname = namespace.class_fullname(firstname);
        let meta_name = fullname.meta_name();
        let inner_namespace = namespace.add(firstname.to_string());
        self.ctx_stack
            .push(HirMakerContext::class(inner_namespace.clone(), typarams));

        // Register constants before processing #initialize
        self._process_const_defs_in_class(&inner_namespace, defs)?;

        // Register #initialize and ivars
        let own_ivars = self._process_initialize(&fullname, shiika_ast::find_initializer(defs))?;
        if !own_ivars.is_empty() {
            // Be careful not to reset ivars of corelib/* by builtin/*
            self.class_dict.define_ivars(&fullname, own_ivars.clone());
            self.define_accessors(&fullname, own_ivars, defs);
        }

        // Register .new
        if fullname.0 != "Never" {
            let class_name = ty::raw(&fullname.0);
            self.method_dict.add_method(
                meta_name.to_type_fullname(),
                self.create_new(&class_name, false)?,
            );
        }

        // Register class-level initialize and ivars
        let cls_ivars =
            self._process_initialize(&meta_name, shiika_ast::find_class_initializer(defs))?;
        if !cls_ivars.is_empty() {
            self.class_dict.define_ivars(&meta_name, cls_ivars.clone());
            self.define_accessors(&meta_name, cls_ivars, defs);
        }

        // Process inner defs
        self.process_defs(&inner_namespace, Some(&fullname), defs)?;
        self.ctx_stack.pop_class_ctx();
        Ok(())
    }

    /// Process a module definition and its inner defs
    fn process_module_def(
        &mut self,
        namespace: &Namespace,
        firstname: &ModuleFirstname,
        typarams: Vec<TyParam>,
        defs: &[shiika_ast::Definition],
    ) -> Result<()> {
        let fullname = namespace.class_fullname(&firstname.to_class_first_name());
        let inner_namespace = namespace.add(firstname.to_string());
        self.ctx_stack
            .push(HirMakerContext::class(inner_namespace.clone(), typarams));

        // Register constants before processing the methods
        self._process_const_defs_in_class(&inner_namespace, defs)?;

        // Process inner defs
        self.process_defs(&inner_namespace, Some(&fullname), defs)?;
        self.ctx_stack.pop_class_ctx();
        Ok(())
    }

    /// Add `#initialize` and return defined ivars
    fn _process_initialize(
        &mut self,
        fullname: &ClassFullname,
        initializer: Option<&shiika_ast::InitializerDefinition>,
    ) -> Result<SkIVars> {
        let mut own_ivars = HashMap::default();
        if let Some(d) = initializer {
            log::trace!("method {}#initialize", &fullname);
            let (sk_method, found_ivars) =
                self.create_initialize(fullname, &d.sig, &d.body_exprs)?;
            self.method_dict
                .add_method(fullname.to_type_fullname(), sk_method);
            own_ivars = found_ivars;
        }
        Ok(own_ivars)
    }

    /// Register constants defined in a class
    fn _process_const_defs_in_class(
        &mut self,
        namespace: &Namespace,
        defs: &[shiika_ast::Definition],
    ) -> Result<()> {
        for def in defs {
            if let shiika_ast::Definition::ConstDefinition { name, expr } = def {
                let full = namespace.const_fullname(name);
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
        sig: &AstMethodSignature,
        body_exprs: &[AstExpression],
    ) -> Result<(SkMethod, SkIVars)> {
        let super_ivars = self.class_dict.superclass_ivars(class_fullname);
        self.convert_method_def_(
            &class_fullname.to_type_fullname(),
            sig,
            body_exprs,
            super_ivars,
        )
    }

    /// Create .new
    fn create_new(&self, class_name: &TermTy, const_is_obj: bool) -> Result<SkMethod> {
        let (initialize_name, init_cls_name) = self._find_initialize(class_name)?;
        let found = self.class_dict.lookup_method(
            &class_name.meta_ty(),
            &method_firstname("new"),
            Default::default(),
        )?;
        let new_body = SkMethodBody::New {
            classname: class_name.fullname.clone(),
            initialize_name,
            init_cls_name,
            arity: found.sig.params.len(),
            const_is_obj,
        };
        if found.sig.has_default_expr() {
            return Err(error::program_error(
                "sorry, #initialize cannot have default expr (yet.)",
            ));
        }
        Ok(SkMethod::simple(found.sig, new_body))
    }

    /// Find actual `initialize` func to call from `.new`
    fn _find_initialize(&self, class: &TermTy) -> Result<(MethodFullname, ClassFullname)> {
        let found = self.class_dict.lookup_method(
            class,
            &method_firstname("initialize"),
            Default::default(),
        )?;
        let fullname = found.owner.to_class_fullname();
        Ok((
            method_fullname(fullname.clone().into(), "initialize"),
            fullname,
        ))
    }

    /// Register a constant defined in the toplevel
    pub(super) fn register_toplevel_const(
        &mut self,
        name: &str,
        expr: &AstExpression,
    ) -> Result<()> {
        let hir_expr = self.convert_expr(expr)?;
        self.constants
            .insert(toplevel_const(name), hir_expr.ty.clone());
        let op = Hir::const_assign(toplevel_const(name), hir_expr, LocationSpan::todo());
        self.const_inits.push(op);
        Ok(())
    }

    /// Register a constant
    pub(super) fn register_const_full(&mut self, fullname: ConstFullname, hir_expr: HirExpression) {
        debug_assert!(!self.constants.contains_key(&fullname));
        self.constants.insert(fullname.clone(), hir_expr.ty.clone());
        let op = Hir::const_assign(fullname, hir_expr, LocationSpan::todo());
        self.const_inits.push(op);
    }

    fn convert_method_def(
        &mut self,
        type_fullname: &TypeFullname,
        sig: &AstMethodSignature,
        body_exprs: &[AstExpression],
    ) -> Result<SkMethod> {
        let (sk_method, _ivars) = self.convert_method_def_(type_fullname, sig, body_exprs, None)?;
        Ok(sk_method)
    }

    /// Create a SkMethod and return it with ctx.iivars
    fn convert_method_def_(
        &mut self,
        type_fullname: &TypeFullname,
        ast_sig: &AstMethodSignature,
        body_exprs: &[AstExpression],
        super_ivars: Option<SkIVars>,
    ) -> Result<(SkMethod, HashMap<String, SkIVar>)> {
        // MethodSignature is built beforehand by class_dict::new
        let signature = self
            .class_dict
            .find_method_sig(type_fullname, &ast_sig.name)
            .unwrap_or_else(|| {
                panic!(
                    "[BUG] signature not found ({}/{})",
                    type_fullname, &ast_sig.name
                )
            });

        self.ctx_stack
            .push(HirMakerContext::method(signature.clone(), super_ivars));
        for param in &signature.params {
            if param.has_default {
                let readonly = true;
                self.ctx_stack
                    .declare_lvar(&param.name, param.ty.clone(), readonly);
            }
        }
        let mut hir_exprs = self.convert_exprs(body_exprs)?;
        if signature.has_default_expr() {
            hir_exprs.prepend(_set_defaults(self, ast_sig)?);
        }
        // Insert ::Void so that last expr always matches to ret_ty
        if signature.ret_ty.is_void_type() {
            hir_exprs.voidify();
        }
        let mut method_ctx = self.ctx_stack.pop_method_ctx();
        let lvars = extract_lvars(&mut method_ctx.lvars);
        type_checking::check_return_value(&self.class_dict, &signature, &hir_exprs)?;

        let method = SkMethod {
            signature,
            body: SkMethodBody::Normal { exprs: hir_exprs },
            lvars,
        };
        Ok((method, method_ctx.iivars))
    }

    /// Process a enum definition
    fn process_enum_def(
        &mut self,
        namespace: &Namespace,
        firstname: &ClassFirstname,
        typarams: Vec<TyParam>,
        cases: &[shiika_ast::EnumCase],
        defs: &[shiika_ast::Definition],
    ) -> Result<()> {
        let fullname = namespace.class_fullname(firstname);
        let inner_namespace = namespace.add(firstname.to_string());
        for case in cases {
            self._register_enum_case_class(&inner_namespace, case)?;
        }
        self.ctx_stack
            .push(HirMakerContext::class(inner_namespace.clone(), typarams));

        self.process_defs(&inner_namespace, Some(&fullname), defs)?;
        self.ctx_stack.pop_class_ctx();
        Ok(())
    }

    /// Create a enum case class
    fn _register_enum_case_class(&mut self, namespace: &Namespace, case: &EnumCase) -> Result<()> {
        let fullname = namespace.class_fullname(&case.name);

        // Register #initialize
        let signature = self
            .class_dict
            .find_method_sig(
                &fullname.to_type_fullname(),
                &method_firstname("initialize"),
            )
            .unwrap();
        let self_ty = ty::raw(&fullname.0);
        let exprs = signature
            .params
            .iter()
            .enumerate()
            .map(|(idx, param)| {
                let argref = Hir::arg_ref(param.ty.clone(), idx, LocationSpan::todo());
                Hir::ivar_assign(
                    &param.name,
                    idx,
                    argref,
                    false,
                    self_ty.clone(),
                    LocationSpan::todo(),
                )
            })
            .collect();
        if signature.has_default_expr() {
            return Err(error::program_error(
                "sorry, enums cannot have default expression (yet).",
            ));
        }
        let initialize = SkMethod::simple(
            signature,
            SkMethodBody::Normal {
                exprs: HirExpressions::new(exprs),
            },
        );
        self.method_dict
            .add_method(fullname.to_type_fullname(), initialize);
        let ivars = self.class_dict.get_class(&fullname).ivars.clone();
        self.define_accessors(&fullname, ivars, Default::default());

        // Register .new
        let const_is_obj = case.params.is_empty();
        let class = ty::raw(&fullname.0);
        self.method_dict.add_method(
            fullname.meta_name().into(),
            self.create_new(&class, const_is_obj)?,
        );
        Ok(())
    }

    /// Generate special lvar name
    /// Note: don't forget calling ctx_stack.declare_lvar
    pub fn generate_lvar_name(&mut self, prefix: &str) -> String {
        let n = self.gensym_ct;
        self.gensym_ct += 1;
        // Suffix `_` because llvm may add numbers after this name
        // eg.
        //   %"expr@0_3" = load %Maybe*, %Maybe** %"expr@0"
        format!("{}@{}_", prefix, n)
    }
}

/// Destructively extract list of local variables
pub fn extract_lvars(lvars: &mut HashMap<String, CtxLVar>) -> HirLVars {
    std::mem::take(lvars)
        .into_iter()
        .map(|(name, ctx_lvar)| HirLVar {
            name,
            ty: ctx_lvar.ty,
            captured: ctx_lvar.captured,
        })
        .collect::<Vec<_>>()
}

/// Create expressions that sets default value for omitted args
fn _set_defaults(mk: &mut HirMaker, ast_sig: &AstMethodSignature) -> Result<Vec<HirExpression>> {
    let target = ast_sig
        .params
        .iter()
        .enumerate()
        .filter_map(|(i, p)| p.default_expr.as_ref().map(|e| (i, &p.name, e)));
    target
        .map(|(i, name, e)| _set_default(mk, i, name, e))
        .collect()
}

// Build a HIR which initializes the lvar for omittable arg
fn _set_default(
    mk: &mut HirMaker,
    idx: usize,
    name: &str,
    expr: &AstExpression,
) -> Result<HirExpression> {
    let value_expr = mk.convert_expr(&expr)?;
    let locs = LocationSpan::internal();
    let arg = Hir::arg_ref(value_expr.ty.clone(), idx, locs.clone());
    let cond_expr = Hir::is_omitted_value(arg.clone());

    let mut then_exprs = HirExpressions::void();
    then_exprs.prepend(vec![Hir::lvar_assign(
        name.to_string(),
        value_expr,
        locs.clone(),
    )]);
    let mut else_exprs = HirExpressions::void();
    else_exprs.prepend(vec![Hir::lvar_assign(name.to_string(), arg, locs.clone())]);

    // TODO: Proper handle later /
    let if_expr = Hir::if_expression(
        ty::raw("Void"),
        vec![],
        cond_expr,
        then_exprs,
        else_exprs,
        LocationSpan::internal(),
    );
    Ok(if_expr)
}
