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
pub struct Index(HashMap<ClassFullname, SkClass>);

impl Index {
    pub fn new(stdlib_classes: HashMap<ClassFullname, SkClass>,
               toplevel_defs: &Vec<ast::Definition>) -> Result<Index, Error> {
        let mut index = Index(HashMap::new());
        index.index_stdlib(stdlib_classes);
        index.index_program(toplevel_defs)?;
        Ok(index)
    }

    pub fn sk_classes(self) -> HashMap<ClassFullname, SkClass> {
        self.0
    }

    /// Find a method from class name and first name
    pub fn find_method(&self, class_fullname: &ClassFullname, method_name: &MethodFirstName) -> Option<&MethodSignature> {
        self.0.get(class_fullname).and_then(|class| class.method_sigs.get(method_name))
    }

    /// Find a class
    pub fn find_class(&self, class_fullname: &ClassFullname) -> Option<&SkClass> {
        self.0.get(class_fullname)
    }

    /// Return true if there is a class of the name
    pub fn class_exists(&self, class_fullname: &str) -> bool {
        self.0.contains_key(&ClassFullname(class_fullname.to_string()))
    }

    /// Register a class
    fn add_class(&mut self, class: SkClass) {
        self.0.insert(class.fullname.clone(), class);
    }

    fn index_stdlib(&mut self, stdlib_classes: HashMap<ClassFullname, SkClass>) {
        stdlib_classes.into_iter().for_each(|(_, sk_class)| {
            self.add_class(sk_class);
        });
    }

    fn index_program(&mut self, toplevel_defs: &Vec<ast::Definition>) -> Result<(), Error> {
        toplevel_defs.iter().try_for_each(|def| {
            match def {
                ast::Definition::ClassDefinition { name, defs } => {
                    self.index_class(&name, &defs);
                    Ok(())
                },
                _ => {
                    Err(error::syntax_error(&format!("must not be toplevel: {:?}", def)))
                }
            }
        })
    }

    fn index_class(&mut self, name: &ClassName, defs: &Vec<ast::Definition>) {
        let class_fullname = name.to_class_fullname(); // TODO: nested class
        let instance_ty = ty::raw(&class_fullname.0);
        let class_ty = instance_ty.meta_ty();

        let metaclass_fullname = class_ty.fullname.clone();
        let mut instance_methods = HashMap::new();
        let mut class_methods = HashMap::new();

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
                _ => panic!("TODO")
            }
        });

        self.add_class(SkClass {
            fullname: class_fullname,
            superclass_fullname: if name.0 == "Object" { None }
                                 else { Some(ClassFullname("Object".to_string())) },
            instance_ty: instance_ty,
            method_sigs: instance_methods,
        });
        self.add_class(SkClass {
            fullname: metaclass_fullname,
            superclass_fullname: Some(ClassFullname("Object".to_string())),
            instance_ty: class_ty,
            method_sigs: class_methods,
        });
    }
}
