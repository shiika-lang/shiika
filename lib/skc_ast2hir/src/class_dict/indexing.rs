use crate::class_dict::build_wtable::build_wtable;
use crate::class_dict::*;
use crate::convert_exprs::params;
use crate::error;
use crate::parse_typarams;
use anyhow::Result;
use shiika_ast::{self, LocationSpan, UnresolvedTypeName};
use shiika_core::{names::*, ty, ty::*};
use skc_error::{self, Label};
use skc_hir::signature::*;
use skc_hir::*;
use std::collections::HashMap;

impl<'hir_maker> ClassDict<'hir_maker> {
    /// Register a class or module
    pub fn add_type(&mut self, sk_type_: impl Into<SkType>) {
        let sk_type = sk_type_.into();
        self.sk_types.0.insert(sk_type.fullname(), sk_type);
    }

    /// Add a method
    /// Used to add auto-defined accessors
    pub fn add_method(&mut self, clsname: &ClassFullname, sig: MethodSignature) {
        let sk_class = self
            .sk_types
            .0
            .get_mut(&clsname.to_type_fullname())
            .unwrap();
        sk_class.base_mut().method_sigs.insert(sig);
    }

    pub fn index_program(&mut self, toplevel_defs: &[&shiika_ast::Definition]) -> Result<()> {
        let namespace = Namespace::root();
        for def in toplevel_defs {
            match def {
                shiika_ast::Definition::ClassDefinition {
                    name,
                    typarams,
                    supers,
                    defs,
                } => self.index_class(&namespace, name, parse_typarams(typarams), supers, defs)?,
                shiika_ast::Definition::ModuleDefinition {
                    name,
                    typarams,
                    defs,
                } => self.index_module(&namespace, name, parse_typarams(typarams), defs)?,
                shiika_ast::Definition::EnumDefinition {
                    name,
                    typarams,
                    cases,
                    defs,
                } => self.index_enum(&namespace, name, parse_typarams(typarams), cases, defs)?,
                shiika_ast::Definition::ConstDefinition { .. } => (),
                _ => {
                    return Err(error::syntax_error(&format!(
                        "must not be toplevel: {:?}",
                        def
                    )))
                }
            }
        }
        Ok(())
    }

    fn index_class(
        &mut self,
        namespace: &Namespace,
        firstname: &ClassFirstname,
        typarams: Vec<ty::TyParam>,
        supers: &[UnresolvedTypeName],
        defs: &[shiika_ast::Definition],
    ) -> Result<()> {
        let inner_namespace = namespace.add(firstname.to_string());
        let fullname = namespace.class_fullname(firstname);
        let metaclass_fullname = fullname.meta_name();
        let (superclass, includes) = self._resolve_supers(namespace, &typarams, supers)?;
        let new_sig = if fullname.0 == "Never" {
            None
        } else {
            Some(signature::signature_of_new(
                &metaclass_fullname,
                self._initializer_params(&inner_namespace, &typarams, &superclass, defs)?,
                typarams.clone(),
            ))
        };

        let (instance_methods, class_methods) =
            self.index_defs_in_class(&inner_namespace, &fullname, &typarams, defs)?;

        let wtable = build_wtable(self, &instance_methods, &includes)?;
        match self.sk_types.0.get_mut(&fullname.to_type_fullname()) {
            Some(sk_type) => {
                // This class is predefined in skc_corelib.
                // Inject `includes`
                if let SkType::Class(sk_class) = sk_type {
                    sk_class.wtable = wtable;
                    sk_class.includes = includes;
                }
                // Inject instance methods
                let method_sigs = &mut sk_type.base_mut().method_sigs;
                method_sigs.append(instance_methods);
                if let Some(sigs) = self.rust_methods.remove(&fullname.to_type_fullname()) {
                    method_sigs.append_vec(sigs);
                }
                // Inject class methods
                let metaclass = self
                    .sk_types
                    .0
                    .get_mut(&metaclass_fullname.to_type_fullname())
                    .unwrap_or_else(|| {
                        panic!(
                            "[BUG] metaclass not found: {} <- {}",
                            fullname, &metaclass_fullname
                        )
                    });
                let meta_method_sigs = &mut metaclass.base_mut().method_sigs;
                meta_method_sigs.append(class_methods);
                if let Some(sigs) = self
                    .rust_methods
                    .remove(&metaclass_fullname.to_type_fullname())
                {
                    meta_method_sigs.append_vec(sigs);
                }
                // Inject `.new` to the metaclass
                if let Some(sig) = new_sig {
                    if !metaclass
                        .base()
                        .method_sigs
                        .contains_key(&method_firstname("new"))
                    {
                        metaclass.base_mut().method_sigs.insert(sig);
                    }
                }
            }
            None => {
                self.add_new_class(
                    &fullname,
                    &typarams,
                    superclass,
                    includes,
                    new_sig,
                    instance_methods,
                    class_methods,
                    Some(false),
                    false,
                )?;
            }
        }
        Ok(())
    }

    /// Resolve superclass and included module names of a class definition
    fn _resolve_supers(
        &self,
        namespace: &Namespace,
        class_typarams: &[ty::TyParam],
        supers: &[UnresolvedTypeName],
    ) -> Result<(Supertype, Vec<Supertype>)> {
        let mut modules = vec![];
        let mut superclass = None;
        for name in supers {
            let ty = self.resolve_typename(namespace, class_typarams, Default::default(), name)?;
            match self.find_type(&ty.erasure().to_type_fullname()) {
                Some(SkType::Class(c)) => {
                    if !modules.is_empty() {
                        return Err(error::program_error(&format!(
                            "superclass {} must be the first",
                            ty
                        )));
                    }
                    if superclass.is_some() {
                        return Err(error::program_error(&format!(
                            "only one superclass is allowed but got {}",
                            ty
                        )));
                    }
                    if c.is_final.unwrap() {
                        return Err(error::program_error(&format!(
                            "inheriting {} is not allowed",
                            ty
                        )));
                    }
                    match &ty.body {
                        TyBody::TyPara(_) => {
                            return Err(error::program_error(&format!(
                                "type parameter {} cannot be a supertype",
                                ty
                            )));
                        }
                        TyBody::TyRaw(lit_ty) => {
                            superclass = Some(Supertype::from_ty(lit_ty.clone()));
                        }
                    }
                }
                Some(SkType::Module(_)) => match &ty.body {
                    TyBody::TyPara(_) => {
                        return Err(error::program_error(&format!(
                            "type parameter {} cannot be a supertype",
                            ty
                        )));
                    }
                    TyBody::TyRaw(lit_ty) => {
                        modules.push(Supertype::from_ty(lit_ty.clone()));
                    }
                },
                None => {
                    return Err(error::program_error(&format!(
                        "unknown class or module {}",
                        ty
                    )));
                }
            }
        }
        Ok((superclass.unwrap_or_else(Supertype::default), modules))
    }

    fn index_module(
        &mut self,
        namespace: &Namespace,
        firstname: &ModuleFirstname,
        typarams: Vec<ty::TyParam>,
        defs: &[shiika_ast::Definition],
    ) -> Result<()> {
        let fullname = namespace.module_fullname(firstname);
        let inner_namespace = namespace.add(firstname.to_string());
        let (instance_methods, class_methods, requirements) =
            self.index_defs_in_module(&inner_namespace, &fullname, &typarams, defs)?;

        match self.sk_types.0.get_mut(&fullname.to_type_fullname()) {
            Some(_) => todo!(),
            None => self.add_new_module(
                &fullname,
                &typarams,
                instance_methods,
                class_methods,
                requirements,
            ),
        }
        Ok(())
    }

    /// Return parameters of `initialize` which is defined by
    /// - `#initialize` in `defs` (if any) or,
    /// - `#initialize` inherited from ancestors.
    fn _initializer_params(
        &self,
        namespace: &Namespace,
        typarams: &[ty::TyParam],
        superclass: &Supertype,
        defs: &[shiika_ast::Definition],
    ) -> Result<Vec<MethodParam>> {
        if let Some(shiika_ast::InitializerDefinition { sig, .. }) =
            shiika_ast::find_initializer(defs)
        {
            // Has explicit initializer definition
            params::convert_params(self, namespace, &sig.params, typarams, Default::default())
        } else {
            // Inherit #initialize from superclass
            let found = self
                .lookup_method(
                    &superclass.to_term_ty(),
                    &method_firstname("initialize"),
                    &LocationSpan::internal(),
                )
                .expect("[BUG] initialize not found");
            Ok(specialized_initialize(&found.sig, superclass).params)
        }
    }

    fn index_enum(
        &mut self,
        namespace: &Namespace,
        firstname: &ClassFirstname,
        typarams: Vec<TyParam>,
        cases: &[shiika_ast::EnumCase],
        defs: &[shiika_ast::Definition],
    ) -> Result<()> {
        let fullname = namespace.class_fullname(firstname);
        let inner_namespace = namespace.add(firstname.to_string());
        let (instance_methods, class_methods) =
            self.index_defs_in_class(&inner_namespace, &fullname, &typarams, defs)?;
        self.add_new_class(
            &fullname,
            &typarams,
            Supertype::simple("Object"),
            Default::default(),
            None,
            instance_methods,
            class_methods,
            Some(true),
            false,
        )?;
        for case in cases {
            self.index_enum_case(namespace, &fullname, &typarams, case)?;
        }

        Ok(())
    }

    fn index_enum_case(
        &mut self,
        namespace: &Namespace,
        enum_fullname: &ClassFullname,
        typarams: &[ty::TyParam],
        case: &shiika_ast::EnumCase,
    ) -> Result<()> {
        let ivar_list = self._enum_case_ivars(namespace, typarams, case)?;
        let fullname = case.name.add_namespace(&enum_fullname.0);
        let superclass = enum_case_superclass(enum_fullname, typarams, case);
        let (new_sig, initialize_sig) = enum_case_new_sig(&ivar_list, typarams, &fullname);

        let mut instance_methods = enum_case_getters(&fullname, &ivar_list);
        instance_methods.insert(initialize_sig);

        let case_typarams = if case.params.is_empty() {
            Default::default()
        } else {
            typarams
        };
        self.add_new_class(
            &fullname,
            case_typarams,
            superclass,
            Default::default(),
            Some(new_sig),
            instance_methods,
            Default::default(),
            Some(true),
            case.params.is_empty(),
        )?;
        let ivars = ivar_list.into_iter().map(|x| (x.name.clone(), x)).collect();
        self.define_ivars(&fullname, ivars);
        Ok(())
    }

    /// List up ivars of an enum case
    fn _enum_case_ivars(
        &self,
        namespace: &Namespace,
        typarams: &[ty::TyParam],
        case: &shiika_ast::EnumCase,
    ) -> Result<Vec<SkIVar>> {
        let mut ivars = vec![];
        for (idx, param) in case.params.iter().enumerate() {
            let ty = self.resolve_typename(namespace, typarams, Default::default(), &param.typ)?;
            let ivar = SkIVar {
                idx,
                name: param.name.clone(),
                ty,
                readonly: true,
            };
            ivars.push(ivar);
        }
        Ok(ivars)
    }

    fn index_defs_in_class(
        &mut self,
        namespace: &Namespace,
        fullname: &ClassFullname,
        typarams: &[ty::TyParam],
        defs: &[shiika_ast::Definition],
    ) -> Result<(MethodSignatures, MethodSignatures)> {
        let (instance_methods, class_methods, _) = self._index_inner_defs(
            namespace,
            fullname.to_type_fullname(),
            typarams,
            defs,
            false,
        )?;
        Ok((instance_methods, class_methods))
    }

    fn index_defs_in_module(
        &mut self,
        namespace: &Namespace,
        fullname: &ModuleFullname,
        typarams: &[ty::TyParam],
        defs: &[shiika_ast::Definition],
    ) -> Result<(MethodSignatures, MethodSignatures, Vec<MethodSignature>)> {
        self._index_inner_defs(namespace, fullname.to_type_fullname(), typarams, defs, true)
    }

    fn _index_inner_defs(
        &mut self,
        namespace: &Namespace,
        fullname: TypeFullname,
        typarams: &[ty::TyParam],
        defs: &[shiika_ast::Definition],
        is_module: bool,
    ) -> Result<(MethodSignatures, MethodSignatures, Vec<MethodSignature>)> {
        let mut instance_methods = MethodSignatures::new();
        let mut class_methods = MethodSignatures::new();
        let mut requirements = vec![];
        for def in defs {
            match def {
                shiika_ast::Definition::InstanceMethodDefinition { sig, .. } => {
                    let hir_sig =
                        self.create_signature(namespace, fullname.clone(), sig, typarams)?;
                    instance_methods.insert(hir_sig);
                }
                shiika_ast::Definition::InitializerDefinition(
                    shiika_ast::InitializerDefinition { sig, .. },
                ) => {
                    let hir_sig =
                        self.create_signature(namespace, fullname.clone(), sig, typarams)?;
                    self._index_accessors(&mut instance_methods, sig, &hir_sig);
                    instance_methods.insert(hir_sig);
                }
                shiika_ast::Definition::ClassMethodDefinition { sig, .. } => {
                    let hir_sig = self.create_signature(
                        namespace,
                        fullname.meta_name().to_type_fullname(),
                        sig,
                        Default::default(),
                    )?;
                    class_methods.insert(hir_sig);
                }
                shiika_ast::Definition::ClassInitializerDefinition(
                    shiika_ast::InitializerDefinition { sig, .. },
                ) => {
                    if !sig.params.is_empty() {
                        return Err(error::program_error(&format!(
                            "{}.{} should take no parameters",
                            namespace, &sig.name
                        )));
                    }
                    let hir_sig = self.create_signature(
                        namespace,
                        fullname.meta_name().to_type_fullname(),
                        sig,
                        Default::default(),
                    )?;
                    class_methods.insert(hir_sig);
                }
                shiika_ast::Definition::ConstDefinition { .. } => (),
                shiika_ast::Definition::ClassDefinition {
                    name,
                    typarams,
                    supers,
                    defs,
                } => {
                    self.index_class(namespace, name, parse_typarams(typarams), supers, defs)?;
                }
                shiika_ast::Definition::ModuleDefinition {
                    name,
                    typarams,
                    defs,
                } => {
                    self.index_module(namespace, name, parse_typarams(typarams), defs)?;
                }
                shiika_ast::Definition::MethodRequirementDefinition { sig } => {
                    if is_module {
                        let hir_sig =
                            self.create_signature(namespace, fullname.clone(), sig, typarams)?;
                        requirements.push(hir_sig);
                    } else {
                        return Err(error::syntax_error(&format!(
                            "only modules have method requirement: {:?} {:?} {:?}",
                            namespace, fullname, sig
                        )));
                    }
                }
                shiika_ast::Definition::EnumDefinition {
                    name,
                    typarams,
                    cases,
                    defs,
                } => {
                    self.index_enum(namespace, name, parse_typarams(typarams), cases, defs)?;
                }
            }
        }
        Ok((instance_methods, class_methods, requirements))
    }

    /// Register getters/setters from signature of `#initialize`
    fn _index_accessors(
        &self,
        instance_methods: &mut MethodSignatures,
        sig: &shiika_ast::AstMethodSignature,
        hir_sig: &MethodSignature,
    ) {
        let type_name = &hir_sig.fullname.type_name;
        for (param, hir_param) in sig.params.iter().zip(hir_sig.params.iter()) {
            if !param.is_iparam {
                continue;
            }
            let sig = MethodSignature {
                fullname: method_fullname(type_name.clone(), &param.name.replace('@', "")),
                ret_ty: hir_param.ty.clone(),
                params: Default::default(),
                typarams: Default::default(),
            };
            instance_methods.insert(sig);
        }
    }

    /// Register a class and its metaclass to self
    // REFACTOR: fix too_many_arguments
    #[allow(clippy::too_many_arguments)]
    fn add_new_class(
        &mut self,
        fullname: &ClassFullname,
        typarams: &[ty::TyParam],
        superclass: Supertype,
        includes: Vec<Supertype>,
        new_sig: Option<MethodSignature>,
        mut instance_methods: MethodSignatures,
        mut class_methods: MethodSignatures,
        is_final: Option<bool>,
        const_is_obj: bool,
    ) -> Result<()> {
        self.transfer_rust_method_sigs(&fullname.to_type_fullname(), &mut instance_methods);

        // Add `.new` to the metaclass
        if let Some(sig) = new_sig {
            class_methods.insert(sig);
        }

        let wtable = build_wtable(self, &instance_methods, &includes)?;
        let base = SkTypeBase {
            erasure: Erasure::nonmeta(&fullname.0),
            typarams: typarams.to_vec(),
            method_sigs: instance_methods,
            foreign: false,
        };
        self.add_type(SkClass {
            base,
            superclass: Some(superclass),
            includes,
            ivars: HashMap::new(), // will be set when processing `#initialize`
            is_final,
            const_is_obj,
            wtable,
        });

        // Create metaclass (which is a subclass of `Class`)
        self.transfer_rust_method_sigs(
            &fullname.meta_name().to_type_fullname(),
            &mut class_methods,
        );
        let the_class = self.get_class(&class_fullname("Class"));
        let meta_ivars = the_class.ivars.clone();
        let base = SkTypeBase {
            erasure: Erasure::meta(&fullname.0),
            typarams: typarams.to_vec(),
            method_sigs: class_methods,
            foreign: false,
        };
        self.add_type(SkClass {
            base,
            superclass: Some(Supertype::simple("Class")),
            includes: Default::default(),
            ivars: meta_ivars,
            is_final: None,
            const_is_obj: false,
            wtable: Default::default(),
        });
        Ok(())
    }

    /// Register a module and its metaclass(metamodule?) to self
    fn add_new_module(
        &mut self,
        fullname: &ModuleFullname,
        typarams: &[ty::TyParam],
        mut instance_methods: MethodSignatures,
        mut class_methods: MethodSignatures,
        requirements: Vec<MethodSignature>,
    ) {
        self.transfer_rust_method_sigs(&fullname.to_type_fullname(), &mut instance_methods);
        let base = SkTypeBase {
            erasure: Erasure::nonmeta(&fullname.0),
            typarams: typarams.to_vec(),
            method_sigs: instance_methods,
            foreign: false,
        };
        self.add_type(SkModule::new(base, requirements));

        // Create metaclass (which is a subclass of `Class`)
        self.transfer_rust_method_sigs(
            &fullname.meta_name().to_type_fullname(),
            &mut class_methods,
        );
        let the_class = self.get_class(&class_fullname("Class"));
        let meta_ivars = the_class.ivars.clone();
        let base = SkTypeBase {
            erasure: Erasure::meta(&fullname.0),
            typarams: typarams.to_vec(),
            method_sigs: class_methods,
            foreign: false,
        };
        self.add_type(SkClass {
            base,
            superclass: Some(Supertype::simple("Class")),
            includes: Default::default(),
            ivars: meta_ivars,
            is_final: None,
            const_is_obj: false,
            wtable: Default::default(),
        });
    }

    /// Convert AstMethodSignature to MethodSignature
    pub fn create_signature(
        &self,
        namespace: &Namespace,
        type_fullname: TypeFullname,
        sig: &shiika_ast::AstMethodSignature,
        class_typarams: &[ty::TyParam],
    ) -> Result<MethodSignature> {
        let method_typarams = parse_typarams(&sig.typarams);
        let fullname = method_fullname(type_fullname, &sig.name.0);
        let ret_ty = if let Some(typ) = &sig.ret_typ {
            self.resolve_typename(namespace, class_typarams, &method_typarams, typ)?
        } else {
            ty::raw("Void") // Default return type.
        };
        Ok(MethodSignature {
            fullname,
            ret_ty,
            params: params::convert_params(
                self,
                namespace,
                &sig.params,
                class_typarams,
                &method_typarams,
            )?,
            typarams: method_typarams,
        })
    }

    /// Resolve the given type name to fullname
    pub fn resolve_typename(
        &self,
        namespace: &Namespace,
        class_typarams: &[ty::TyParam],
        method_typarams: &[ty::TyParam],
        name: &UnresolvedTypeName,
    ) -> Result<TermTy> {
        // Check it is a typaram
        if name.args.is_empty() && name.names.len() == 1 {
            let s = name.names.first().unwrap();
            if let Some(idx) = class_typarams.iter().position(|t| *s == t.name) {
                return Ok(ty::typaram_ref(s, TyParamKind::Class, idx).into_term_ty());
            } else if let Some(idx) = method_typarams.iter().position(|t| *s == t.name) {
                return Ok(ty::typaram_ref(s, TyParamKind::Method, idx).into_term_ty());
            }
        }
        // Otherwise:
        let mut tyargs = vec![];
        for arg in &name.args {
            tyargs.push(self.resolve_typename(namespace, class_typarams, method_typarams, arg)?);
        }
        let (resolved_base, base_typarams) =
            self._resolve_simple_typename(namespace, &name.names, &name.locs)?;
        if name.args.len() != base_typarams.len() {
            return Err(error::type_error(&format!(
                "wrong number of type arguments: {:?}",
                name
            )));
        }
        Ok(ty::nonmeta(&resolved_base.join("::"), tyargs))
    }

    /// Resolve the given type name (without type arguments) to fullname
    /// Also returns the typarams of the class, if any
    fn _resolve_simple_typename(
        &self,
        namespace: &Namespace,
        names: &[String],
        locs: &LocationSpan,
    ) -> Result<(Vec<String>, &[TyParam])> {
        let n = namespace.size();
        for k in 0..=n {
            let mut resolved = namespace.head(n - k).to_vec();
            resolved.append(&mut names.to_vec());
            if let Some(typarams) = self
                .type_index
                .get(&class_fullname(resolved.join("::")).into())
            {
                return Ok((resolved, typarams));
            }
        }

        let msg = format!("unknown type {} in {:?}", names.join("::"), namespace);
        let report = skc_error::build_report(msg, locs, |r, locs_span| {
            r.with_label(Label::new(locs_span).with_message("unknown type"))
        });
        Err(error::name_error(&report))
    }

    fn transfer_rust_method_sigs(
        &mut self,
        fullname: &TypeFullname,
        method_sigs: &mut MethodSignatures,
    ) {
        if let Some(sigs) = self.rust_methods.remove(fullname) {
            method_sigs.append_vec(sigs);
        }
    }
}

/// Returns superclass of a enum case
fn enum_case_superclass(
    enum_fullname: &ClassFullname,
    typarams: &[ty::TyParam],
    case: &shiika_ast::EnumCase,
) -> Supertype {
    if case.params.is_empty() {
        // eg. Maybe::None : Maybe<Never>
        let tyargs = typarams
            .iter()
            .map(|_| ty::raw("Never"))
            .collect::<Vec<_>>();
        Supertype::from_ty(LitTy::new(enum_fullname.0.clone(), tyargs, false))
    } else {
        // eg. Maybe::Some<out V> : Maybe<V>
        let tyargs = typarams
            .iter()
            .enumerate()
            .map(|(i, t)| ty::typaram_ref(&t.name, TyParamKind::Class, i).into_term_ty())
            .collect::<Vec<_>>();
        Supertype::from_ty(LitTy::new(enum_fullname.0.clone(), tyargs, false))
    }
}

/// Returns signature of `.new` and `#initialize` of an enum case
fn enum_case_new_sig(
    ivar_list: &[SkIVar],
    typarams: &[ty::TyParam],
    fullname: &ClassFullname,
) -> (MethodSignature, MethodSignature) {
    let params = ivar_list
        .iter()
        .map(|ivar| MethodParam {
            name: ivar.name.to_string(),
            ty: ivar.ty.clone(),
            has_default: false,
        })
        .collect::<Vec<_>>();
    (
        signature::signature_of_new(&fullname.meta_name(), params.clone(), typarams.to_vec()),
        signature::signature_of_initialize(fullname, params),
    )
}

/// Create signatures of getters of an enum case
fn enum_case_getters(case_fullname: &ClassFullname, ivars: &[SkIVar]) -> MethodSignatures {
    let iter = ivars.iter().map(|ivar| MethodSignature {
        fullname: method_fullname(case_fullname.to_type_fullname(), &ivar.accessor_name()),
        ret_ty: ivar.ty.clone(),
        params: Default::default(),
        typarams: Default::default(),
    });
    MethodSignatures::from_iterator(iter)
}
