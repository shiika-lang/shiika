use crate::hir::{HirExpression, HirExpressions};

#[derive(Debug, Clone)]
pub enum Component {
    /// A boolean expression that is a part of match condition
    Test(HirExpression),
    /// A local variable binding introduced by match
    Bind(String, HirExpression),
}

#[derive(Debug, Clone)]
pub struct MatchClause {
    pub components: Vec<Component>,
    pub body_hir: HirExpressions,
}
