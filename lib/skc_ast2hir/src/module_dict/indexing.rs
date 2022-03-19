use crate::module_dict::*;
use crate::error;
use crate::parse_typarams;
use anyhow::Result;
use shiika_ast;
use shiika_core::{names::*, ty, ty::*};
use skc_hir::signature::*;
use skc_hir::*;
use std::collections::HashMap;

type MethodSignatures = HashMap<MethodFirstname, MethodSignature>;

impl<'hir_maker> ModuleDict<'hir_maker> {
    /// Register a class
    pub fn add_class(&mut self, class: SkModule) {
        self.sk_modules.insert(class.fullname(), class);
    }

    /// Add a method
    /// Used to add auto-defined accessors
    pub fn add_method(&mut self, clsname: &ModuleFullname, sig: MethodSignature) {
        let sk_class = self.sk_modules.get_mut(clsname).unwrap();
        sk_class
            .method_sigs
            .insert(sig.fullname.first_name.clone(), sig);
    }

    pub fn index_program(&mut self, toplevel_defs: &[&shiika_ast::Definition]) -> Result<()> {
        let namespace = Namespace::root();
        for def in toplevel_defs {
            match def {
                shiika_ast::Definition::ClassDefinition {
                    name,
                    typarams,
                    superclass,
                    defs,
                } => {
                    self.index_module(&namespace, name, parse_typarams(typarams), Some(superclass), defs)?
                }
                shiika_ast::Definition::ModuleDefinition {
                    name,
                    typarams,
                    defs,
                } => {
                    self.index_module(&namespace, name, parse_typarams(typarams), None, defs)?
                }
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

    // Index a module or a class
    fn index_module(
        &mut self,
        namespace: &Namespace,
        firstname: &ModuleFirstname,
        typarams: Vec<ty::TyParam>,
        ast_superclass: Option<&Option<ConstName>>,
        defs: &[shiika_ast::Definition],
    ) -> Result<()> {
        let is_class = ast_superclass.is_some();
        let fullname = namespace.module_fullname(firstname);
        let metaclass_fullname = fullname.meta_name();
        let (new_sig, class_info) = if is_class {
            let superclass = if let Some(name) = ast_superclass.unwrap() {
                let ty = self._resolve_typename(namespace, &typarams, Default::default(), name)?;

                if self.get_class(&ty.erasure()).is_final.unwrap() {
                    return Err(error::program_error(&format!("cannot inherit from {}", ty)));
                }
                Superclass::from_ty(ty)
            } else {
                Superclass::default()
            };
            let new_sig = if fullname.0 == "Never" {
                None
            } else {
                Some(signature::signature_of_new(
                    &metaclass_fullname,
                    self._initializer_params(namespace, &typarams, &superclass, defs)?,
                    &ty::return_type_of_new(&fullname, &typarams),
                ))
            };
            let class_info = ClassInfo {
                superclass: Some(superclass),
                ivars: Default::default(),
                is_final: Some(false),
                const_is_obj: false,
            };
            (new_sig, Some(class_info))
        } else {
            Default::default()
        };

        let inner_namespace = namespace.add(firstname);
        let (instance_methods, class_methods) =
            self.index_defs_in_class(&inner_namespace, &fullname, &typarams, defs)?;

        match self.sk_modules.get_mut(&fullname) {
            Some(class) => {
                // Merge methods to existing class
                // Shiika will not support reopening a class but this is needed
                // for classes defined both in src corelib/ and in builtin/.
                class.method_sigs.extend(instance_methods);
                let metaclass = self
                    .sk_modules
                    .get_mut(&metaclass_fullname)
                    .unwrap_or_else(|| {
                        panic!(
                            "[BUG] metaclass not found: {} <- {}",
                            fullname, &metaclass_fullname
                        )
                    });
                metaclass.method_sigs.extend(class_methods);
                // Add `.new` to the metaclass
                if let Some(sig) = new_sig {
                    if !metaclass.method_sigs.contains_key(&method_firstname("new")) {
                        metaclass
                            .method_sigs
                            .insert(sig.fullname.first_name.clone(), sig);
                    }
                }
            }
            None => self.add_new_module(
                &fullname,
                &typarams,
                class_info,
                new_sig,
                instance_methods,
                class_methods,
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
        superclass: &Superclass,
        defs: &[shiika_ast::Definition],
    ) -> Result<Vec<MethodParam>> {
        if let Some(shiika_ast::Definition::InstanceMethodDefinition { sig, .. }) =
            defs.iter().find(|d| d.is_initializer())
        {
            // Has explicit initializer definition
            self.convert_params(namespace, &sig.params, typarams, Default::default())
        } else {
            // Inherit #initialize from superclass
            let (sig, _) = self
                .lookup_method(superclass.ty(), &method_firstname("initialize"), &[])
                .expect("[BUG] initialize not found");
            Ok(specialized_initialize(&sig, superclass).params)
        }
    }

    fn index_enum(
        &mut self,
        namespace: &Namespace,
        firstname: &ModuleFirstname,
        typarams: Vec<TyParam>,
        cases: &[shiika_ast::EnumCase],
        defs: &[shiika_ast::Definition],
    ) -> Result<()> {
        let fullname = namespace.module_fullname(firstname);
        let inner_namespace = namespace.add(firstname);
        let (instance_methods, class_methods) =
            self.index_defs_in_class(&inner_namespace, &fullname, &typarams, defs)?;
        let class_info = ClassInfo {
            superclass: Some(Superclass::simple("Object")),
            ivars: Default::default(),
            is_final: Some(true),
            const_is_obj: false,
        };
        self.add_new_module(
            &fullname,
            &typarams,
            Some(class_info),
            None,
            instance_methods,
            class_methods,
        );
        for case in cases {
            self.index_enum_case(namespace, &fullname, &typarams, case)?;
        }

        Ok(())
    }

    fn index_enum_case(
        &mut self,
        namespace: &Namespace,
        enum_fullname: &ModuleFullname,
        typarams: &[ty::TyParam],
        case: &shiika_ast::EnumCase,
    ) -> Result<()> {
        let ivar_list = self._enum_case_ivars(namespace, typarams, case)?;
        let fullname = case.name.add_namespace(&enum_fullname.0);
        let superclass = enum_case_superclass(enum_fullname, typarams, case);
        let (new_sig, initialize_sig) = enum_case_new_sig(&ivar_list, typarams, &fullname);

        let mut instance_methods = enum_case_getters(&fullname, &ivar_list);
        instance_methods.insert(method_firstname("initialize"), initialize_sig);

        let case_typarams = if case.params.is_empty() {
            Default::default()
        } else {
            typarams
        };
        let class_info = ClassInfo {
            superclass: Some(superclass),
            ivars: Default::default(),
            is_final: Some(true),
            const_is_obj: case.params.is_empty(),
        };
        self.add_new_module(
            &fullname,
            case_typarams,
            Some(class_info),
            Some(new_sig),
            instance_methods,
            Default::default(),
        );
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
            let ty = self._resolve_typename(namespace, typarams, Default::default(), &param.typ)?;
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
        fullname: &ModuleFullname,
        typarams: &[ty::TyParam],
        defs: &[shiika_ast::Definition],
    ) -> Result<(MethodSignatures, MethodSignatures)> {
        let mut instance_methods = HashMap::new();
        let mut class_methods = HashMap::new();
        for def in defs {
            match def {
                shiika_ast::Definition::InstanceMethodDefinition { sig, .. } => {
                    let hir_sig = self.create_signature(namespace, fullname, sig, typarams)?;
                    instance_methods.insert(sig.name.clone(), hir_sig);
                }
                shiika_ast::Definition::ClassMethodDefinition { sig, .. } => {
                    let hir_sig = self.create_signature(
                        namespace,
                        &fullname.meta_name(),
                        sig,
                        Default::default(),
                    )?;
                    class_methods.insert(sig.name.clone(), hir_sig);
                }
                shiika_ast::Definition::ConstDefinition { .. } => (),
                shiika_ast::Definition::ClassDefinition {
                    name,
                    typarams,
                    superclass,
                    defs,
                } => {
                    self.index_module(namespace, name, parse_typarams(typarams), Some(superclass), defs)?;
                }
                shiika_ast::Definition::ModuleDefinition {
                    name,
                    typarams,
                    defs,
                } => {
                    self.index_module(namespace, name, parse_typarams(typarams), None, defs)?;
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
        Ok((instance_methods, class_methods))
    }

    /// Register a class/module and its metaclass to self
    fn add_new_module(
        &mut self,
        fullname: &ModuleFullname,
        typarams: &[ty::TyParam],
        class_info: Option<ClassInfo>,
        new_sig: Option<MethodSignature>,
        instance_methods: HashMap<MethodFirstname, MethodSignature>,
        mut class_methods: HashMap<MethodFirstname, MethodSignature>,
    ) {
        // Add `.new` to the metaclass
        if let Some(sig) = new_sig {
            class_methods.insert(sig.fullname.first_name.clone(), sig);
        }

        self.add_class(SkModule {
            erasure: Erasure::nonmeta(&fullname.0),
            typarams: typarams.to_vec(),
            method_sigs: instance_methods,
            foreign: false,
            class_info,
        });

        // Create metaclass (which is a subclass of `Class`)
        let the_class = self.get_class(&module_fullname("Class"));
        let meta_ivars = the_class.ivars.clone();
        let metaclass_info = ClassInfo {
            superclass: Some(Superclass::simple("Class")),
            ivars: meta_ivars,
            is_final: None,
            const_is_obj: false,
        };
        self.add_class(SkModule {
            erasure: Erasure::meta(&fullname.0),
            typarams: typarams.to_vec(),
            method_sigs: class_methods,
            foreign: false,
            class_info: Some(metaclass_info),
        });
    }

    /// Convert AstMethodSignature to MethodSignature
    pub fn create_signature(
        &self,
        namespace: &Namespace,
        module_fullname: &ModuleFullname,
        sig: &shiika_ast::AstMethodSignature,
        module_typarams: &[ty::TyParam],
    ) -> Result<MethodSignature> {
        let method_typarams = parse_typarams(&sig.typarams);
        let fullname = method_fullname(module_fullname, &sig.name.0);
        let ret_ty = if let Some(typ) = &sig.ret_typ {
            self._resolve_typename(namespace, module_typarams, &method_typarams, typ)?
        } else {
            ty::raw("Void") // Default return type.
        };
        Ok(MethodSignature {
            fullname,
            ret_ty,
            params: self.convert_params(
                namespace,
                &sig.params,
                module_typarams,
                &method_typarams,
            )?,
            typarams: method_typarams,
        })
    }

    /// Convert ast params to hir params
    pub fn convert_params(
        &self,
        namespace: &Namespace,
        ast_params: &[shiika_ast::Param],
        module_typarams: &[ty::TyParam],
        method_typarams: &[ty::TyParam],
    ) -> Result<Vec<MethodParam>> {
        let mut hir_params = vec![];
        for param in ast_params {
            hir_params.push(MethodParam {
                name: param.name.to_string(),
                ty: self._resolve_typename(
                    namespace,
                    module_typarams,
                    method_typarams,
                    &param.typ,
                )?,
            });
        }
        Ok(hir_params)
    }

    /// Resolve the given type name to fullname
    fn _resolve_typename(
        &self,
        namespace: &Namespace,
        module_typarams: &[ty::TyParam],
        method_typarams: &[ty::TyParam],
        name: &ConstName,
    ) -> Result<TermTy> {
        // Check it is a typaram
        if name.args.is_empty() && name.names.len() == 1 {
            let s = name.names.first().unwrap();
            if let Some(idx) = module_typarams.iter().position(|t| *s == t.name) {
                return Ok(ty::typaram_ref(s, TyParamKind::Class, idx).into_term_ty());
            } else if let Some(idx) = method_typarams.iter().position(|t| *s == t.name) {
                return Ok(ty::typaram_ref(s, TyParamKind::Method, idx).into_term_ty());
            }
        }
        // Otherwise:
        let mut tyargs = vec![];
        for arg in &name.args {
            tyargs.push(self._resolve_typename(namespace, module_typarams, method_typarams, arg)?);
        }
        let (resolved_base, base_typarams) =
            self._resolve_simple_typename(namespace, &name.names)?;
        if name.args.len() != base_typarams.len() {
            return Err(error::type_error(&format!(
                "wrong number of type arguments: {:?}",
                name
            )));
        }
        Ok(ty::nonmeta(&resolved_base, tyargs))
    }

    /// Resolve the given type name (without type arguments) to fullname
    /// Also returns the typarams of the class, if any
    fn _resolve_simple_typename(
        &self,
        namespace: &Namespace,
        names: &[String],
    ) -> Result<(Vec<String>, &[TyParam])> {
        let n = namespace.size();
        for k in 0..=n {
            let mut resolved = namespace.head(n - k).to_vec();
            resolved.append(&mut names.to_vec());
            if let Some(typarams) = self.module_index.get(&module_fullname(resolved.join("::"))) {
                return Ok((resolved, typarams));
            }
        }
        Err(error::name_error(&format!(
            "unknown type {:?} in {:?}",
            names, namespace,
        )))
    }
}

/// Returns superclass of a enum case
fn enum_case_superclass(
    enum_fullname: &ModuleFullname,
    typarams: &[ty::TyParam],
    case: &shiika_ast::EnumCase,
) -> Superclass {
    if case.params.is_empty() {
        // eg. Maybe::None : Maybe<Never>
        let tyargs = typarams
            .iter()
            .map(|_| ty::raw("Never"))
            .collect::<Vec<_>>();
        Superclass::new(enum_fullname, tyargs)
    } else {
        // eg. Maybe::Some<out V> : Maybe<V>
        let tyargs = typarams
            .iter()
            .enumerate()
            .map(|(i, t)| ty::typaram_ref(&t.name, TyParamKind::Class, i).into_term_ty())
            .collect::<Vec<_>>();
        Superclass::new(enum_fullname, tyargs)
    }
}

/// Returns signature of `.new` and `#initialize` of an enum case
fn enum_case_new_sig(
    ivar_list: &[SkIVar],
    typarams: &[ty::TyParam],
    fullname: &ModuleFullname,
) -> (MethodSignature, MethodSignature) {
    let params = ivar_list
        .iter()
        .map(|ivar| MethodParam {
            name: ivar.name.to_string(),
            ty: ivar.ty.clone(),
        })
        .collect::<Vec<_>>();
    let ret_ty = if ivar_list.is_empty() {
        ty::raw(&fullname.0)
    } else {
        let tyargs = typarams
            .iter()
            .enumerate()
            .map(|(i, t)| ty::typaram_ref(&t.name, TyParamKind::Class, i).into_term_ty())
            .collect::<Vec<_>>();
        ty::spe(&fullname.0, tyargs)
    };
    (
        signature::signature_of_new(&fullname.meta_name(), params.clone(), &ret_ty),
        signature::signature_of_initialize(fullname, params),
    )
}

/// Create signatures of getters of an enum case
fn enum_case_getters(case_fullname: &ModuleFullname, ivars: &[SkIVar]) -> MethodSignatures {
    ivars
        .iter()
        .map(|ivar| {
            let sig = MethodSignature {
                fullname: method_fullname(case_fullname, &ivar.accessor_name()),
                ret_ty: ivar.ty.clone(),
                params: Default::default(),
                typarams: Default::default(),
            };
            (method_firstname(&ivar.name), sig)
        })
        .collect()
}
