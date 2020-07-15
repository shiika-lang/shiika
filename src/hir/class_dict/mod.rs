pub mod class_dict;
mod indexing;
mod query;
use std::collections::HashMap;
use crate::ast;
use crate::error::*;
use crate::hir;
use crate::hir::*;
use crate::hir::class_dict::class_dict::ClassDict;
use crate::ty::*;
use crate::names::*;

pub fn create(ast: &ast::Program, corelib: HashMap<ClassFullname, SkClass>) -> Result<ClassDict, Error> {
    let mut dict = ClassDict::new();
    dict.index_corelib(corelib);
    let defs = ast.toplevel_items.iter().filter_map(|item| {
        match item {
            ast::TopLevelItem::Def(x) => Some(x),
            ast::TopLevelItem::Expr(_) => None,
        }
    }).collect::<Vec<_>>();
    dict.index_program(&defs)?;
    Ok(dict)
}

impl ClassDict {
    pub fn new() -> ClassDict {
        ClassDict {
            sk_classes: HashMap::new()
        }
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

