mod indexing;
mod query;
use crate::ast;
use crate::error::*;
use crate::hir;
use crate::hir::*;
use crate::names::*;
use crate::ty::*;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Default)]
pub struct ClassDict {
    /// Indexed classes.
    /// Note that .ivars are empty at first (because their types cannot be decided
    /// while indexing)
    pub sk_classes: HashMap<ClassFullname, SkClass>,
}

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
    fn initializer_params(&self, typarams: &[String], super_class: &TermTy, defs: &[ast::Definition]) -> Vec<MethodParam> {
        if let Some(ast::Definition::InstanceMethodDefinition { sig, .. }) =
            defs.iter().find(|d| d.is_initializer())
        {
            // Has explicit initializer definition
            hir::signature::convert_params(&sig.params, typarams, &[])
        } else {
            // Inherit #initialize from superclass
            let (sig, _) = self
                .lookup_method(&super_class, &method_firstname("initialize"))
                .expect("[BUG] initialize not found");
            sig.params
        }
    }
}
