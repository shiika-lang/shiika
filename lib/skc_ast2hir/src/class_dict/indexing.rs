use crate::class_dict::build_wtable::build_wtable;
use crate::class_dict::*;
use crate::convert_exprs::params;
use crate::error;
use crate::parse_typarams;
use anyhow::Result;
use shiika_ast::{self, AstMethodSignature, LocationSpan, UnresolvedTypeName};
use shiika_core::{names::*, ty, ty::*};
use skc_error::{self, Label};
use skc_hir::method_signature::{signature_of_new, MethodSignature};
use skc_hir::*;
use std::collections::HashMap;

/// Used during indexing: signatures of `native def`s (and whether it is
/// `native_async`) collected from the class body, keyed by the type name
/// (`Meta:Foo` for class methods). They are converted to `MethodSignature`s
/// when the type is registered (see `transfer_rust_methods`).
type RustMethods = HashMap<TypeFullname, Vec<(AstMethodSignature, bool, Vec<WhenCond>)>>;

/// An `implement M ... end` block in a class body
struct ImplementBlock<'a> {
    module: &'a UnresolvedTypeName,
    /// Names of the methods defined in the block
    method_names: Vec<MethodFirstname>,
    /// Conditions of the `when` block which this block is in (conditional
    /// conformance; empty if not in a `when` block)
    when_conds: Vec<WhenCond>,
}

struct ClassSpec {
    namespace: Namespace,
    fullname: ClassFullname,
    typarams: Vec<ty::TyParam>,
    superclass: Option<Supertype>,
    includes: Vec<Supertype>,
    include_conds: HashMap<String, Vec<WhenCond>>,
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
        allow_native: bool,
    ) -> Result<()> {
        let mut rust_methods = RustMethods::new();
        *self.pending_bound_checks.borrow_mut() = Some(vec![]);
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
                    self.resolve_typarams(&namespace, typarams, Default::default())?,
                    supers,
                    defs,
                    &mut rust_methods,
                )?,
                shiika_ast::Definition::ModuleDefinition {
                    name,
                    typarams,
                    supers,
                    defs,
                } => self.index_module(
                    &namespace,
                    name,
                    self.resolve_typarams(&namespace, typarams, Default::default())?,
                    supers,
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
                    self.resolve_typarams(&namespace, typarams, Default::default())?,
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
        self.check_virtual_asyncness()?;
        if !allow_native {
            if let Some(sig) = find_native_def(toplevel_defs.iter().copied()) {
                return Err(error::program_error(&format!(
                    "`native def {}' is only allowed in a package which has rust_libs",
                    sig.name
                )));
            }
        }
        let annotations = self.pending_bound_checks.borrow_mut().take().unwrap();
        for (ty, locs) in annotations {
            crate::type_system::bounds::check_annotation(self, &ty, &locs)?;
        }
        crate::type_system::bounds::check_sk_types(self)?;
        crate::type_system::supermodules::check_sk_types(self)?;
        Ok(())
    }

    /// Convert `AstTyParam`s to `TyParam`s, resolving the upper bound
    /// (eg. `Ord` of `class Foo<T: Ord>`) with `resolve_typename`.
    /// `outer_typarams` is the class typarams when resolving method typarams
    /// (used only to detect a typaram used as a bound, which is an error).
    fn resolve_typarams(
        &self,
        namespace: &Namespace,
        ast_typarams: &[shiika_ast::AstTyParam],
        outer_typarams: &[ty::TyParam],
    ) -> Result<Vec<ty::TyParam>> {
        let plain = parse_typarams(ast_typarams);
        ast_typarams
            .iter()
            .zip(plain.iter())
            .map(|(ast_tp, tp)| {
                let mut tp = tp.clone();
                if let Some(bound_name) = &ast_tp.upper_bound {
                    let bound_ty =
                        self.resolve_typename(namespace, outer_typarams, &plain, bound_name)?;
                    if bound_ty.contains_typaram_ref() {
                        let msg = format!(
                            "a type parameter cannot appear in the bound of a type parameter \
                             (in the bound of `{}'; F-bounded polymorphism is not supported)",
                            &tp.name
                        );
                        let report =
                            skc_error::build_report(msg.clone(), &bound_name.locs, |r, span| {
                                r.with_label(Label::new(span).with_message(msg.clone()))
                            });
                        return Err(error::program_error(&report));
                    }
                    match &bound_ty.body {
                        TyBody::TyRaw(lit_ty) => tp.upper_bound = lit_ty.clone(),
                        TyBody::TyPara(_) => unreachable!(), // excluded above
                    }
                }
                Ok(tp)
            })
            .collect()
    }

    /// Raise error if a (Rust-implemented) sync method overrides an async method.
    fn check_virtual_asyncness(&self) -> Result<()> {
        for sk_type in self.sk_types.types.values() {
            let SkType::Class(sk_class) = sk_type else {
                continue;
            };
            for (sig, _) in sk_type.base().method_sigs.to_ordered() {
                // Only sync methods can violate the invariant
                if sig.asyncness != Asyncness::Sync {
                    continue;
                }
                let Some(found) = self.lookup_ancestor_method(sk_class, &sig.fullname.first_name)
                else {
                    continue;
                };
                if found.sig.asyncness == Asyncness::Async {
                    return Err(error::program_error(&format!(
                        "method {} must be async because it overrides async method {}",
                        sig.fullname, found.sig.fullname
                    )));
                }
            }
        }
        Ok(())
    }

    /// Look up `method_name` in the ancestors (included modules and superclass
    /// chain) of `sk_class`, skipping `sk_class` itself.
    fn lookup_ancestor_method(
        &self,
        sk_class: &SkClass,
        method_name: &MethodFirstname,
    ) -> Option<FoundMethod> {
        for modinfo in &sk_class.includes {
            if let Some(found) =
                self.find_method(&modinfo.erasure().to_type_fullname(), method_name)
            {
                return Some(found);
            }
        }
        let super_ty = sk_class.superclass.as_ref()?.to_term_ty();
        self.try_lookup_method(&super_ty, method_name)
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
        check_reserved_type_name(&firstname.0)?;
        let inner_namespace = namespace.add(firstname.to_string());
        let fullname = namespace.class_fullname(firstname);
        let superclass = if fullname.0 == "Object" {
            None
        } else {
            Some(self._resolve_superclass(namespace, &typarams, supers)?)
        };

        let (instance_methods, class_methods, implements) = self.index_defs_in_class(
            inheritable,
            &inner_namespace,
            &fullname,
            &typarams,
            &superclass,
            defs,
            rust_methods,
        )?;
        let (includes, include_conds) = self._resolve_implements(
            &inner_namespace,
            &fullname,
            &typarams,
            &superclass,
            &instance_methods,
            &implements,
        )?;

        self.add_new_class(
            ClassSpec {
                namespace: namespace.clone(),
                fullname: fullname.clone(),
                typarams,
                superclass,
                includes,
                include_conds,
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

    /// Resolve the superclass of a class definition
    fn _resolve_superclass(
        &self,
        namespace: &Namespace,
        class_typarams: &[ty::TyParam],
        supers: &[UnresolvedTypeName],
    ) -> Result<Supertype> {
        let mut superclass = None;
        for name in supers {
            let ty = self.resolve_typename(namespace, class_typarams, Default::default(), name)?;
            crate::type_system::variance::check_supertype(self, class_typarams, &ty, &name.locs)?;
            match self.find_type(&ty.erasure().to_type_fullname()) {
                Some(SkType::Class(c)) => {
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
                Some(SkType::Module(_)) => {
                    return Err(located_error(
                        format!(
                            "`{}' is a module; use `implement {}' in the class body to include it",
                            name, name
                        ),
                        &name.locs,
                    ));
                }
                None => {
                    return Err(error::program_error(&format!(
                        "unknown class or module {}",
                        ty
                    )));
                }
            }
        }
        Ok(superclass.unwrap_or_else(Supertype::default))
    }

    /// Resolve the modules of the `implement` blocks of a class and check
    /// that each block defines exactly the methods of the module.
    fn _resolve_implements(
        &self,
        namespace: &Namespace,
        fullname: &ClassFullname,
        class_typarams: &[ty::TyParam],
        superclass: &Option<Supertype>,
        instance_methods: &MethodSignatures,
        implements: &[ImplementBlock],
    ) -> Result<(Vec<Supertype>, HashMap<String, Vec<WhenCond>>)> {
        let mut modules: Vec<Supertype> = vec![];
        let mut include_conds = HashMap::new();
        for blk in implements {
            let name = blk.module;
            let ty = self.resolve_typename(namespace, class_typarams, Default::default(), name)?;
            crate::type_system::variance::check_supertype(self, class_typarams, &ty, &name.locs)?;
            let sk_module = match self.find_type(&ty.erasure().to_type_fullname()) {
                Some(SkType::Module(m)) => m,
                Some(SkType::Class(_)) => {
                    return Err(located_error(
                        format!("`{}' is a class; only a module can be implemented", name),
                        &name.locs,
                    ));
                }
                None => {
                    return Err(located_error(
                        format!("unknown module `{}'", name),
                        &name.locs,
                    ));
                }
            };
            let lit_ty = match &ty.body {
                TyBody::TyPara(_) => {
                    return Err(located_error(
                        format!("type parameter `{}' cannot be implemented", name),
                        &name.locs,
                    ));
                }
                TyBody::TyRaw(lit_ty) => lit_ty.clone(),
            };
            let module_name = sk_module.fullname().to_type_fullname();
            if modules
                .iter()
                .any(|m| m.erasure().to_type_fullname() == module_name)
            {
                return Err(located_error(
                    format!("`implement {}' appears twice in `{}'", name, fullname),
                    &name.locs,
                ));
            }
            if let Some(owner) = self._ancestor_implementing(superclass, &module_name) {
                return Err(located_error(
                    format!(
                        "`{}' is already implemented by `{}', the superclass of `{}'\n  \
                         hint: override its methods outside of `implement'",
                        name, owner, fullname
                    ),
                    &name.locs,
                ));
            }
            for method_name in &blk.method_names {
                if !sk_module.base.method_sigs.contains_key(method_name) {
                    return Err(located_error(
                        format!(
                            "`{}' is not a method of `{}'\n  \
                             hint: define it outside of `implement {}'",
                            method_name, name, name
                        ),
                        &name.locs,
                    ));
                }
            }
            for req in &sk_module.requirements {
                let req_name = &req.fullname.first_name;
                if blk.method_names.contains(req_name) {
                    continue;
                }
                let msg = if instance_methods.contains_key(req_name) {
                    format!(
                        "`{}' is a requirement of `{}'; define it in `implement {}'",
                        req_name, name, name
                    )
                } else {
                    format!(
                        "`implement {}' in `{}' does not define the required method `{}'",
                        name, fullname, req_name
                    )
                };
                return Err(located_error(msg, &name.locs));
            }
            if blk.when_conds.is_empty() {
                modules.push(Supertype::from_ty(lit_ty));
            } else {
                // `T == X` replaces `T` with `X` (eg. `Foo<String>` for
                // `implement Foo<T>` in `when T == String`)
                let eq_tyargs = when_body_tyargs(class_typarams, &blk.when_conds, true);
                modules.push(Supertype::from_ty(lit_ty.substitute_with_self(&eq_tyargs)));
                include_conds.insert(module_name.0.clone(), blk.when_conds.clone());
            }
        }
        Ok((modules, include_conds))
    }

    /// Resolve the conditions of a `when` block
    fn _resolve_when_conds(
        &self,
        namespace: &Namespace,
        owner: &TypeFullname,
        typarams: &[ty::TyParam],
        conds: &[shiika_ast::AstWhenCond],
    ) -> Result<Vec<WhenCond>> {
        let mut v: Vec<WhenCond> = vec![];
        for cond in conds {
            let Some(idx) = typarams.iter().position(|tp| tp.name == cond.typaram) else {
                return Err(located_error(
                    format!("`{}' is not a type parameter of `{}'", cond.typaram, owner),
                    &cond.locs,
                ));
            };
            let typaram = ty::typaram_ref(&typarams[idx], TyParamKind::Class, idx).into_term_ty();
            if v.iter().any(|c| c.typaram == typaram) {
                return Err(located_error(
                    format!(
                        "type parameter `{}' appears more than once in `when'",
                        cond.typaram
                    ),
                    &cond.locs,
                ));
            }
            let ty = self.resolve_typename(namespace, typarams, Default::default(), &cond.ty)?;
            if ty.contains_typaram_ref() {
                return Err(located_error(
                    format!(
                        "the type `{}' in `when' cannot contain type parameters (not supported yet)",
                        ty
                    ),
                    &cond.locs,
                ));
            }
            let kind = if cond.is_eq {
                WhenCondKind::Eq
            } else {
                WhenCondKind::Bound
            };
            v.push(WhenCond { typaram, kind, ty });
        }
        Ok(v)
    }

    /// A method which overrides a method in a `when` block must be in a `when`
    /// block of the same conditions (and vice versa)
    fn _check_when_override(
        &self,
        superclass: &Option<Supertype>,
        sig: &MethodSignature,
    ) -> Result<()> {
        let Some(sup) = superclass else {
            return Ok(());
        };
        let Some(found) = self.try_lookup_method(&sup.to_term_ty(), &sig.fullname.first_name)
        else {
            return Ok(());
        };
        let parent = found.sig.specialize(sup.type_args(), Default::default());
        if parent.when_conds != sig.when_conds {
            return Err(error::program_error(&format!(
                "`{}' overrides `{}' but the conditions of `when' differ",
                sig, parent
            )));
        }
        Ok(())
    }

    /// Returns the nearest ancestor class which implements `module_name`
    fn _ancestor_implementing(
        &self,
        superclass: &Option<Supertype>,
        module_name: &TypeFullname,
    ) -> Option<ClassFullname> {
        let mut sup = superclass.as_ref().map(|s| s.erasure().to_class_fullname());
        while let Some(name) = sup {
            let sk_class = self.get_class(&name);
            if sk_class
                .includes
                .iter()
                .any(|m| &m.erasure().to_type_fullname() == module_name)
            {
                return Some(name);
            }
            sup = sk_class
                .superclass
                .as_ref()
                .map(|s| s.erasure().to_class_fullname());
        }
        None
    }

    fn index_module(
        &mut self,
        namespace: &Namespace,
        firstname: &ModuleFirstname,
        typarams: Vec<ty::TyParam>,
        supers: &[UnresolvedTypeName],
        defs: &[shiika_ast::Definition],
        rust_methods: &mut RustMethods,
    ) -> Result<()> {
        check_reserved_type_name(&firstname.0)?;
        let fullname = namespace.module_fullname(firstname);
        let inner_namespace = namespace.add(firstname.to_string());
        let supers = self._resolve_module_supers(namespace, &fullname, &typarams, supers)?;
        let (instance_methods, class_methods, requirements) =
            self.index_defs_in_module(&inner_namespace, &fullname, &typarams, defs, rust_methods)?;

        self.add_new_module(
            namespace,
            &fullname,
            &typarams,
            supers,
            instance_methods,
            class_methods,
            requirements,
            rust_methods,
        )?;
        Ok(())
    }

    /// Resolve prerequisite module names of a module definition
    /// (eg. `Equatable<X>` of `module Comparable<X> : Equatable<X>`)
    fn _resolve_module_supers(
        &self,
        namespace: &Namespace,
        fullname: &ModuleFullname,
        module_typarams: &[ty::TyParam],
        supers: &[UnresolvedTypeName],
    ) -> Result<Vec<Supertype>> {
        let mut modules: Vec<Supertype> = vec![];
        for name in supers {
            let ty = self.resolve_typename(namespace, module_typarams, Default::default(), name)?;
            let lit_ty = match &ty.body {
                TyBody::TyPara(_) => {
                    return Err(located_error(
                        format!("type parameter `{}' cannot be a prerequisite module", ty),
                        &name.locs,
                    ));
                }
                TyBody::TyRaw(lit_ty) => lit_ty.clone(),
            };
            if lit_ty.erasure().to_type_fullname() == fullname.to_type_fullname() {
                return Err(located_error(
                    format!("module `{}' cannot require itself", fullname),
                    &name.locs,
                ));
            }
            match self.find_type(&lit_ty.erasure().to_type_fullname()) {
                Some(SkType::Class(_)) => {
                    return Err(located_error(
                        format!(
                            "module cannot have a superclass: `{}' is a class",
                            lit_ty.erasure().to_type_fullname()
                        ),
                        &name.locs,
                    ));
                }
                Some(SkType::Module(_)) => {}
                None => {
                    return Err(located_error(
                        format!("unknown class or module {}", ty),
                        &name.locs,
                    ));
                }
            }
            crate::type_system::variance::check_supertype(self, module_typarams, &ty, &name.locs)?;
            if modules.iter().any(|m| m.erasure() == lit_ty.erasure()) {
                return Err(located_error(
                    format!("module `{}' is required twice", lit_ty.erasure()),
                    &name.locs,
                ));
            }
            modules.push(Supertype::from_ty(lit_ty));
        }
        Ok(modules)
    }

    /// Returns `supers` and their prerequisite modules (transitively), each
    /// specialized with the type parameters of the module being indexed.
    /// Raises an error if a module appears with different type arguments
    /// (eg. both `M0<Int>` and `M0<String>`).
    fn _all_module_supers(
        &self,
        fullname: &ModuleFullname,
        supers: &[Supertype],
    ) -> Result<Vec<Supertype>> {
        let mut v: Vec<Supertype> = vec![];
        let mut stack = supers.iter().rev().cloned().collect::<Vec<_>>();
        while let Some(sup) = stack.pop() {
            if let Some(existing) = v.iter().find(|m| m.erasure() == sup.erasure()) {
                if existing != &sup {
                    return Err(error::program_error(format!(
                        "module `{}' requires both `{}' and `{}'",
                        fullname,
                        existing.to_term_ty(),
                        sup.to_term_ty()
                    )));
                }
                continue;
            }
            let sk_module = self.get_module(&sup.erasure().to_module_fullname());
            let subs = sk_module.specialized_supers(sup.type_args());
            stack.extend(subs.into_iter().rev());
            v.push(sup);
        }
        Ok(v)
    }

    /// Raise error if methods of a module conflict with those of its
    /// prerequisite modules, or methods of prerequisite modules conflict
    /// with each other.
    fn _check_module_method_conflicts(
        &self,
        fullname: &ModuleFullname,
        supers: &[Supertype],
        instance_methods: &MethodSignatures,
        requirements: &[MethodSignature],
    ) -> Result<()> {
        if supers.is_empty() {
            return Ok(());
        }
        let mut owners: HashMap<MethodFirstname, Erasure> = HashMap::new();
        for sup in self._all_module_supers(fullname, supers)? {
            let sk_module = self.get_module(&sup.erasure().to_module_fullname());
            for sig in sk_module.base.method_sigs.iter() {
                let name = &sig.fullname.first_name;
                if let Some(owner) = owners.get(name) {
                    return Err(error::program_error(format!(
                        "method `{}' of module `{}' conflicts with the one of `{}' \
                         (both are required by `{}')",
                        name,
                        sup.erasure(),
                        owner,
                        fullname
                    )));
                }
                owners.insert(name.clone(), sup.erasure());
            }
        }
        let own_methods = instance_methods.iter().chain(requirements.iter());
        for sig in own_methods {
            let name = &sig.fullname.first_name;
            if let Some(owner) = owners.get(name) {
                return Err(error::program_error(format!(
                    "module `{}' cannot define method `{}' because it is defined in \
                     the prerequisite module `{}'",
                    fullname, name, owner
                )));
            }
        }
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
        check_reserved_type_name(&firstname.0)?;
        let fullname = namespace.class_fullname(firstname);
        let inner_namespace = namespace.add(firstname.to_string());
        let (instance_methods, class_methods, implements) = self.index_defs_in_class(
            false,
            &inner_namespace,
            &fullname,
            &typarams,
            &None,
            defs,
            rust_methods,
        )?;
        if let Some(blk) = implements.first() {
            return Err(located_error(
                format!(
                    "enum `{}' cannot implement a module (not supported yet)",
                    fullname
                ),
                &blk.module.locs,
            ));
        }
        self.add_new_class(
            ClassSpec {
                namespace: namespace.clone(),
                fullname: fullname.clone(),
                typarams: typarams.clone(),
                superclass: Some(Supertype::simple("Object")),
                includes: Default::default(),
                include_conds: Default::default(),
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
                include_conds: Default::default(),
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
            crate::type_system::variance::check_enum_case_ivar(
                self,
                typarams,
                &ty,
                &param.typ.locs,
            )?;
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

    fn index_defs_in_class<'a>(
        &mut self,
        inheritable: bool,
        namespace: &Namespace,
        fullname: &ClassFullname,
        typarams: &[ty::TyParam],
        superclass: &Option<Supertype>,
        defs: &'a [shiika_ast::Definition],
        rust_methods: &mut RustMethods,
    ) -> Result<(MethodSignatures, MethodSignatures, Vec<ImplementBlock<'a>>)> {
        let (instance_methods, class_methods, _, implements) = self._index_inner_defs(
            inheritable,
            namespace,
            fullname.to_type_fullname(),
            typarams,
            superclass,
            defs,
            false,
            rust_methods,
        )?;
        Ok((instance_methods, class_methods, implements))
    }

    fn index_defs_in_module(
        &mut self,
        namespace: &Namespace,
        fullname: &ModuleFullname,
        typarams: &[ty::TyParam],
        defs: &[shiika_ast::Definition],
        rust_methods: &mut RustMethods,
    ) -> Result<(MethodSignatures, MethodSignatures, Vec<MethodSignature>)> {
        let (instance_methods, class_methods, requirements, implements) = self._index_inner_defs(
            false,
            namespace,
            fullname.to_type_fullname(),
            typarams,
            &None,
            defs,
            true,
            rust_methods,
        )?;
        if let Some(blk) = implements.first() {
            return Err(located_error(
                format!(
                    "module `{}' cannot implement a module; use `module {} : {}' to require it",
                    fullname, fullname, blk.module
                ),
                &blk.module.locs,
            ));
        }
        Ok((instance_methods, class_methods, requirements))
    }

    #[allow(clippy::type_complexity)]
    fn _index_inner_defs<'a>(
        &mut self,
        inheritable: bool,
        namespace: &Namespace,
        fullname: TypeFullname,
        typarams: &[ty::TyParam],
        superclass: &Option<Supertype>,
        defs: &'a [shiika_ast::Definition],
        is_module: bool,
        rust_methods: &mut RustMethods,
    ) -> Result<(
        MethodSignatures,
        MethodSignatures,
        Vec<MethodSignature>,
        Vec<ImplementBlock<'a>>,
    )> {
        let mut instance_methods = MethodSignatures::new();
        let mut class_methods = MethodSignatures::new();
        let mut requirements = vec![];
        let mut natives = vec![];
        let mut implements = vec![];
        // Flatten the blocks, keeping the `when` conditions of each def
        let mut items: Vec<(&shiika_ast::Definition, Vec<WhenCond>)> = vec![];
        for def in defs {
            match def {
                shiika_ast::Definition::ImplementDefinition {
                    module,
                    defs: inner,
                } => {
                    implements.push(implement_block(module, inner, vec![])?);
                    items.extend(inner.iter().map(|d| (d, vec![])));
                }
                shiika_ast::Definition::WhenDefinition { conds, defs: inner } => {
                    let when_conds =
                        self._resolve_when_conds(namespace, &fullname, typarams, conds)?;
                    check_when_block(conds, inner)?;
                    for d in inner {
                        if let shiika_ast::Definition::ImplementDefinition {
                            module,
                            defs: inner2,
                        } = d
                        {
                            // Conditional conformance
                            implements.push(implement_block(module, inner2, when_conds.clone())?);
                            items.extend(inner2.iter().map(|d2| (d2, when_conds.clone())));
                        } else {
                            items.push((d, when_conds.clone()));
                        }
                    }
                }
                _ => items.push((def, vec![])),
            }
        }
        check_when_method_names(&fullname, &items)?;
        for (def, when_conds) in items {
            match def {
                shiika_ast::Definition::InstanceMethodDefinition { sig, .. }
                | shiika_ast::Definition::InitializerDefinition(
                    shiika_ast::InitializerDefinition { sig, .. },
                ) => {
                    let intrinsic = match fullname.0.as_str() {
                        "Int" | "Float" => {
                            matches!(sig.name.0.as_str(), "<" | "<=" | ">" | ">=" | "==")
                        }
                        "Bool" => sig.name.0 == "==",
                        _ => false,
                    };
                    if intrinsic && self.imported_classes.get_type(&fullname).is_some() {
                        return Err(error::program_error(&format!(
                            "cannot redefine intrinsic method {}#{}",
                            fullname, sig.name
                        )));
                    }
                    let prev = self.set_self_ty(
                        is_module.then(|| ClassDict::module_self_ty(&fullname, typarams)),
                    );
                    let hir_sig = self.create_maybe_virtual_signature(
                        inheritable || is_module,
                        namespace,
                        fullname.clone(),
                        sig,
                        typarams,
                        superclass,
                        false,
                    );
                    self.set_self_ty(prev);
                    let hir_sig = apply_when_conds(hir_sig?, typarams, when_conds);
                    crate::type_system::variance::check_method_signature(
                        self, typarams, sig, &hir_sig,
                    )?;
                    self._check_when_override(superclass, &hir_sig)?;
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
                        self.resolve_typarams(namespace, typarams, Default::default())?,
                        supers,
                        defs,
                        rust_methods,
                    )?;
                }
                shiika_ast::Definition::ModuleDefinition {
                    name,
                    typarams,
                    supers,
                    defs,
                } => {
                    self.index_module(
                        namespace,
                        name,
                        self.resolve_typarams(namespace, typarams, Default::default())?,
                        supers,
                        defs,
                        rust_methods,
                    )?;
                }
                shiika_ast::Definition::MethodRequirementDefinition { sig } => {
                    if is_module {
                        let prev =
                            self.set_self_ty(Some(ClassDict::module_self_ty(&fullname, typarams)));
                        let hir_sig = self.create_signature(
                            namespace,
                            fullname.clone(),
                            sig,
                            typarams,
                            false,
                            false,
                        );
                        self.set_self_ty(prev);
                        let hir_sig = hir_sig?;
                        crate::type_system::variance::check_method_signature(
                            self, typarams, sig, &hir_sig,
                        )?;
                        crate::type_system::variance::check_self_in_requirement(
                            self, sig, &hir_sig,
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
                        self.resolve_typarams(namespace, typarams, Default::default())?,
                        cases,
                        defs,
                        rust_methods,
                    )?;
                }
                shiika_ast::Definition::ImplementDefinition { .. }
                | shiika_ast::Definition::WhenDefinition { .. } => {
                    unreachable!("flattened above")
                }
                shiika_ast::Definition::NativeMethodDefinition {
                    sig,
                    is_class_method,
                    is_async,
                } => {
                    let type_name = if *is_class_method {
                        fullname.meta_name().to_type_fullname()
                    } else {
                        fullname.clone()
                    };
                    natives.push((type_name, sig.clone(), *is_async, when_conds));
                }
            }
        }
        for (type_name, sig, is_async, when_conds) in natives {
            let defined = if type_name == fullname {
                &instance_methods
            } else {
                &class_methods
            };
            let entry = rust_methods.entry(type_name.clone()).or_default();
            if defined.contains_key(&sig.name) || entry.iter().any(|(s, _, _)| s.name == sig.name) {
                return Err(error::program_error(&format!(
                    "method `{}' of `{}' is defined twice (`def' and `native def' cannot share a name)",
                    sig.name, type_name
                )));
            }
            entry.push((sig, is_async, when_conds));
        }
        Ok((instance_methods, class_methods, requirements, implements))
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
                when_conds: vec![],
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
        let class_ty = ty::nonmeta(
            &c.fullname.0,
            ty::typarams_to_typaram_refs(&c.typarams, TyParamKind::Class, false)
                .into_iter()
                .map(|t| t.into_term_ty())
                .collect(),
        );
        let wtable = build_wtable(
            self,
            &class_ty,
            &c.typarams,
            &c.instance_methods,
            &c.superclass,
            &c.includes,
            &c.include_conds,
        )?;

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
                    include_conds: Default::default(),
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
        sk_class.include_conds = c.include_conds;
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
                    include_conds: Default::default(),
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
        supers: Vec<Supertype>,
        mut instance_methods: MethodSignatures,
        mut class_methods: MethodSignatures,
        requirements: Vec<MethodSignature>,
        rust_methods: &mut RustMethods,
    ) -> Result<()> {
        // Register a module
        let fullname_ = fullname.to_type_fullname();
        instance_methods.append_vec(self.transfer_rust_methods(
            true,
            namespace,
            &fullname_,
            typarams,
            &None,
            rust_methods,
        )?);
        self._check_module_method_conflicts(fullname, &supers, &instance_methods, &requirements)?;
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
                self.add_type(SkModule::new(base, requirements, vec![]));
            }
            self.sk_types.types.get_mut(&fullname_).unwrap()
        };
        // May not be a module when predefined as bootstrap
        // (eg. `Shiika::Internal::Ptr` is a class)
        if let SkType::Module(sk_module) = sk_type {
            sk_module.supers = supers;
        }
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
                    include_conds: Default::default(),
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
        extendable: bool,
        namespace: &Namespace,
        fullname: TypeFullname,
        sig: &shiika_ast::AstMethodSignature,
        typarams: &[ty::TyParam],
        superclass: &Option<Supertype>,
        is_rust: bool,
    ) -> Result<MethodSignature> {
        let is_virtual = if extendable {
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
        let method_typarams = self.resolve_typarams(namespace, &sig.typarams, class_typarams)?;
        if let Some(tp) = sig
            .typarams
            .iter()
            .find(|t| t.variance != shiika_ast::AstVariance::Invariant)
        {
            let msg = format!(
                "method type parameter `{}' cannot have variance (`in'/`out')",
                &tp.name
            );
            let report = skc_error::build_report(msg.clone(), &tp.locs, |r, span| {
                r.with_label(Label::new(span).with_message(msg))
            });
            return Err(error::program_error(report));
        }
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
            when_conds: vec![],
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
        if name.names.len() == 1 && name.names[0] == "Self" {
            return self.resolve_self_type(name);
        }
        // Check it is a typaram
        if name.args.is_empty() && name.names.len() == 1 {
            let s = name.names.first().unwrap();
            if let Some(idx) = class_typarams.iter().position(|t| *s == t.name) {
                return Ok(
                    ty::typaram_ref(&class_typarams[idx], TyParamKind::Class, idx).into_term_ty(),
                );
            } else if let Some(idx) = method_typarams.iter().position(|t| *s == t.name) {
                return Ok(
                    ty::typaram_ref(&method_typarams[idx], TyParamKind::Method, idx).into_term_ty(),
                );
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
        let ty = ty::nonmeta(&resolved_base.join("::"), tyargs);
        if let Some(checks) = self.pending_bound_checks.borrow_mut().as_mut() {
            checks.push((ty.clone(), name.locs.clone()));
        }
        Ok(ty)
    }

    /// Resolve `Self`
    fn resolve_self_type(&self, name: &UnresolvedTypeName) -> Result<TermTy> {
        let msg = if !name.args.is_empty() {
            "`Self' does not take type arguments"
        } else if let Some(t) = self.self_ty.borrow().as_ref() {
            return Ok(t.clone());
        } else {
            "`Self' can only be used in the instance methods of a module"
        };
        Err(located_error(msg.to_string(), &name.locs))
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
        extendable: bool,
        namespace: &Namespace,
        typename: &TypeFullname,
        typarams: &[ty::TyParam],
        superclass: &Option<Supertype>,
        rust_methods: &mut RustMethods,
    ) -> Result<Vec<MethodSignature>> {
        let v = rust_methods.remove(typename).unwrap_or_default();
        v.into_iter()
            .map(|(sig, is_async, when_conds)| {
                let hir_sig = self.create_maybe_virtual_signature(
                    extendable,
                    namespace,
                    typename.clone(),
                    &sig,
                    typarams,
                    superclass,
                    true,
                )?;
                let mut hir_sig = apply_when_conds(hir_sig, typarams, when_conds);
                // A method of an extendable type (an inheritable class or a
                // module) is virtual because a subclass/includer may override
                // it, so it must be async. See also: `check_virtual_asyncness`
                if !is_async && extendable {
                    return Err(error::program_error(&format!(
                        "method {} must be async because it belongs to an extendable type",
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

/// Attach the conditions of the `when` block to `sig`. For `T == X`, `T` in
/// the signature is replaced with `X`.
fn apply_when_conds(
    mut sig: MethodSignature,
    typarams: &[ty::TyParam],
    when_conds: Vec<WhenCond>,
) -> MethodSignature {
    if when_conds.iter().any(|c| c.kind == WhenCondKind::Eq) {
        let tyargs = when_body_tyargs(typarams, &when_conds, true);
        sig = sig.substitute_with_self(&tyargs);
    }
    sig.when_conds = when_conds;
    sig
}

/// Returns the type arguments to substitute the class type parameters in a
/// `when` block. `T == X` replaces `T` with `X`. `T: X` replaces `T` with
/// `T` whose upper bound is `X` (unless `eq_only`).
pub fn when_body_tyargs(
    typarams: &[ty::TyParam],
    when_conds: &[WhenCond],
    eq_only: bool,
) -> Vec<TermTy> {
    typarams
        .iter()
        .enumerate()
        .map(|(i, tp)| {
            let orig = ty::typaram_ref(tp, TyParamKind::Class, i);
            let cond = when_conds.iter().find(|c| match &c.typaram.body {
                TyBody::TyPara(r) => r.kind == TyParamKind::Class && r.idx == i,
                _ => false,
            });
            match cond {
                Some(c) if c.kind == WhenCondKind::Eq => c.ty.clone(),
                Some(c) if !eq_only => match &c.ty.body {
                    TyBody::TyRaw(lit_ty) => {
                        let mut r = orig;
                        r.upper_bound = lit_ty.clone();
                        r.into_term_ty()
                    }
                    TyBody::TyPara(_) => orig.into_term_ty(),
                },
                _ => orig.into_term_ty(),
            }
        })
        .collect()
}

/// Check the contents of a `when` block
fn check_when_block(
    conds: &[shiika_ast::AstWhenCond],
    defs: &[shiika_ast::Definition],
) -> Result<()> {
    for def in defs {
        match def {
            shiika_ast::Definition::InstanceMethodDefinition { .. }
            | shiika_ast::Definition::NativeMethodDefinition {
                is_class_method: false,
                ..
            }
            | shiika_ast::Definition::ImplementDefinition { .. } => {}
            _ => {
                return Err(located_error(
                    "only instance methods and `implement' can be defined in `when' \
                     (`initialize', class methods, etc. are not allowed)"
                        .to_string(),
                    &conds[0].locs,
                ));
            }
        }
    }
    Ok(())
}

/// A method in a `when` block cannot have the same name as another method
/// (methods are looked up by name; overloading by conditions is not supported)
fn check_when_method_names(
    owner: &TypeFullname,
    items: &[(&shiika_ast::Definition, Vec<WhenCond>)],
) -> Result<()> {
    let name_of = |def: &shiika_ast::Definition| match def {
        shiika_ast::Definition::InstanceMethodDefinition { sig, .. }
        | shiika_ast::Definition::NativeMethodDefinition {
            sig,
            is_class_method: false,
            ..
        } => Some(sig.name.clone()),
        _ => None,
    };
    for (i, (def, conds)) in items.iter().enumerate() {
        if conds.is_empty() {
            continue;
        }
        let Some(name) = name_of(def) else { continue };
        for (j, (other, _)) in items.iter().enumerate() {
            if i != j && name_of(other).as_ref() == Some(&name) {
                return Err(error::program_error(&format!(
                    "method `{}' of `{}' is defined in a `when' block and elsewhere \
                     (overloading by the conditions is not supported)",
                    name, owner
                )));
            }
        }
    }
    Ok(())
}

/// Make an `ImplementBlock` from `implement module ... end` and check its
/// contents
fn implement_block<'a>(
    module: &'a UnresolvedTypeName,
    defs: &[shiika_ast::Definition],
    when_conds: Vec<WhenCond>,
) -> Result<ImplementBlock<'a>> {
    let mut method_names = vec![];
    for inner in defs {
        match inner {
            shiika_ast::Definition::InstanceMethodDefinition { sig, .. }
            | shiika_ast::Definition::NativeMethodDefinition {
                sig,
                is_class_method: false,
                ..
            } => method_names.push(sig.name.clone()),
            _ => {
                return Err(located_error(
                    format!(
                        "only instance methods can be defined in `implement {}'",
                        module
                    ),
                    &module.locs,
                ));
            }
        }
    }
    Ok(ImplementBlock {
        module,
        method_names,
        when_conds,
    })
}

/// Returns the signature of a `native def` in `defs` (recursively), if any
fn find_native_def<'a>(
    defs: impl Iterator<Item = &'a shiika_ast::Definition>,
) -> Option<&'a AstMethodSignature> {
    for def in defs {
        let found = match def {
            shiika_ast::Definition::NativeMethodDefinition { sig, .. } => Some(sig),
            shiika_ast::Definition::ClassDefinition { defs, .. }
            | shiika_ast::Definition::ModuleDefinition { defs, .. }
            | shiika_ast::Definition::EnumDefinition { defs, .. } => find_native_def(defs.iter()),
            _ => None,
        };
        if found.is_some() {
            return found;
        }
    }
    None
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
                ty::typaram_ref(t, TyParamKind::Class, i).into_term_ty()
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
        when_conds: vec![],
    });
    MethodSignatures::from_iterator(iter)
}

/// `Self` is reserved for the type of a module's includer
fn check_reserved_type_name(name: &str) -> Result<()> {
    if name == "Self" {
        return Err(error::program_error(
            "`Self' is reserved and cannot be used as a class, module or enum name",
        ));
    }
    Ok(())
}

/// Create a program error with a label on `locs`
fn located_error(msg: String, locs: &LocationSpan) -> anyhow::Error {
    let report = skc_error::build_report(msg.clone(), locs, |r, span| {
        r.with_label(Label::new(span).with_message(msg))
    });
    error::program_error(report)
}
