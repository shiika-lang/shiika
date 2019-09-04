/// Index of all the classes and methods
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
use crate::stdlib::Stdlib;

#[derive(Debug, PartialEq)]
pub struct Index {
    body: IndexBody
}
type IndexBody = HashMap<ClassFullname, HashMap<MethodName, MethodSignature>>;

impl Index {
    pub fn new(stdlib: &Stdlib, toplevel_defs: &Vec<ast::Definition>) -> Result<Index, Error> {
        let mut index = Index { body: HashMap::new() };
        index.index_stdlib(stdlib);
        index.index_program(toplevel_defs)?;
        Ok(index)
    }

    /// Find a method from class name and first name
    pub fn find_method(&self, class_fullname: &ClassFullname, method_name: &MethodName) -> Option<&MethodSignature> {
        self.body.get(class_fullname).and_then(|methods| methods.get(method_name))
    }

    /// Return true if there is a class of the name
    pub fn class_exists(&self, class_fullname: &str) -> bool {
        self.body.contains_key(&ClassFullname(class_fullname.to_string()))
    }

    /// Register a class and its methods
    fn add_class(&mut self, class_fullname: ClassFullname, sk_methods: Vec<SkMethod>) {
        self.body.insert(class_fullname, sk_methods)
    }

    fn index_stdlib(&mut self, stdlib: &Stdlib) {
        stdlib.sk_classes.values().for_each(|sk_class| {
            let mut sk_methods = HashMap::new();
            sk_class.method_sigs.iter().for_each(|sig| {
                sk_methods.insert(sig.name.clone(), sig.clone());
            });
            self.add_class(sk_class.fullname.clone(), sk_methods);
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

        let metaclass_fullname = class_ty.fullname;
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

        self.add_class(class_fullname, instance_methods);
        self.add_class(metaclass_fullname, class_methods);
    }
}
