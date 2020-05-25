use std::collections::HashMap;
use crate::ast;
use crate::error;
use crate::error::*;
use crate::hir;
use crate::hir::*;
use crate::ty::*;
use crate::names::*;

#[derive(Debug, PartialEq)]
pub struct ClassDict {
    /// Indexed classes.
    /// Note that .ivars are empty at first (because their types cannot be decided
    /// while indexing)
    pub sk_classes: HashMap<ClassFullname, SkClass>
}

pub fn create(ast: &ast::Program, corelib: HashMap<ClassFullname, SkClass>) -> Result<ClassDict, Error> {
    let mut dict = ClassDict::new();
    dict.index_corelib(corelib);
    dict.index_program(&ast.toplevel_defs)?;
    Ok(dict)
}

impl ClassDict {
    pub fn new() -> ClassDict {
        ClassDict {
            sk_classes: HashMap::new()
        }
    }

    /// Find a method from class name and first name
    pub fn find_method(&self, class_fullname: &ClassFullname, method_name: &MethodFirstname) -> Option<&MethodSignature> {
        self.sk_classes.get(class_fullname).and_then(|class| class.method_sigs.get(method_name))
    }

    /// Similar to find_method, but lookup into superclass if not in the class.
    /// Returns Err if not found.
    pub fn lookup_method(&self,
                         class_fullname: &ClassFullname,
                         method_name: &MethodFirstname)
                         -> Result<(&MethodSignature, ClassFullname), Error> {
        self.lookup_method_(class_fullname, class_fullname, method_name)
    }
    fn lookup_method_(&self,
                      receiver_class_fullname: &ClassFullname,
                      class_fullname: &ClassFullname,
                      method_name: &MethodFirstname)
                         -> Result<(&MethodSignature, ClassFullname), Error> {
        if let Some(sig) = self.find_method(class_fullname, method_name) {
            Ok((sig, class_fullname.clone()))
        }
        else {
            // Look up in superclass
            let sk_class = self.find_class(class_fullname)
                .unwrap_or_else(|| panic!("[BUG] lookup_method: asked to find `{}' but class `{}' not found", &method_name.0, &class_fullname.0));
            if let Some(super_name) = &sk_class.superclass_fullname {
                self.lookup_method_(receiver_class_fullname, super_name, method_name)
            }
            else {
                Err(error::program_error(&format!("method {:?} not found on {:?}", method_name, receiver_class_fullname)))
            }
        }
    }

    /// Find a class
    pub fn find_class(&self, class_fullname: &ClassFullname) -> Option<&SkClass> {
        self.sk_classes.get(class_fullname)
    }

    /// Find a class. Panic if not found
    pub fn get_class(&self,
                     class_fullname: &ClassFullname,
                     dbg_name: &str) -> &SkClass {
        self.find_class(class_fullname)
            .unwrap_or_else(|| panic!("[BUG] {}: class `{}' not found", &dbg_name, &class_fullname.0))
    }

    /// Find a class. Panic if not found
    pub fn get_class_mut(&mut self,
                         class_fullname: &ClassFullname,
                         dbg_name: &str) -> &mut SkClass {
        self.sk_classes.get_mut(&class_fullname)
            .unwrap_or_else(|| panic!("[BUG] {}: class `{}' not found", &dbg_name, &class_fullname.0))
    }

    /// Return true if there is a class of the name
    pub fn class_exists(&self, class_fullname: &str) -> bool {
        self.sk_classes.contains_key(&ClassFullname(class_fullname.to_string()))
    }

    /// Find the superclass
    /// Return None if the class is `Object`
    pub fn get_superclass(&self, classname: &ClassFullname) -> Option<&SkClass> {
        let cls = self.get_class(&classname, "ClassDict::get_superclass");
        cls.superclass_fullname.as_ref().map(|super_name| {
            self.get_class(&super_name, "ClassDict::get_superclass")
        })
    }

    pub fn find_ivar(&self,
                     classname: &ClassFullname,
                     ivar_name: &str) -> Option<&SkIVar> {
        let class = self.sk_classes.get(&classname)
            .unwrap_or_else(|| panic!("[BUG] ClassDict::find_ivar: class `{}' not found", &classname));
        class.ivars.get(ivar_name)
    }

    /// Define ivars of a class
    pub fn define_ivars(&mut self,
                        classname: &ClassFullname,
                        own_ivars: HashMap<String, SkIVar>) -> Result<(), Error> {
        let super_ivars = match self.get_superclass(&classname) {
            Some(super_cls) => super_cls.ivars.clone(),
            None => HashMap::new(),
        };
        let class = self.get_class_mut(&classname, "ClassDict::define_ivars");
        class.ivars = super_ivars;
        own_ivars.into_iter().for_each(|(k, v)| { class.ivars.insert(k, v); });
        Ok(())
    }

    /// Register a class
    fn add_class(&mut self, class: SkClass) {
        self.sk_classes.insert(class.fullname.clone(), class);
    }

    pub fn index_corelib(&mut self, corelib: HashMap<ClassFullname, SkClass>) {
        corelib.into_iter().for_each(|(_, c)| {
            self.add_class(SkClass {
                fullname: c.fullname,
                superclass_fullname: c.superclass_fullname,
                instance_ty: c.instance_ty,
                ivars: c.ivars,
                method_sigs: c.method_sigs
            })
        });
    }

    pub fn index_program(&mut self, toplevel_defs: &[ast::Definition]) -> Result<(), Error> {
        toplevel_defs.iter().try_for_each(|def| {
            match def {
                ast::Definition::ClassDefinition { name, super_name, defs } => {
                    self.index_class(&name.add_namespace(""), &super_name, &defs)?;
                    Ok(())
                },
                ast::Definition::ConstDefinition { .. } => Ok(()),
                _ => {
                    Err(error::syntax_error(&format!("must not be toplevel: {:?}", def)))
                }
            }
        })
    }

    fn index_class(&mut self,
                   fullname: &ClassFullname,
                   super_name: &ClassFullname,
                   defs: &[ast::Definition]) -> Result<(), Error> {
        let instance_ty = ty::raw(&fullname.0);
        let class_ty = instance_ty.meta_ty();

        let metaclass_fullname = class_ty.fullname.clone();
        let mut instance_methods = HashMap::new();
        let mut class_methods = HashMap::new();
        let new_sig = signature_of_new(&metaclass_fullname,
                                      self.initializer_params(&super_name, &defs),
                                      &instance_ty);

        for def in defs {
            match def {
                ast::Definition::InstanceMethodDefinition { sig, .. } => {
                    let hir_sig = crate::hir::create_signature(fullname.to_string(), sig);
                    instance_methods.insert(sig.name.clone(), hir_sig);
                },
                ast::Definition::ClassMethodDefinition { sig, .. } => {
                    let hir_sig = crate::hir::create_signature(metaclass_fullname.to_string(), sig);
                    class_methods.insert(sig.name.clone(), hir_sig);
                },
                ast::Definition::ConstDefinition { .. } => (),
                ast::Definition::ClassDefinition { name, super_name, defs } => {
                    let full = name.add_namespace(&fullname.0);
                    self.index_class(&full, &super_name, &defs)?;
                }
            }
        }

        match self.sk_classes.get_mut(&fullname) {
            Some(class) => {
                // Merge methods to existing class (Class is reopened)
                class.method_sigs.extend(instance_methods);
                let metaclass = self.sk_classes.get_mut(&metaclass_fullname)
                    .expect("[BUG] Only class is indexed");
                metaclass.method_sigs.extend(class_methods);
                // Add `.new` to the metaclass
                if !metaclass.method_sigs.contains_key(&method_firstname("new")) {
                    metaclass.method_sigs.insert(new_sig.fullname.first_name.clone(), new_sig);
                }
            },
            None => {
                // Add `.new` to the metaclass
                class_methods.insert(new_sig.fullname.first_name.clone(), new_sig);
                if !self.class_exists(&super_name.0) {
                    return Err(error::name_error(&format!("unknown superclass: {:?}", super_name)))
                }
                self.add_class(SkClass {
                    fullname: fullname.clone(),
                    superclass_fullname: Some(super_name.clone()),
                    instance_ty,
                    ivars: HashMap::new(),
                    method_sigs: instance_methods,
                });
                self.add_class(SkClass {
                    fullname: metaclass_fullname,
                    superclass_fullname: Some(class_fullname("Class")),
                    instance_ty: class_ty,
                    ivars: HashMap::new(),
                    method_sigs: class_methods,
                });
            }
        }
        Ok(())
    }

    /// Return parameters of `initialize`
    fn initializer_params(&self,
                          clsname: &ClassFullname,
                          defs: &[ast::Definition]) -> Vec<MethodParam> {
        if let Some(ast::Definition::InstanceMethodDefinition { sig, .. }) = defs.iter().find(|d| d.is_initializer()) {
            hir::convert_params(&sig.params)
        }
        else {
            let (sig, _found_cls) = 
                self.lookup_method(&clsname, &method_firstname("initialize"))
                    .expect("[BUG] initialize not found");
            sig.params.clone()
        }
    }
}
