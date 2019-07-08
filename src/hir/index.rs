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

// class_fullname => method_name => signature
pub type Index = HashMap<String, HashMap<String, MethodSignature>>;

pub fn new(stdlib: &Vec<SkClass>, toplevel_defs: &Vec<ast::Definition>) -> Result<Index, Error> {
    let mut index = HashMap::new();

    index_stdlib(&mut index, stdlib);
    index_program(&mut index, toplevel_defs)?;

    Ok(index)
}

fn index_stdlib(index: &mut Index, stdlib: &Vec<SkClass>) {
    stdlib.iter().for_each(|sk_class| {
        let mut sk_methods = HashMap::new();
        sk_class.methods.iter().for_each(|sk_method| {
            sk_methods.insert(sk_method.signature.name.to_string(),
                              sk_method.signature.clone());
        });
        index.insert(sk_class.fullname.to_string(), sk_methods);
    });
}

fn index_program(index: &mut Index, toplevel_defs: &Vec<ast::Definition>) -> Result<(), Error> {
    toplevel_defs.iter().try_for_each(|def| {
        match def {
            ast::Definition::ClassDefinition { name, defs } => {
                index_class(index, &name, &defs);
                Ok(())
            },
            _ => {
                Err(error::syntax_error(&format!("must not be toplevel: {:?}", def)))
            }
        }
    })
}

fn index_class(index: &mut Index, name: &str, defs: &Vec<ast::Definition>) {
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

    index.insert(class_fullname.to_string(), sk_methods);
}

fn create_signature(class_fullname: String, name: String, params: &Vec<ast::Param>, ret_typ: &ast::Typ) -> MethodSignature {
    let fullname = class_fullname + "#" + &name;
    let ret_ty = convert_typ(ret_typ);
    let param_tys = params.iter().map(|param|
        convert_typ(&param.typ)
    ).collect();

    MethodSignature { name, fullname, ret_ty, param_tys }
}

fn convert_typ(typ: &ast::Typ) -> TermTy {
    ty::raw(&typ.name)
}
