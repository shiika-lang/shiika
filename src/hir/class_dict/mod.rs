mod indexing;
mod query;
use crate::ast;
use crate::error::*;
use crate::hir;
use crate::hir::*;
use crate::names::*;
use crate::ty::*;

#[derive(Debug, PartialEq)]
pub struct ClassDict<'hir_maker> {
    /// Indexed classes.
    /// Note that .ivars are empty at first (because their types cannot be decided
    /// while indexing)
    pub sk_classes: SkClasses,
    /// Imported classes
    imported_classes: &'hir_maker SkClasses,
}

pub fn create<'hir_maker>(
    ast: &ast::Program,
    // Corelib classes (REFACTOR: corelib should provide methods only)
    initial_sk_classes: SkClasses,
    imported_classes: &'hir_maker SkClasses,
) -> Result<ClassDict<'hir_maker>, Error> {
    let mut dict = ClassDict {
        sk_classes: initial_sk_classes,
        imported_classes: imported_classes,
    };
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

impl<'hir_maker> ClassDict<'hir_maker> {
    /// Return parameters of `initialize`
    fn initializer_params(
        &self,
        typarams: &[String],
        super_class: &TermTy,
        defs: &[ast::Definition],
    ) -> Vec<MethodParam> {
        if let Some(ast::Definition::InstanceMethodDefinition { sig, .. }) =
            defs.iter().find(|d| d.is_initializer())
        {
            // Has explicit initializer definition
            hir::signature::convert_params(&sig.params, typarams, &[])
        } else {
            // Inherit #initialize from superclass
            let (sig, _) = self
                .lookup_method(&super_class, &method_firstname("initialize"), &[])
                .expect("[BUG] initialize not found");
            sig.params
        }
    }
}
