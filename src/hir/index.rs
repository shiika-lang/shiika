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

#[derive(Debug, PartialEq)]
pub struct Index {
    pub body: IndexBody
}
// class_fullname => method_name => signature
type IndexBody = HashMap<String, HashMap<String, MethodSignature>>;

impl Index {
    pub fn new(stdlib: &Vec<SkClass>, toplevel_defs: &Vec<ast::Definition>) -> Result<Index, Error> {
        let mut body = HashMap::new();

        index_stdlib(&mut body, stdlib);
        index_program(&mut body, toplevel_defs)?;

        Ok(Index { body })
    }

    pub fn get(&self, class_fullname: &str) -> Option<&HashMap<String, MethodSignature>> {
        self.body.get(class_fullname)
    }

    pub fn find_method(&self, class_fullname: &str, method_name: &str) -> Option<&MethodSignature> {
        self.body.get(class_fullname).and_then(|methods| methods.get(method_name))
    }
}

fn index_stdlib(body: &mut IndexBody, stdlib: &Vec<SkClass>) {
    stdlib.iter().for_each(|sk_class| {
        let mut sk_methods = HashMap::new();
        sk_class.methods.iter().for_each(|sk_method| {
            sk_methods.insert(sk_method.signature.name.to_string(),
                              sk_method.signature.clone());
        });
        body.insert(sk_class.fullname.to_string(), sk_methods);
    });
}

fn index_program(body: &mut IndexBody, toplevel_defs: &Vec<ast::Definition>) -> Result<(), Error> {
    toplevel_defs.iter().try_for_each(|def| {
        match def {
            ast::Definition::ClassDefinition { name, defs } => {
                index_class(body, &name, &defs);
                Ok(())
            },
            _ => {
                Err(error::syntax_error(&format!("must not be toplevel: {:?}", def)))
            }
        }
    })
}

fn index_class(body: &mut IndexBody, name: &str, defs: &Vec<ast::Definition>) {
    let class_fullname = name; // TODO: nested class
    let mut sk_methods = HashMap::new();
    defs.iter().for_each(|def| {
        match def {
            ast::Definition::InstanceMethodDefinition { name, params, ret_typ, .. } => {
                let sig = create_signature(class_fullname.to_string(), name.to_string(),
                                           &params, &ret_typ);
                sk_methods.insert(name.to_string(), sig);
            },
            _ => panic!("TODO")
        }
    });

    body.insert(class_fullname.to_string(), sk_methods);
}

fn create_signature(class_fullname: String, name: String, params: &Vec<ast::Param>, ret_typ: &ast::Typ) -> MethodSignature {
    let fullname = class_fullname + "#" + &name;
    let ret_ty = convert_typ(ret_typ);
    let params = params.iter().map(|param|
        MethodParam { name: param.name.to_string(), ty: convert_typ(&param.typ) }
    ).collect();

    MethodSignature { name, fullname, ret_ty, params }
}

fn convert_typ(typ: &ast::Typ) -> TermTy {
    ty::raw(&typ.name)
}
