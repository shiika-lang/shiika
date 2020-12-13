use crate::ast;
use crate::error;
use crate::error::*;
use crate::hir::class_dict::class_dict::ClassDict;
use crate::hir::signature;
use crate::hir::*;
use crate::names::*;
use crate::ty::*;
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
                typarams: vec![],
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
        let instance_ty = ty::raw(&fullname.0);
        let class_ty = instance_ty.meta_ty();

        let metaclass_fullname = class_ty.fullname.clone();
        let mut instance_methods = HashMap::new();
        let mut class_methods = HashMap::new();
        let new_sig = signature::signature_of_new(
            &metaclass_fullname,
            self.initializer_params(&super_name.instance_ty(), &defs),
            &ty::return_type_of_new(fullname, typarams),
        );

        for def in defs {
            match def {
                ast::Definition::InstanceMethodDefinition { sig, .. } => {
                    let hir_sig = signature::create_signature(&fullname, sig, typarams);
                    instance_methods.insert(sig.name.clone(), hir_sig);
                }
                ast::Definition::ClassMethodDefinition { sig, .. } => {
                    let hir_sig = signature::create_signature(&metaclass_fullname, sig, &[]);
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
            None => {
                let ty_params = typarams.iter().map(|s| TyParam { name: s.to_string() }).collect::<Vec<_>>();
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
                    typarams: ty_params.clone(),
                    superclass_fullname: Some(super_name.clone()),
                    instance_ty,
                    ivars: HashMap::new(),
                    method_sigs: instance_methods,
                    const_is_obj: false,
                });
                self.add_class(SkClass {
                    fullname: metaclass_fullname,
                    typarams: ty_params,
                    superclass_fullname: Some(class_fullname("Class")),
                    instance_ty: class_ty,
                    ivars: HashMap::new(),
                    method_sigs: class_methods,
                    const_is_obj: false,
                });
            }
        }
        Ok(())
    }
}
