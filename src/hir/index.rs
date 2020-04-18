/// Index of all the classes and method signatures
///
/// Note: `MethodSignature` contained in `Index` is "as is" and
/// may be wrong (eg. its return type does not exist).
/// It is checked in `HirMaker`.
use std::collections::HashMap;
use crate::ast;
use crate::error;
use crate::error::*;
use crate::hir::*;
use crate::ty::*;
use crate::names::*;

#[derive(Debug, PartialEq)]
pub struct Index {
    // TODO: Rename to `idx_classes`
    pub classes: HashMap<ClassFullname, IdxClass>
}

#[derive(Debug, PartialEq)]
pub struct IdxClass {
    pub fullname: ClassFullname,
    pub superclass_fullname: Option<ClassFullname>,
    pub instance_ty: TermTy,
    pub ivars: Rc<HashMap<String, SkIVar>>,
    pub method_sigs: HashMap<MethodFirstname, MethodSignature>,
}

impl Index {
    pub fn new() -> Index {
        Index {
            classes: HashMap::new()
        }
    }

    /// Find a method from class name and first name
    pub fn find_method(&self, class_fullname: &ClassFullname, method_name: &MethodFirstname) -> Option<&MethodSignature> {
        self.classes.get(class_fullname).and_then(|class| class.method_sigs.get(method_name))
    }

    /// Find a class
    pub fn find_class(&self, class_fullname: &ClassFullname) -> Option<&IdxClass> {
        self.classes.get(class_fullname)
    }

    /// Return true if there is a class of the name
    pub fn class_exists(&self, class_fullname: &str) -> bool {
        self.classes.contains_key(&ClassFullname(class_fullname.to_string()))
    }

    /// Register a class
    fn add_class(&mut self, class: IdxClass) {
        self.classes.insert(class.fullname.clone(), class);
    }

    pub fn index_corelib(&mut self, corelib: HashMap<ClassFullname, SkClass>) {
        corelib.into_iter().for_each(|(_, c)| {
            self.add_class(IdxClass {
                fullname: c.fullname,
                superclass_fullname: c.superclass_fullname,
                instance_ty: c.instance_ty,
                ivars: c.ivars,
                method_sigs: c.method_sigs
            })
        });
    }

    pub fn index_program(&mut self, toplevel_defs: &Vec<ast::Definition>) -> Result<(), Error> {
        toplevel_defs.iter().try_for_each(|def| {
            match def {
                ast::Definition::ClassDefinition { name, defs } => {
                    self.index_class(&name, &defs);
                    Ok(())
                },
                ast::Definition::ConstDefinition { .. } => Ok(()),
                _ => {
                    Err(error::syntax_error(&format!("must not be toplevel: {:?}", def)))
                }
            }
        })
    }

    fn index_class(&mut self, name: &ClassFirstname, defs: &Vec<ast::Definition>) {
        let class_fullname = name.to_class_fullname(); // TODO: nested class
        let instance_ty = ty::raw(&class_fullname.0);
        let class_ty = instance_ty.meta_ty();

        let metaclass_fullname = class_ty.fullname.clone();
        let mut instance_methods = HashMap::new();
        let mut class_methods = HashMap::new();
        let new_sig = signature_of_new(&metaclass_fullname,
                                       initializer_params(&defs).unwrap_or(&vec![]),
                                       &instance_ty);

        defs.iter().for_each(|def| {
            match def {
                ast::Definition::InstanceMethodDefinition { sig, .. } => {
                    let hir_sig = crate::hir::create_signature(class_fullname.to_string(), sig);
                    instance_methods.insert(sig.name.clone(), hir_sig);
                },
                ast::Definition::ClassMethodDefinition { sig, .. } => {
                    let hir_sig = crate::hir::create_signature(metaclass_fullname.to_string(), sig);
                    class_methods.insert(sig.name.clone(), hir_sig);
                },
                ast::Definition::ConstDefinition { .. } => (),
                _ => panic!("TODO")
            }
        });

        match self.classes.get_mut(&class_fullname) {
            Some(class) => {
                // Merge methods to existing class (Class is reopened)
                class.method_sigs.extend(instance_methods);
                let metaclass = self.classes.get_mut(&metaclass_fullname)
                    .expect("[BUG] Only class is indexed");
                metaclass.method_sigs.extend(class_methods);
                // Add `.new` to the metaclass
                if !metaclass.method_sigs.contains_key(&MethodFirstname("new".to_string())) {
                    metaclass.method_sigs.insert(new_sig.fullname.first_name.clone(), new_sig);
                }
            },
            None => {
                // Add `.new` to the metaclass
                class_methods.insert(new_sig.fullname.first_name.clone(), new_sig);
                self.add_class(IdxClass {
                    fullname: class_fullname,
                    superclass_fullname: if name.0 == "Object" { None }
                                         else { Some(ClassFullname("Object".to_string())) },
                    instance_ty: instance_ty,
                    ivars: Rc::new(HashMap::new()),
                    method_sigs: instance_methods,
                });
                self.add_class(IdxClass {
                    fullname: metaclass_fullname,
                    superclass_fullname: Some(ClassFullname("Object".to_string())),
                    instance_ty: class_ty,
                    ivars: Rc::new(HashMap::new()),
                    method_sigs: class_methods,
                });
            }
        }
    }
}

/// Return parameters of `initialize`
fn initializer_params(defs: &Vec<ast::Definition>) -> Option<&Vec<ast::Param>> {
    match defs.iter().find(|d| d.is_initializer()) {
        Some(ast::Definition::InstanceMethodDefinition { sig, .. }) => {
            Some(&sig.params)
        },
        // `initialize` takes no args
        // TODO: may be inheriting superclass's initialize
        _ => None
    }
}
