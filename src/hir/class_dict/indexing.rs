use crate::ast;
use crate::error;
use crate::error::*;
use crate::hir::class_dict::class_dict::*;
use crate::hir::*;
use crate::names::*;
use std::collections::HashMap;

type MethodSignatures = HashMap<MethodFirstname, MethodSignature>;

impl<'hir_maker> ClassDict<'hir_maker> {
    /// Register a class
    pub fn add_class(&mut self, class: SkClass) {
        self.sk_classes.insert(class.fullname.clone(), class);
    }

    /// Add a method
    /// Used to add auto-defined accessors
    pub fn add_method(&mut self, clsname: &ClassFullname, sig: MethodSignature) {
        let sk_class = self.sk_classes.get_mut(&clsname).unwrap();
        sk_class
            .method_sigs
            .insert(sig.fullname.first_name.clone(), sig);
    }

    pub fn index_program(&mut self, toplevel_defs: &[&ast::Definition]) -> Result<(), Error> {
        let namespace = Namespace::root();
        for def in toplevel_defs {
            match def {
                ast::Definition::ClassDefinition {
                    name,
                    typarams,
                    superclass,
                    defs,
                } => self.index_class(&namespace, &name, &typarams, &superclass, &defs)?,
                ast::Definition::EnumDefinition {
                    name,
                    typarams,
                    cases,
                } => self.index_enum(&namespace, &name, &typarams, &cases)?,
                ast::Definition::ConstDefinition { .. } => (),
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
        typarams: &[String],
        ast_superclass: &Option<ConstName>,
        defs: &[ast::Definition],
    ) -> Result<(), Error> {
        let fullname = namespace.class_fullname(firstname);
        let metaclass_fullname = fullname.meta_name();
        let superclass = if let Some(name) = ast_superclass {
            Superclass::from_ty(self._resolve_typename(
                namespace,
                typarams,
                Default::default(),
                name,
            )?)
        } else {
            Superclass::default()
        };
        let new_sig = signature::signature_of_new(
            &metaclass_fullname,
            self._initializer_params(namespace, typarams, &superclass, &defs)?,
            &ty::return_type_of_new(&fullname, typarams),
        );

        let inner_namespace = namespace.add(firstname);
        let (instance_methods, class_methods) =
            self.index_defs_in_class(&inner_namespace, &fullname, typarams, defs)?;

        match self.sk_classes.get_mut(&fullname) {
            Some(class) => {
                // Merge methods to existing class
                // Shiika will not support reopening a class but this is needed
                // for classes defined both in src corelib/ and in builtin/.
                class.method_sigs.extend(instance_methods);
                let metaclass = self
                    .sk_classes
                    .get_mut(&metaclass_fullname)
                    .expect("[BUG] Only class is indexed");
                metaclass.method_sigs.extend(class_methods);
                // Add `.new` to the metaclass
                if !metaclass.method_sigs.contains_key(&method_firstname("new")) {
                    metaclass
                        .method_sigs
                        .insert(new_sig.fullname.first_name.clone(), new_sig);
                }
            }
            None => self.add_new_class(
                &fullname,
                typarams,
                superclass,
                Some(new_sig),
                instance_methods,
                class_methods,
                false,
            )?,
        }
        Ok(())
    }

    /// Return parameters of `initialize` which is defined by
    /// - `#initialize` in `defs` (if any) or,
    /// - `#initialize` inherited from ancestors.
    fn _initializer_params(
        &self,
        namespace: &Namespace,
        typarams: &[String],
        superclass: &Superclass,
        defs: &[ast::Definition],
    ) -> Result<Vec<MethodParam>, Error> {
        if let Some(ast::Definition::InstanceMethodDefinition { sig, .. }) =
            defs.iter().find(|d| d.is_initializer())
        {
            // Has explicit initializer definition
            self.convert_params(namespace, &sig.params, typarams, Default::default())
        } else {
            // Inherit #initialize from superclass
            let (sig, _) = self
                .lookup_method(&superclass.ty(), &method_firstname("initialize"), &[])
                .expect("[BUG] initialize not found");
            Ok(specialized_initialize(&sig, superclass).params)
        }
    }

    fn index_enum(
        &mut self,
        namespace: &Namespace,
        firstname: &ClassFirstname,
        typarams: &[String],
        cases: &[ast::EnumCase],
    ) -> Result<(), Error> {
        let fullname = namespace.class_fullname(firstname);
        let instance_methods = Default::default();
        self.add_new_class(
            &fullname,
            typarams,
            Superclass::simple("Object"),
            None,
            instance_methods,
            Default::default(),
            false,
        )?;
        for case in cases {
            self.index_enum_case(namespace, &fullname, typarams, case)?;
        }
        Ok(())
    }

    fn index_enum_case(
        &mut self,
        namespace: &Namespace,
        enum_fullname: &ClassFullname,
        typarams: &[String],
        case: &ast::EnumCase,
    ) -> Result<(), Error> {
        let ivar_list = self._enum_case_ivars(namespace, typarams, case)?;
        let fullname = case.name.add_namespace(&enum_fullname.0);
        let superclass = enum_case_superclass(enum_fullname, typarams, case);
        let (new_sig, initialize_sig) = enum_case_new_sig(&ivar_list, typarams, &fullname);

        let mut instance_methods = enum_case_getters(&fullname, &ivar_list);
        instance_methods.insert(method_firstname("initialize"), initialize_sig);

        self.add_new_class(
            &fullname,
            &typarams,
            superclass,
            Some(new_sig),
            instance_methods,
            Default::default(),
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
        typarams: &[String],
        case: &ast::EnumCase,
    ) -> Result<Vec<SkIVar>, Error> {
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
        fullname: &ClassFullname,
        typarams: &[String],
        defs: &[ast::Definition],
    ) -> Result<(MethodSignatures, MethodSignatures), Error> {
        let mut instance_methods = HashMap::new();
        let mut class_methods = HashMap::new();
        for def in defs {
            match def {
                ast::Definition::InstanceMethodDefinition { sig, .. } => {
                    let hir_sig = self._create_signature(namespace, &fullname, sig, typarams)?;
                    instance_methods.insert(sig.name.clone(), hir_sig);
                }
                ast::Definition::ClassMethodDefinition { sig, .. } => {
                    let hir_sig = self._create_signature(
                        namespace,
                        &fullname.meta_name(),
                        sig,
                        Default::default(),
                    )?;
                    class_methods.insert(sig.name.clone(), hir_sig);
                }
                ast::Definition::ConstDefinition { .. } => (),
                ast::Definition::ClassDefinition {
                    name,
                    typarams,
                    superclass,
                    defs,
                } => {
                    self.index_class(namespace, &name, &typarams, &superclass, &defs)?;
                }
                ast::Definition::EnumDefinition {
                    name,
                    typarams,
                    cases,
                } => {
                    self.index_enum(namespace, &name, &typarams, &cases)?;
                }
            }
        }
        Ok((instance_methods, class_methods))
    }

    /// Register a class and its metaclass to self
    fn add_new_class(
        &mut self,
        fullname: &ClassFullname,
        typaram_names: &[String],
        superclass: Superclass,
        new_sig: Option<MethodSignature>,
        instance_methods: HashMap<MethodFirstname, MethodSignature>,
        mut class_methods: HashMap<MethodFirstname, MethodSignature>,
        const_is_obj: bool,
    ) -> Result<(), Error> {
        let typarams = typaram_names
            .iter()
            .map(|s| TyParam {
                name: s.to_string(),
            })
            .collect::<Vec<_>>();

        // Add `.new` to the metaclass
        if let Some(sig) = new_sig {
            class_methods.insert(sig.fullname.first_name.clone(), sig);
        }

        self.add_class(SkClass {
            fullname: fullname.clone(),
            typarams: typarams.clone(),
            superclass: Some(superclass),
            instance_ty: ty::raw(&fullname.0),
            ivars: HashMap::new(), // will be set when processing `#initialize`
            method_sigs: instance_methods,
            const_is_obj,
            foreign: false,
        });

        // Crete metaclass (which is a subclass of `Class`)
        let the_class = self.get_class(&class_fullname("Class"));
        let meta_ivars = the_class.ivars.clone();
        self.add_class(SkClass {
            fullname: fullname.meta_name(),
            typarams,
            superclass: Some(Superclass::simple("Class")),
            instance_ty: ty::meta(&fullname.0),
            ivars: meta_ivars,
            method_sigs: class_methods,
            const_is_obj: false,
            foreign: false,
        });
        Ok(())
    }

    fn _create_signature(
        &self,
        namespace: &Namespace,
        class_fullname: &ClassFullname,
        sig: &ast::AstMethodSignature,
        class_typarams: &[String],
    ) -> Result<MethodSignature, Error> {
        let fullname = method_fullname(class_fullname, &sig.name.0);
        let ret_ty = if let Some(typ) = &sig.ret_typ {
            self._resolve_typename(namespace, class_typarams, &sig.typarams, typ)?
        } else {
            ty::raw("Void") // Default return type.
        };
        Ok(MethodSignature {
            fullname,
            ret_ty,
            params: self.convert_params(namespace, &sig.params, class_typarams, &sig.typarams)?,
            typarams: sig.typarams.clone(),
        })
    }

    /// Convert ast params to hir params
    pub fn convert_params(
        &self,
        namespace: &Namespace,
        ast_params: &[ast::Param],
        class_typarams: &[String],
        method_typarams: &[String],
    ) -> Result<Vec<MethodParam>, Error> {
        let mut hir_params = vec![];
        for param in ast_params {
            hir_params.push(MethodParam {
                name: param.name.to_string(),
                ty: self._resolve_typename(
                    namespace,
                    class_typarams,
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
        class_typarams: &[String],
        method_typarams: &[String],
        name: &ConstName,
    ) -> Result<TermTy, Error> {
        // Check it is a typaram
        if name.args.is_empty() && name.names.len() == 1 {
            let s = name.names.first().unwrap();
            if let Some(idx) = class_typarams.iter().position(|t| s == t) {
                return Ok(ty::typaram(s, TyParamKind::Class, idx));
            } else if let Some(idx) = method_typarams.iter().position(|t| s == t) {
                return Ok(ty::typaram(s, TyParamKind::Method, idx));
            }
        }
        // Otherwise:
        let mut tyargs = vec![];
        for arg in &name.args {
            tyargs.push(self._resolve_typename(namespace, class_typarams, method_typarams, arg)?);
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
    ) -> Result<(Vec<String>, &[TyParam]), Error> {
        let n = namespace.len();
        for k in 0..=n {
            let mut resolved = namespace.head(n - k).to_vec();
            resolved.append(&mut names.to_vec());
            if let Some(typarams) = self.class_index.get(&class_fullname(resolved.join("::"))) {
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
    enum_fullname: &ClassFullname,
    typarams: &[String],
    case: &ast::EnumCase,
) -> Superclass {
    if case.params.is_empty() {
        let tyargs = typarams
            .iter()
            .map(|_| ty::raw("Never"))
            .collect::<Vec<_>>();
        Superclass::new(enum_fullname, tyargs)
    } else {
        let tyargs = typarams
            .iter()
            .enumerate()
            .map(|(i, s)| ty::typaram(s, TyParamKind::Class, i))
            .collect::<Vec<_>>();
        Superclass::new(enum_fullname, tyargs)
    }
}

/// Returns signature of `.new` and `#initialize` of an enum case
fn enum_case_new_sig(
    ivar_list: &[SkIVar],
    typarams: &[String],
    fullname: &ClassFullname,
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
            .map(|(i, s)| ty::typaram(s, TyParamKind::Class, i))
            .collect::<Vec<_>>();
        ty::spe(&fullname.0, tyargs)
    };
    (
        signature::signature_of_new(&fullname.meta_name(), params.clone(), &ret_ty),
        signature::signature_of_initialize(&fullname, params),
    )
}

/// Create signatures of getters of an enum case
fn enum_case_getters(case_fullname: &ClassFullname, ivars: &[SkIVar]) -> MethodSignatures {
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
