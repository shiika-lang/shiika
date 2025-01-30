use crate::*;
use anyhow::Result;
use shiika_core::names::Namespace;

pub trait AstVisitor {
    /// Called for each constant definition
    fn visit_const_definition(
        &mut self,
        _namespace: &Namespace,
        _name: &str,
        _expr: &AstExpression,
    ) -> Result<()> {
        Ok(())
    }

    /// Called for each method definition
    fn visit_method_definition(
        &mut self,
        _namespace: &Namespace,
        _instance: bool,
        _initializer: bool,
        _sig: &AstMethodSignature,
        _body_exprs: &Vec<AstExpression>,
    ) -> Result<()> {
        Ok(())
    }

    /// Called for each toplevel expression
    fn visit_toplevel_expr(&mut self, _expr: &AstExpression) -> Result<()> {
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
            TopLevelItem::Expr(expr) => {
                self.visit_toplevel_expr(expr)?;
            }
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

            Definition::InstanceMethodDefinition { sig, body_exprs } => {
                self.visit_method_definition(namespace, true, false, sig, body_exprs)?;
            }
            Definition::InitializerDefinition(d) => {
                self.visit_method_definition(namespace, true, true, &d.sig, &d.body_exprs)?;
            }
            Definition::ClassMethodDefinition { sig, body_exprs } => {
                self.visit_method_definition(namespace, false, false, sig, body_exprs)?;
            }
            Definition::ClassInitializerDefinition(d) => {
                self.visit_method_definition(namespace, false, true, &d.sig, &d.body_exprs)?;
            }

            Definition::ConstDefinition { name, expr } => {
                self.visit_const_definition(namespace, name, expr)?;
            }

            Definition::MethodRequirementDefinition { .. } => {}
        }
        Ok(())
    }
}
