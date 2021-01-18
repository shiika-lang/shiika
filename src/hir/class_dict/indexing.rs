use crate::ast;
use crate::error;
use crate::error::*;
use crate::hir::class_dict::class_dict::ClassDict;
use crate::hir::*;
use crate::names::*;
use std::collections::HashMap;

impl ClassDict {
    /// Define ivars of a class
    pub fn define_ivars(
        &mut self,
        classname: &ClassFullname,
        own_ivars: HashMap<String, SkIVar>,
    ) -> Result<(), Error> {
        let super_ivars = match self.get_superclass(&classname) {
            Some(super_cls) => super_cls.ivars.clone(),
            None => HashMap::new(),
        };
        let class = self.get_class_mut(&classname, "ClassDict::define_ivars");
        debug_assert!(class.ivars.is_empty());
        class.ivars = super_ivars;
        own_ivars.into_iter().for_each(|(k, v)| {
            class.ivars.insert(k, v);
        });
        Ok(())
    }

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

    pub fn index_corelib(&mut self, corelib: HashMap<ClassFullname, SkClass>) {
        corelib.into_iter().for_each(|(_, c)| {
            self.add_class(SkClass {
                fullname: c.fullname,
                typarams: c.typarams,
                superclass_fullname: c.superclass_fullname,
                instance_ty: c.instance_ty,
                ivars: c.ivars,
                method_sigs: c.method_sigs,
                const_is_obj: c.const_is_obj,
            })
        });
    }

    pub fn index_program(&mut self, toplevel_defs: &[&ast::Definition]) -> Result<(), Error> {
        toplevel_defs.iter().try_for_each(|def| match def {
            ast::Definition::ClassDefinition {
                name,
                typarams,
                super_name,
                defs,
            } => {
                self.index_class(&name.add_namespace(""), &typarams, &super_name, &defs)?;
                Ok(())
            }
            ast::Definition::ConstDefinition { .. } => Ok(()),
            _ => Err(error::syntax_error(&format!(
                "must not be toplevel: {:?}",
                def
            ))),
        })
    }

    fn index_class(
        &mut self,
        fullname: &ClassFullname,
        typarams: &[String],
        super_name: &ClassFullname,
        defs: &[ast::Definition],
    ) -> Result<(), Error> {
        let metaclass_fullname = fullname.meta_name();
        let new_sig = signature::signature_of_new(
            &metaclass_fullname,
            self.initializer_params(typarams, &super_name.instance_ty(), &defs),
            &ty::return_type_of_new(fullname, typarams),
        );

        let (instance_methods, class_methods) =
            self.index_defs_in_class(fullname, typarams, defs)?;

        match self.sk_classes.get_mut(&fullname) {
            Some(class) => {
                // Merge methods to existing class (Class is reopened)
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
            None => self.add_new_class(fullname, typarams, super_name, new_sig, instance_methods, class_methods)?
        }
        Ok(())
    }

    fn index_defs_in_class(
        &mut self,
        fullname: &ClassFullname,
        typarams: &[String],
        defs: &[ast::Definition],
    ) -> Result<
        (
            HashMap<MethodFirstname, MethodSignature>,
            HashMap<MethodFirstname, MethodSignature>,
        ),
        Error,
    > {
        let mut instance_methods = HashMap::new();
        let mut class_methods = HashMap::new();
        for def in defs {
            match def {
                ast::Definition::InstanceMethodDefinition { sig, .. } => {
                    let hir_sig = signature::create_signature(&fullname, sig, typarams);
                    instance_methods.insert(sig.name.clone(), hir_sig);
                }
                ast::Definition::ClassMethodDefinition { sig, .. } => {
                    let hir_sig = signature::create_signature(&fullname.meta_name(), sig, typarams);
                    class_methods.insert(sig.name.clone(), hir_sig);
                }
                ast::Definition::ConstDefinition { .. } => (),
                ast::Definition::ClassDefinition {
                    name,
                    typarams,
                    super_name,
                    defs,
                } => {
                    let full = name.add_namespace(&fullname.0);
                    self.index_class(&full, &typarams, &super_name, &defs)?;
                }
            }
        }
        Ok((instance_methods, class_methods))
    }

    fn add_new_class(
        &mut self,
        fullname: &ClassFullname,
        typaram_names: &[String],
        super_name: &ClassFullname,
        new_sig: MethodSignature,
        instance_methods: HashMap<MethodFirstname, MethodSignature>,
        mut class_methods: HashMap<MethodFirstname, MethodSignature>,
    ) -> Result<(), Error> {
        let typarams = typaram_names
            .iter()
            .map(|s| TyParam {
                name: s.to_string(),
            })
            .collect::<Vec<_>>();

        // Add `.new` to the metaclass
        class_methods.insert(new_sig.fullname.first_name.clone(), new_sig);
        if !self.class_exists(&super_name.0) {
            return Err(error::name_error(&format!(
                "unknown superclass: {:?}",
                super_name
            )));
        }

        self.add_class(SkClass {
            fullname: fullname.clone(),
            typarams: typarams.clone(),
            superclass_fullname: Some(super_name.clone()),
            instance_ty: ty::raw(&fullname.0),
            ivars: HashMap::new(), // will be set when processing `#initialize`
            method_sigs: instance_methods,
            const_is_obj: false,
        });

        // Crete metaclass (which is a subclass of `Class`)
        let the_class = self.get_class(&class_fullname("Class"), "index_class");
        let meta_ivars = the_class.ivars.clone();
        self.add_class(SkClass {
            fullname: fullname.meta_name(),
            typarams,
            superclass_fullname: Some(class_fullname("Class")),
            instance_ty: ty::meta(&fullname.0),
            ivars: meta_ivars,
            method_sigs: class_methods,
            const_is_obj: false,
        });
        Ok(())
    }
}
