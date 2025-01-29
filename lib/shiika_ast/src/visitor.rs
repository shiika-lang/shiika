use crate::*;
use anyhow::Result;
use shiika_core::names::Namespace;

pub trait AstVisitor {
    /// Callback function.
    fn visit_const_definition(
        &mut self,
        _namespace: &Namespace,
        _name: &str,
        _expr: &AstExpression,
    ) -> Result<()> {
        Ok(())
    }

    fn walk_program(&mut self, program: &Program) -> Result<()> {
        let namespace = Namespace::root();
        for item in &program.toplevel_items {
            self.walk_toplevel_item(&namespace, item)?;
        }
        Ok(())
    }

    fn walk_toplevel_item(&mut self, namespace: &Namespace, item: &TopLevelItem) -> Result<()> {
        match item {
            TopLevelItem::Def(def) => {
                self.walk_definition(namespace, def)?;
            }
            TopLevelItem::Expr(_) => {}
        }
        Ok(())
    }

    fn walk_definition(&mut self, namespace: &Namespace, def: &Definition) -> Result<()> {
        match &def {
            Definition::ClassDefinition { name, defs, .. } => {
                let inner_ns = namespace.add(name.0.clone());
                for def in defs {
                    self.walk_definition(&inner_ns, def)?;
                }
            }
            Definition::ModuleDefinition { name, defs, .. } => {
                let inner_ns = namespace.add(name.0.clone());
                for def in defs {
                    self.walk_definition(&inner_ns, def)?;
                }
            }
            Definition::EnumDefinition { name, defs, .. } => {
                let inner_ns = namespace.add(name.0.clone());
                for def in defs {
                    self.walk_definition(&inner_ns, def)?;
                }
            }
            Definition::ConstDefinition { name, expr } => {
                self.visit_const_definition(namespace, name, expr)?;
            }
            _ => {}
        }
        Ok(())
    }
}
