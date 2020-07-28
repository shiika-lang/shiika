mod class_dict;
pub use crate::hir::class_dict::class_dict::ClassDict;
mod indexing;
mod query;
use crate::ast;
use crate::error::*;
use crate::hir;
use crate::hir::*;
use crate::names::*;
use crate::ty::*;
use std::collections::HashMap;

pub fn create(
    ast: &ast::Program,
    corelib: HashMap<ClassFullname, SkClass>,
) -> Result<ClassDict, Error> {
    let mut dict = ClassDict::default();
    dict.index_corelib(corelib);
    let defs = ast
        .toplevel_items
        .iter()
        .filter_map(|item| match item {
            ast::TopLevelItem::Def(x) => Some(x),
            ast::TopLevelItem::Expr(_) => None,
        })
        .collect::<Vec<_>>();
    dict.index_program(&defs)?;
    Ok(dict)
}

impl ClassDict {
    /// Return parameters of `initialize`
    fn initializer_params(&self, class: &TermTy, defs: &[ast::Definition]) -> Vec<MethodParam> {
        if let Some(ast::Definition::InstanceMethodDefinition { sig, .. }) =
            defs.iter().find(|d| d.is_initializer())
        {
            // Has explicit initializer definition
            // TODO: Support typarams in initializer params
            hir::signature::convert_params(&sig.params, &vec![])
        } else {
            // Inherit #initialize from superclass
            let (sig, _found_cls) = self
                .lookup_method(&class, &method_firstname("initialize"))
                .expect("[BUG] initialize not found");
            sig.params.clone()
        }
    }
}
