mod location;
mod token;
pub use crate::location::{Location, LocationSpan};
pub use crate::token::Token;
use shiika_core::names::*;

#[derive(Debug, PartialEq)]
pub struct Program {
    pub toplevel_items: Vec<TopLevelItem>,
}

impl Program {
    pub fn default() -> Program {
        Program {
            toplevel_items: vec![],
        }
    }

    pub fn append(&mut self, other: &mut Program) {
        self.toplevel_items.append(&mut other.toplevel_items);
    }
}

#[derive(Debug, PartialEq)]
pub enum TopLevelItem {
    Def(Definition),
    Expr(AstExpression),
}

#[derive(Debug, PartialEq)]
pub enum Definition {
    ClassDefinition {
        name: ClassFirstname,
        typarams: Vec<AstTyParam>,
        supers: Vec<UnresolvedTypeName>,
        defs: Vec<Definition>,
    },
    ModuleDefinition {
        name: ModuleFirstname,
        typarams: Vec<AstTyParam>,
        defs: Vec<Definition>,
    },
    EnumDefinition {
        name: ClassFirstname,
        typarams: Vec<AstTyParam>,
        cases: Vec<EnumCase>,
        defs: Vec<Definition>,
    },
    InstanceMethodDefinition {
        sig: AstMethodSignature,
        body_exprs: Vec<AstExpression>,
    },
    InitializerDefinition(InitializerDefinition),
    ClassMethodDefinition {
        sig: AstMethodSignature,
        body_exprs: Vec<AstExpression>,
    },
    ClassInitializerDefinition(InitializerDefinition),
    MethodRequirementDefinition {
        sig: AstMethodSignature,
    },
    ConstDefinition {
        name: String,
        expr: AstExpression,
    },
}

#[derive(Debug, PartialEq)]
pub struct InitializerDefinition {
    pub sig: AstMethodSignature,
    pub body_exprs: Vec<AstExpression>,
}

pub fn find_initializer(defs: &[Definition]) -> Option<&InitializerDefinition> {
    defs.iter().find_map(|def| match def {
        Definition::InitializerDefinition(x) => Some(x),
        _ => None,
    })
}

pub fn find_class_initializer(defs: &[Definition]) -> Option<&InitializerDefinition> {
    defs.iter().find_map(|def| match def {
        Definition::ClassInitializerDefinition(x) => Some(x),
        _ => None,
    })
}

#[derive(Debug, PartialEq)]
pub struct EnumCase {
    pub name: ClassFirstname,
    pub params: Vec<Param>,
}

#[derive(Debug, PartialEq)]
pub struct AstMethodSignature {
    pub name: MethodFirstname,
    pub typarams: Vec<AstTyParam>,
    pub params: Vec<Param>,
    pub ret_typ: Option<UnresolvedTypeName>,
}

/// A type parameter
#[derive(Debug, PartialEq, Clone)]
pub struct AstTyParam {
    pub name: String,
    pub variance: AstVariance,
}

#[derive(Debug, PartialEq, Clone)]
pub enum AstVariance {
    Invariant,
    Covariant,     // eg. `in T`
    Contravariant, // eg. `out T`
}

#[derive(Debug, PartialEq, Clone)]
pub struct Param {
    pub name: String,
    pub typ: UnresolvedTypeName,
    pub is_iparam: bool, // eg. `def initialize(@a: Int)`
}

#[derive(Debug, PartialEq, Clone)]
pub struct BlockParam {
    pub name: String,
    pub opt_typ: Option<UnresolvedTypeName>,
}

/// A type name not yet resolved.
/// eg. for `A::B<C>`, `names` is `A, B` and `args` is `C`.
#[derive(Debug, PartialEq, Clone)]
pub struct UnresolvedTypeName {
    pub names: Vec<String>,
    pub args: Vec<UnresolvedTypeName>,
    pub locs: LocationSpan,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AstExpression {
    pub body: AstExpressionBody,
    pub primary: bool,
    pub locs: LocationSpan,
}

#[derive(Debug, PartialEq, Clone)]
pub enum AstExpressionBody {
    LogicalNot {
        expr: Box<AstExpression>,
    },
    LogicalAnd {
        left: Box<AstExpression>,
        right: Box<AstExpression>,
    },
    LogicalOr {
        left: Box<AstExpression>,
        right: Box<AstExpression>,
    },
    If {
        cond_expr: Box<AstExpression>,
        then_exprs: Vec<AstExpression>,
        else_exprs: Option<Vec<AstExpression>>,
    },
    Match {
        cond_expr: Box<AstExpression>,
        clauses: Vec<AstMatchClause>,
    },
    While {
        cond_expr: Box<AstExpression>,
        body_exprs: Vec<AstExpression>,
    },
    Break,
    Return {
        arg: Option<Box<AstExpression>>,
    },
    LVarAssign {
        name: String,
        rhs: Box<AstExpression>,
        /// Whether declared with `var` (TODO: rename to `readonly`?)
        is_var: bool,
    },
    IVarAssign {
        name: String,
        rhs: Box<AstExpression>,
        /// Whether declared with `var`
        is_var: bool,
    },
    ConstAssign {
        names: Vec<String>,
        rhs: Box<AstExpression>,
    },
    MethodCall(AstMethodCall),
    LambdaExpr {
        params: Vec<BlockParam>,
        exprs: Vec<AstExpression>,
        /// true if this is from `fn(){}`. false if this is a block (do-end/{})
        is_fn: bool,
    },
    // Local variable reference or method call with implicit receiver(self)
    BareName(String),
    IVarRef(String),
    CapitalizedName(UnresolvedConstName),
    SpecializeExpression {
        base_name: UnresolvedConstName,
        args: Vec<AstExpression>,
    },
    PseudoVariable(Token),
    ArrayLiteral(Vec<AstExpression>),
    FloatLiteral {
        value: f64,
    },
    DecimalLiteral {
        value: i64,
    },
    StringLiteral {
        content: String,
    },
}

/// Method call has its own struct
#[derive(Debug, PartialEq, Clone)]
pub struct AstMethodCall {
    pub receiver_expr: Option<Box<AstExpression>>, // Box is needed for E0072 "has infinite size" error
    pub method_name: MethodFirstname,
    pub arg_exprs: Vec<AstExpression>,
    pub type_args: Vec<AstExpression>,
    pub has_block: bool,
    pub may_have_paren_wo_args: bool,
}

impl AstMethodCall {
    pub fn first_arg_cloned(&self) -> AstExpression {
        self.arg_exprs[0].clone()
    }
}

/// Patterns of match expression
#[derive(Debug, PartialEq, Clone)]
pub enum AstPattern {
    ExtractorPattern {
        names: Vec<String>,
        params: Vec<AstPattern>,
    },
    VariablePattern(String),
    BooleanLiteralPattern(bool),
    IntegerLiteralPattern(i64),
    FloatLiteralPattern(f64),
    StringLiteralPattern(String),
}

pub type AstMatchClause = (AstPattern, Vec<AstExpression>);

impl AstExpression {
    pub fn may_have_paren_wo_args(&self) -> bool {
        match &self.body {
            AstExpressionBody::MethodCall(x) => x.may_have_paren_wo_args,
            AstExpressionBody::BareName(_) => true,
            _ => false,
        }
    }

    /// True if this can be the left hand side of an assignment
    pub fn is_lhs(&self) -> bool {
        if self.may_have_paren_wo_args() {
            return true;
        }
        match &self.body {
            AstExpressionBody::IVarRef(_) => true,
            AstExpressionBody::CapitalizedName(_) => true,
            AstExpressionBody::MethodCall(x) => x.method_name.0 == "[]",
            _ => false,
        }
    }

    /// If `self` is ConstAssign, convert it to a ConstDefinition
    pub fn as_const_def(&self) -> Option<Definition> {
        if let AstExpressionBody::ConstAssign { names, rhs } = &self.body {
            Some(Definition::ConstDefinition {
                name: names.join("::"),
                expr: *rhs.clone(),
            })
        } else {
            None
        }
    }
}
