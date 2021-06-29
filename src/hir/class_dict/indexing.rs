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
        for def in toplevel_defs {
            match def {
                ast::Definition::ClassDefinition {
                    name,
                    typarams,
                    superclass,
                    defs,
                } => self.index_class(&name.add_namespace(""), &typarams, &superclass, &defs)?,
                ast::Definition::EnumDefinition {
                    name,
                    typarams,
                    cases,
                } => self.index_enum(&name.add_namespace(""), &typarams, &cases)?,
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
        fullname: &ClassFullname,
        typarams: &[String],
        ast_superclass: &Option<ConstName>,
        defs: &[ast::Definition],
    ) -> Result<(), Error> {
        let metaclass_fullname = fullname.meta_name();
        // TODO: check ast_superclass is valid
        let superclass = if let Some(n) = ast_superclass {
            Superclass::from_const_name(n, typarams)
        } else {
            Superclass::default()
        };
        let new_sig = signature::signature_of_new(
            &metaclass_fullname,
            self.initializer_params(typarams, &superclass, &defs),
            &ty::return_type_of_new(fullname, typarams),
        );

        let (instance_methods, class_methods) =
            self.index_defs_in_class(fullname, typarams, defs)?;

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
                fullname,
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

    fn index_enum(
        &mut self,
        fullname: &ClassFullname,
        typarams: &[String],
        cases: &[ast::EnumCase],
    ) -> Result<(), Error> {
        let instance_methods = Default::default();
        self.add_new_class(
            fullname,
            typarams,
            Superclass::simple("Object"),
            None,
            instance_methods,
            Default::default(),
            false,
        )?;
        for case in cases {
            self.index_enum_case(fullname, typarams, case)?;
        }
        Ok(())
    }

    fn index_enum_case(
        &mut self,
        enum_fullname: &ClassFullname,
        enum_typarams: &[String],
        case: &ast::EnumCase,
    ) -> Result<(), Error> {
        validate_enum_case_typarams(enum_fullname, enum_typarams, case)?;
        let ivar_list = enum_case_ivars(case)?;
        let fullname = case.name.add_namespace(&enum_fullname.0);
        let superclass = enum_case_superclass(enum_fullname, enum_typarams, case);
        let new_sig = enum_case_new_sig(&fullname, case);

        let instance_methods = enum_case_getters(&fullname, &ivar_list);
        self.add_new_class(
            &fullname,
            &case.typarams,
            superclass,
            Some(new_sig),
            instance_methods,
            Default::default(),
            enum_typarams.is_empty(),
        )?;
        let ivars = ivar_list.into_iter().map(|x| (x.name.clone(), x)).collect();
        self.define_ivars(&fullname, ivars);
        Ok(())
    }

    fn index_defs_in_class(
        &mut self,
        fullname: &ClassFullname,
        typarams: &[String],
        defs: &[ast::Definition],
    ) -> Result<(MethodSignatures, MethodSignatures), Error> {
        let mut instance_methods = HashMap::new();
        let mut class_methods = HashMap::new();
        for def in defs {
            match def {
                ast::Definition::InstanceMethodDefinition { sig, .. } => {
                    let hir_sig = signature::create_signature(&fullname, sig, typarams);
                    instance_methods.insert(sig.name.clone(), hir_sig);
                }
                ast::Definition::ClassMethodDefinition { sig, .. } => {
                    let hir_sig = signature::create_signature(&fullname.meta_name(), sig, &[]);
                    class_methods.insert(sig.name.clone(), hir_sig);
                }
                ast::Definition::ConstDefinition { .. } => (),
                ast::Definition::ClassDefinition {
                    name,
                    typarams,
                    superclass,
                    defs,
                } => {
                    let full = name.add_namespace(&fullname.0);
                    self.index_class(&full, &typarams, &superclass, &defs)?;
                }
                ast::Definition::EnumDefinition {
                    name,
                    typarams,
                    cases,
                } => {
                    let full = name.add_namespace(&fullname.0);
                    self.index_enum(&full, &typarams, &cases)?;
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

        if !self.is_valid_superclass(superclass.ty(), typaram_names) {
            return Err(error::name_error(&format!(
                "superclass {:?} of {:?} does not exist",
                superclass, fullname,
            )));
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
}

/// Check if `case_typarams` is subset of `enum_typarams`
fn validate_enum_case_typarams(
    enum_fullname: &ClassFullname,
    enum_typarams: &[String],
    case: &ast::EnumCase,
) -> Result<(), Error> {
    for c in &case.typarams {
        if !enum_typarams.contains(&c) {
            return Err(name_error(&format!(
                "`{}::{}' has undefined type parameter `{}'",
                enum_fullname, &case.name, c
            )));
        }
    }
    Ok(())
}

/// List up ivars of an enum case
fn enum_case_ivars(case: &ast::EnumCase) -> Result<Vec<SkIVar>, Error> {
    let mut ivars = vec![];
    for (idx, param) in case.params.iter().enumerate() {
        let ty = signature::convert_typ(&param.typ, &case.typarams, &[]);
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

/// Returns superclass of a enum case
fn enum_case_superclass(
    enum_fullname: &ClassFullname,
    enum_typarams: &[String],
    case: &ast::EnumCase,
) -> Superclass {
    if case.params.is_empty() {
        let tyargs = enum_typarams
            .iter()
            .map(|_| ty::raw("Never"))
            .collect::<Vec<_>>();
        Superclass::new(enum_fullname, tyargs)
    } else {
        let tyargs = enum_typarams
            .iter()
            .enumerate()
            .map(|(i, s)| ty::typaram(s, TyParamKind::Class, i))
            .collect::<Vec<_>>();
        Superclass::new(enum_fullname, tyargs)
    }
}

/// Returns signature of `.new` of an enum case
fn enum_case_new_sig(fullname: &ClassFullname, case: &ast::EnumCase) -> MethodSignature {
    let params = signature::convert_params(&case.params, &case.typarams, &[]);
    let ret_ty = if case.typarams.is_empty() {
        ty::raw(&fullname.0)
    } else {
        let tyargs = case
            .typarams
            .iter()
            .enumerate()
            .map(|(i, s)| ty::typaram(s, TyParamKind::Class, i))
            .collect::<Vec<_>>();
        ty::spe(&fullname.0, tyargs)
    };
    signature::signature_of_new(&fullname.meta_name(), params, &ret_ty)
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
