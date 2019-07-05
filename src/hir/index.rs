use std::collections::HashMap;
use crate::ast;
use crate::error::*;
use crate::hir::*;
use crate::ty::*;

// class_fullname => method_name => signature
pub type Index = HashMap<String, HashMap<String, MethodSignature>>;

pub fn new(stdlib: &HashMap<String, SkClass>, toplevel_defs: &Vec<ast::Definition>) -> Result<Index, Error> {
    let mut index = HashMap::new();

    index_stdlib(&mut index, stdlib);

    Ok(index)
}

fn index_stdlib(index: &mut Index, stdlib: &HashMap<String, SkClass>) {
    stdlib.values().for_each(|sk_class| {
        let mut sk_methods = HashMap::new();
        sk_class.methods.values().for_each(|sk_method| {
            // TODO: stdlib should create Index rather than clone them
            sk_methods.insert(sk_method.signature.name.to_string(),
                              sk_method.signature.clone());
        });
        index.insert(sk_class.fullname.to_string(), sk_methods);
    });
}
