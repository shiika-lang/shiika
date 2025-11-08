use crate::class_dict::build_wtable::build_wtable;
use crate::class_dict::*;
use crate::convert_exprs::params;
use crate::error;
use crate::parse_typarams;
use anyhow::Result;
use shiika_ast::{self, LocationSpan, UnresolvedTypeName};
use shiika_core::{names::*, ty, ty::*};
use skc_error::{self, Label};
use skc_hir::method_signature::{signature_of_new, MethodSignature};
use skc_hir::*;
use std::collections::HashMap;

/// Used during indexing.
/// To register Rust methods, the class must be already indexed, but for #initialize, we want
/// to index it along with the class.
pub type RustMethods = HashMap<TypeFullname, Vec<(AstMethodSignature, bool)>>;

struct ClassSpec {
    namespace: Namespace,
    fullname: ClassFullname,
    typarams: Vec<ty::TyParam>,
    superclass: Option<Supertype>,
    includes: Vec<Supertype>,
    instance_methods: MethodSignatures,
    class_methods: MethodSignatures,
    inheritable: bool,
    const_is_obj: bool,
    has_new: bool,
}

impl<'hir_maker> ClassDict<'hir_maker> {
    /// Register a class or module
    pub fn add_type(&mut self, sk_type_: impl Into<SkType>) {
        let sk_type = sk_type_.into();
        self.sk_types.types.insert(sk_type.fullname(), sk_type);
    }

    /// Add a method
    /// Used to add auto-defined accessors
    pub fn add_method(&mut self, sig: MethodSignature) {
        let clsname = &sig.fullname.type_name;
        let sk_class = self.sk_types.types.get_mut(&clsname).unwrap();
        sk_class.base_mut().method_sigs.insert(sig);
    }

    pub fn index_program(
        &mut self,
        toplevel_defs: &[&shiika_ast::Definition],
        mut rust_methods: RustMethods,
    ) -> Result<()> {
        let namespace = Namespace::root();
        for def in toplevel_defs {
            match def {
                shiika_ast::Definition::ClassDefinition {
                    inheritable,
                    name,
                    typarams,
                    supers,
                    defs,
                } => self.index_class(
                    *inheritable,
                    &namespace,
                    name,
                    parse_typarams(typarams),
                    supers,
                    defs,
                    &mut rust_methods,
                )?,
                shiika_ast::Definition::ModuleDefinition {
                    name,
                    typarams,
                    defs,
                } => self.index_module(
                    &namespace,
                    name,
                    parse_typarams(typarams),
                    defs,
                    &mut rust_methods,
                )?,
                shiika_ast::Definition::EnumDefinition {
                    name,
                    typarams,
                    cases,
                    defs,
                } => self.index_enum(
                    &namespace,
                    name,
                    parse_typarams(typarams),
                    cases,
                    defs,
                    &mut rust_methods,
                )?,
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
        inheritable: bool,
        namespace: &Namespace,
        firstname: &ClassFirstname,
        typarams: Vec<ty::TyParam>,
        supers: &[UnresolvedTypeName],
        defs: &[shiika_ast::Definition],
        rust_methods: &mut RustMethods,
    ) -> Result<()> {
        let inner_namespace = namespace.add(firstname.to_string());
        let fullname = namespace.class_fullname(firstname);
        let (superclass, includes) = if fullname.0 == "Object" {
            (None, vec![])
        } else {
            let (supercls, includes) = self._resolve_supers(namespace, &typarams, supers)?;
            (Some(supercls), includes)
        };

        let (instance_methods, class_methods) = self.index_defs_in_class(
            inheritable,
            &inner_namespace,
            &fullname,
            &typarams,
            &superclass,
            defs,
            rust_methods,
        )?;

        self.add_new_class(
            ClassSpec {
                namespace: namespace.clone(),
                fullname: fullname.clone(),
                typarams,
                superclass,
                includes,
                instance_methods,
                class_methods,
                inheritable,
                // `Void` is the only non-enum class whose const_is_obj=true
                const_is_obj: (fullname.0 == "Void"),
                // `Never` is the only class which cannot have an instance
                has_new: (fullname.0 != "Never"),
            },
            rust_methods,
        )?;
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
                    if !c.inheritable {
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
        rust_methods: &mut RustMethods,
    ) -> Result<()> {
        let fullname = namespace.module_fullname(firstname);
        let inner_namespace = namespace.add(firstname.to_string());
        let (instance_methods, class_methods, requirements) =
            self.index_defs_in_module(&inner_namespace, &fullname, &typarams, defs, rust_methods)?;

        self.add_new_module(
            namespace,
            &fullname,
            &typarams,
            instance_methods,
            class_methods,
            requirements,
            rust_methods,
        )?;
        Ok(())
    }

    /// Return parameters of `initialize`.
    fn _initializer_params(
        &self,
        fullname: &TypeFullname,
        superclass: &Option<Supertype>,
    ) -> Result<Vec<MethodParam>> {
        // Is it `Object.new`?
        if fullname.0 == "Object" {
            return Ok(vec![]);
        }
        // Does it have own `#initialize`?
        if let Ok(found) = self.lookup_method(
            &fullname.to_ty(),
            &method_firstname("initialize"),
            &LocationSpan::internal(),
        ) {
            return Ok(found.sig.params.clone());
        }
        // Does it inherits `#initialize`?
        if let Some(sup) = superclass {
            if let Ok(found) = self.lookup_method(
                &sup.to_term_ty(),
                &method_firstname("initialize"),
                &LocationSpan::internal(),
            ) {
                // Inherit #initialize from superclass
                return Ok(specialized_initialize(&found.sig, sup).params);
            }
        }
        // No initializer found, return empty params
        Ok(vec![])
    }

    fn index_enum(
        &mut self,
        namespace: &Namespace,
        firstname: &ClassFirstname,
        typarams: Vec<TyParam>,
        cases: &[shiika_ast::EnumCase],
        defs: &[shiika_ast::Definition],
        rust_methods: &mut RustMethods,
    ) -> Result<()> {
        let fullname = namespace.class_fullname(firstname);
        let inner_namespace = namespace.add(firstname.to_string());
        let (instance_methods, class_methods) = self.index_defs_in_class(
            false,
            &inner_namespace,
            &fullname,
            &typarams,
            &None,
            defs,
            rust_methods,
        )?;
        self.add_new_class(
            ClassSpec {
                namespace: namespace.clone(),
                fullname: fullname.clone(),
                typarams: typarams.clone(),
                superclass: Some(Supertype::simple("Object")),
                includes: Default::default(),
                instance_methods,
                class_methods,
                inheritable: false,
                const_is_obj: false,
                has_new: false,
            },
            rust_methods,
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
        let (superclass, case_typarams) = enum_case_superclass(enum_fullname, typarams, case);
        let (_, initialize_sig) = enum_case_new_sig(&ivar_list, &case_typarams, &fullname);

        let mut instance_methods = enum_case_getters(&fullname, &ivar_list);
        instance_methods.insert(initialize_sig);

        self.add_new_class(
            ClassSpec {
                namespace: namespace.clone(),
                fullname: fullname.clone(),
                typarams: case_typarams,
                superclass: Some(superclass),
                includes: Default::default(),
                instance_methods,
                class_methods: Default::default(),
                inheritable: false,
                const_is_obj: case.params.is_empty(),
                has_new: true,
            },
            &mut Default::default(),
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
        inheritable: bool,
        namespace: &Namespace,
        fullname: &ClassFullname,
        typarams: &[ty::TyParam],
        superclass: &Option<Supertype>,
        defs: &[shiika_ast::Definition],
        rust_methods: &mut RustMethods,
    ) -> Result<(MethodSignatures, MethodSignatures)> {
        let (instance_methods, class_methods, _) = self._index_inner_defs(
            inheritable,
            namespace,
            fullname.to_type_fullname(),
            typarams,
            superclass,
            defs,
            false,
            rust_methods,
        )?;
        Ok((instance_methods, class_methods))
    }

    fn index_defs_in_module(
        &mut self,
        namespace: &Namespace,
        fullname: &ModuleFullname,
        typarams: &[ty::TyParam],
        defs: &[shiika_ast::Definition],
        rust_methods: &mut RustMethods,
    ) -> Result<(MethodSignatures, MethodSignatures, Vec<MethodSignature>)> {
        self._index_inner_defs(
            false,
            namespace,
            fullname.to_type_fullname(),
            typarams,
            &None,
            defs,
            true,
            rust_methods,
        )
    }

    fn _index_inner_defs(
        &mut self,
        inheritable: bool,
        namespace: &Namespace,
        fullname: TypeFullname,
        typarams: &[ty::TyParam],
        superclass: &Option<Supertype>,
        defs: &[shiika_ast::Definition],
        is_module: bool,
        rust_methods: &mut RustMethods,
    ) -> Result<(MethodSignatures, MethodSignatures, Vec<MethodSignature>)> {
        let mut instance_methods = MethodSignatures::new();
        let mut class_methods = MethodSignatures::new();
        let mut requirements = vec![];
        for def in defs {
            match def {
                shiika_ast::Definition::InstanceMethodDefinition { sig, .. }
                | shiika_ast::Definition::InitializerDefinition(
                    shiika_ast::InitializerDefinition { sig, .. },
                ) => {
                    let hir_sig = self.create_maybe_virtual_signature(
                        inheritable,
                        namespace,
                        fullname.clone(),
                        sig,
                        typarams,
                        superclass,
                        false,
                    )?;
                    if sig.name.0 == "initialize" {
                        self._index_accessors(&mut instance_methods, sig, &hir_sig);
                    }
                    instance_methods.insert(hir_sig);
                }
                shiika_ast::Definition::ClassMethodDefinition { sig, .. } => {
                    let hir_sig = self.create_signature(
                        namespace,
                        fullname.meta_name().to_type_fullname(),
                        sig,
                        Default::default(),
                        false,
                        false,
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
                        false,
                        false,
                    )?;
                    class_methods.insert(hir_sig);
                }
                shiika_ast::Definition::ConstDefinition { .. } => (),
                shiika_ast::Definition::ClassDefinition {
                    inheritable,
                    name,
                    typarams,
                    supers,
                    defs,
                } => {
                    self.index_class(
                        *inheritable,
                        namespace,
                        name,
                        parse_typarams(typarams),
                        supers,
                        defs,
                        rust_methods,
                    )?;
                }
                shiika_ast::Definition::ModuleDefinition {
                    name,
                    typarams,
                    defs,
                } => {
                    self.index_module(
                        namespace,
                        name,
                        parse_typarams(typarams),
                        defs,
                        rust_methods,
                    )?;
                }
                shiika_ast::Definition::MethodRequirementDefinition { sig } => {
                    if is_module {
                        let hir_sig = self.create_signature(
                            namespace,
                            fullname.clone(),
                            sig,
                            typarams,
                            false,
                            false,
                        )?;
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
                    self.index_enum(
                        namespace,
                        name,
                        parse_typarams(typarams),
                        cases,
                        defs,
                        rust_methods,
                    )?;
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
                asyncness: Asyncness::Sync,
                is_virtual: false,
                is_rust: false,
            };
            instance_methods.insert(sig);
        }
    }

    /// Register a class and its metaclass to self
    fn add_new_class(&mut self, mut c: ClassSpec, rust_methods: &mut RustMethods) -> Result<()> {
        let fullname_ = c.fullname.to_type_fullname();
        c.instance_methods.append_vec(self.transfer_rust_methods(
            c.inheritable,
            &c.namespace,
            &fullname_,
            &c.typarams,
            &c.superclass,
            rust_methods,
        )?);
        let wtable = build_wtable(self, &c.instance_methods, &c.includes)?;

        let sk_type = {
            if self.known(&fullname_) {
                // predefined as bootstrap
            } else {
                let ivars = self.superclass_ivars(&c.superclass).unwrap_or_default();
                let base = SkTypeBase {
                    erasure: Erasure::nonmeta(&c.fullname.0),
                    typarams: c.typarams.clone(),
                    method_sigs: Default::default(),
                    foreign: false,
                };
                self.add_type(SkClass {
                    base,
                    superclass: c.superclass.clone(),
                    includes: Default::default(),
                    ivars, // may be overridden when processing `#initialize`
                    inheritable: Default::default(),
                    const_is_obj: c.const_is_obj,
                    wtable: Default::default(),
                });
            }
            self.sk_types.types.get_mut(&fullname_).unwrap()
        };
        let SkType::Class(sk_class) = sk_type else {
            unreachable!()
        };
        sk_class.wtable = wtable;
        sk_class.includes = c.includes;
        sk_class.inheritable = c.inheritable;
        sk_type.base_mut().method_sigs.append(c.instance_methods);

        // Create metaclass (which is a subclass of `Class`)
        let new_params = self._initializer_params(&fullname_, &c.superclass)?;
        let meta_name = c.fullname.meta_name().to_type_fullname();
        c.class_methods.append_vec(self.transfer_rust_methods(
            false,
            &c.namespace,
            &meta_name,
            &c.typarams,
            &Some(Supertype::simple("Class")),
            rust_methods,
        )?);
        let meta_type = {
            if self.known(&meta_name) {
                // predefined as bootstrap
            } else {
                let the_class = self.get_class(&class_fullname("Class"));
                let meta_ivars = the_class.ivars.clone();
                let base = SkTypeBase {
                    erasure: Erasure::meta(&c.fullname.0),
                    typarams: c.typarams.to_vec(),
                    method_sigs: Default::default(),
                    foreign: false,
                };
                self.add_type(SkClass {
                    base,
                    superclass: Some(Supertype::simple("Class")),
                    includes: Default::default(),
                    ivars: meta_ivars,
                    inheritable: false,
                    const_is_obj: false,
                    wtable: Default::default(),
                });
            }
            self.sk_types.types.get_mut(&meta_name).unwrap()
        };
        // Add `.new` to the metaclass
        if c.has_new
            && !meta_type
                .base()
                .method_sigs
                .contains_key(&method_firstname("new"))
        {
            c.class_methods.insert(signature_of_new(
                &c.fullname.meta_name(),
                new_params,
                c.typarams.to_vec(),
            ));
        }

        meta_type.base_mut().method_sigs.append(c.class_methods);
        Ok(())
    }

    /// Register a module and its metaclass to self
    fn add_new_module(
        &mut self,
        namespace: &Namespace,
        fullname: &ModuleFullname,
        typarams: &[ty::TyParam],
        mut instance_methods: MethodSignatures,
        mut class_methods: MethodSignatures,
        requirements: Vec<MethodSignature>,
        rust_methods: &mut RustMethods,
    ) -> Result<()> {
        // Register a module
        let fullname_ = fullname.to_type_fullname();
        instance_methods.append_vec(self.transfer_rust_methods(
            false,
            namespace,
            &fullname_,
            typarams,
            &None,
            rust_methods,
        )?);
        let sk_type = {
            if self.known(&fullname_) {
                // predefined as bootstrap
            } else {
                let base = SkTypeBase {
                    erasure: Erasure::nonmeta(&fullname.0),
                    typarams: typarams.to_vec(),
                    method_sigs: Default::default(),
                    foreign: false,
                };
                self.add_type(SkModule::new(base, requirements));
            }
            self.sk_types.types.get_mut(&fullname_).unwrap()
        };
        sk_type.base_mut().method_sigs.append(instance_methods);

        // Register its metaclass
        let meta_name = fullname.meta_name().to_type_fullname();
        class_methods.append_vec(self.transfer_rust_methods(
            false,
            namespace,
            &meta_name,
            typarams,
            &None,
            rust_methods,
        )?);
        let meta_type = {
            if self.known(&meta_name) {
                // predefined as bootstrap
            } else {
                let the_class = self.get_class(&class_fullname("Class"));
                let meta_ivars = the_class.ivars.clone();
                let base = SkTypeBase {
                    erasure: Erasure::meta(&fullname.0),
                    typarams: typarams.to_vec(),
                    method_sigs: Default::default(),
                    foreign: false,
                };
                self.add_type(SkClass {
                    base,
                    superclass: Some(Supertype::simple("Class")),
                    includes: Default::default(),
                    ivars: meta_ivars,
                    inheritable: false,
                    const_is_obj: false,
                    wtable: Default::default(),
                });
            }
            self.sk_types.types.get_mut(&meta_name).unwrap()
        };
        meta_type.base_mut().method_sigs.append(class_methods);
        Ok(())
    }

    /// Checks if the method is virtual and returns the signature.
    pub fn create_maybe_virtual_signature(
        &self,
        inheritable: bool,
        namespace: &Namespace,
        fullname: TypeFullname,
        sig: &shiika_ast::AstMethodSignature,
        typarams: &[ty::TyParam],
        superclass: &Option<Supertype>,
        is_rust: bool,
    ) -> Result<MethodSignature> {
        let is_virtual = if inheritable {
            true
        } else if let Some(superclass) = superclass {
            self.try_lookup_method(&superclass.to_term_ty(), &sig.name)
                .is_some()
        } else {
            false
        };
        self.create_signature(
            namespace,
            fullname.clone(),
            sig,
            typarams,
            is_virtual,
            is_rust,
        )
    }

    /// Convert AstMethodSignature to MethodSignature
    pub fn create_signature(
        &self,
        // Used to resolve type names
        namespace: &Namespace,
        type_fullname: TypeFullname,
        sig: &shiika_ast::AstMethodSignature,
        class_typarams: &[ty::TyParam],
        is_virtual: bool,
        is_rust: bool,
    ) -> Result<MethodSignature> {
        let method_typarams = parse_typarams(&sig.typarams);
        let fullname = method_fullname(type_fullname, &sig.name.0);
        let ret_ty = if let Some(typ) = &sig.ret_typ {
            self.resolve_typename(namespace, class_typarams, &method_typarams, typ)?
        } else {
            ty::raw("Void") // Default return type.
        };
        let asyncness = if is_virtual {
            Asyncness::Async
        } else {
            Asyncness::Unknown
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
            asyncness,
            is_virtual,
            is_rust,
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

    fn transfer_rust_methods(
        &mut self,
        inheritable: bool,
        namespace: &Namespace,
        typename: &TypeFullname,
        typarams: &[ty::TyParam],
        superclass: &Option<Supertype>,
        rust_methods: &mut RustMethods,
    ) -> Result<Vec<MethodSignature>> {
        let v = rust_methods.remove(typename).unwrap_or_default();
        v.into_iter()
            .map(|(sig, is_async)| {
                let mut hir_sig = self.create_maybe_virtual_signature(
                    inheritable,
                    namespace,
                    typename.clone(),
                    &sig,
                    typarams,
                    superclass,
                    true,
                )?;
                if !is_async && hir_sig.is_virtual {
                    return Err(error::program_error(&format!(
                        "method {} must be async because it is virtual",
                        hir_sig.fullname
                    )));
                }
                hir_sig.asyncness = if is_async {
                    Asyncness::Async
                } else {
                    Asyncness::Sync
                };
                Ok(hir_sig)
            })
            .collect()
    }

    fn known(&self, fullname: &TypeFullname) -> bool {
        self.sk_types.types.contains_key(fullname)
    }
}

/// Returns superclass of a enum case
fn enum_case_superclass(
    enum_fullname: &ClassFullname,
    typarams: &[ty::TyParam],
    case: &shiika_ast::EnumCase,
) -> (Supertype, Vec<ty::TyParam>) {
    let mut case_typarams = vec![];
    let tyargs = typarams
        .iter()
        .enumerate()
        .map(|(i, t)| {
            if case.appears(&t.name) {
                case_typarams.push(t.clone());
                ty::typaram_ref(&t.name, TyParamKind::Class, i).into_term_ty()
            } else {
                ty::raw("Never")
            }
        })
        .collect::<Vec<_>>();
    let supertype = Supertype::from_ty(LitTy::new(enum_fullname.0.clone(), tyargs, false));
    (supertype, case_typarams)
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
        signature_of_new(&fullname.meta_name(), params.clone(), typarams.to_vec()),
        signature_of_enum_initialize(fullname, params),
    )
}

/// Create signatures of getters of an enum case
fn enum_case_getters(case_fullname: &ClassFullname, ivars: &[SkIVar]) -> MethodSignatures {
    let iter = ivars.iter().map(|ivar| MethodSignature {
        fullname: method_fullname(case_fullname.to_type_fullname(), &ivar.accessor_name()),
        ret_ty: ivar.ty.clone(),
        params: Default::default(),
        typarams: Default::default(),
        asyncness: Asyncness::Unknown,
        is_virtual: false,
        is_rust: false,
    });
    MethodSignatures::from_iterator(iter)
}
