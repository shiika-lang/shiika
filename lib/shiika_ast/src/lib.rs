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
    MethodCall {
        receiver_expr: Option<Box<AstExpression>>, // Box is needed to aboid E0072
        method_name: MethodFirstname,
        arg_exprs: Vec<AstExpression>,
        type_args: Vec<AstExpression>,
        has_block: bool,
        may_have_paren_wo_args: bool,
    },
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
        match self.body {
            AstExpressionBody::MethodCall {
                may_have_paren_wo_args,
                ..
            } => may_have_paren_wo_args,
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
            AstExpressionBody::MethodCall { method_name, .. } => method_name.0 == "[]",
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

/// Create an expression for an assigment
pub fn assignment(lhs: AstExpression, rhs: AstExpression) -> AstExpression {
    let body = match lhs.body {
        AstExpressionBody::BareName(s) => AstExpressionBody::LVarAssign {
            name: s,
            rhs: Box::new(rhs),
            is_var: false,
        },
        AstExpressionBody::IVarRef(name) => AstExpressionBody::IVarAssign {
            name,
            rhs: Box::new(rhs),
            is_var: false,
        },
        AstExpressionBody::CapitalizedName(names) => AstExpressionBody::ConstAssign {
            names: names.0,
            rhs: Box::new(rhs),
        },
        AstExpressionBody::MethodCall {
            receiver_expr,
            method_name,
            mut arg_exprs,
            ..
        } => {
            arg_exprs.push(rhs);
            AstExpressionBody::MethodCall {
                receiver_expr,
                method_name: method_name.append("="),
                arg_exprs,
                type_args: vec![],
                has_block: false,
                may_have_paren_wo_args: false,
            }
        }
        _ => panic!("[BUG] unexpectd lhs: {:?}", lhs.body),
    };
    non_primary_expression(body)
}

pub fn lvar_decl(name: String, rhs: AstExpression) -> AstExpression {
    non_primary_expression(AstExpressionBody::LVarAssign {
        name,
        rhs: Box::new(rhs),
        is_var: true,
    })
}

pub fn ivar_decl(name: String, rhs: AstExpression) -> AstExpression {
    non_primary_expression(AstExpressionBody::IVarAssign {
        name,
        rhs: Box::new(rhs),
        is_var: true,
    })
}

pub fn ivar_assign(name: String, rhs: AstExpression) -> AstExpression {
    non_primary_expression(AstExpressionBody::IVarAssign {
        name,
        rhs: Box::new(rhs),
        is_var: false,
    })
}

pub fn method_call(
    receiver_expr: Option<AstExpression>,
    method_name: &str,
    arg_exprs: Vec<AstExpression>,
    type_args: Vec<AstExpression>,
    primary: bool,
    has_block: bool,
    may_have_paren_wo_args: bool,
) -> AstExpression {
    AstExpression {
        primary,
        body: AstExpressionBody::MethodCall {
            receiver_expr: receiver_expr.map(Box::new),
            method_name: method_firstname(method_name),
            arg_exprs,
            type_args,
            has_block,
            may_have_paren_wo_args,
        },
        locs: LocationSpan::todo(),
    }
}

pub fn bare_name(name: &str) -> AstExpression {
    primary_expression(AstExpressionBody::BareName(name.to_string()))
}

pub fn unary_expr(expr: AstExpression, op: &str) -> AstExpression {
    primary_expression(AstExpressionBody::MethodCall {
        receiver_expr: Some(Box::new(expr)),
        method_name: method_firstname(op),
        arg_exprs: vec![],
        type_args: vec![],
        has_block: false,
        may_have_paren_wo_args: false,
    })
}

pub fn bin_op_expr(left: AstExpression, op: &str, right: AstExpression) -> AstExpression {
    non_primary_expression(AstExpressionBody::MethodCall {
        receiver_expr: Some(Box::new(left)),
        method_name: method_firstname(op),
        arg_exprs: vec![right],
        type_args: vec![],
        has_block: false,
        may_have_paren_wo_args: false,
    })
}

pub fn lambda_expr(
    params: Vec<BlockParam>,
    exprs: Vec<AstExpression>,
    is_fn: bool,
) -> AstExpression {
    primary_expression(AstExpressionBody::LambdaExpr {
        params,
        exprs,
        is_fn,
    })
}

pub fn primary_expression(body: AstExpressionBody) -> AstExpression {
    AstExpression {
        primary: true,
        body,
        locs: LocationSpan::todo(),
    }
}

pub fn non_primary_expression(body: AstExpressionBody) -> AstExpression {
    AstExpression {
        primary: false,
        body,
        locs: LocationSpan::todo(),
    }
}

/// Extend `foo.bar` to `foo.bar args`
/// (expr must be a MethodCall or a BareName)
pub fn set_method_call_args(
    expr: AstExpression,
    args: Vec<AstExpression>,
    has_block: bool,
) -> AstExpression {
    match expr.body {
        AstExpressionBody::MethodCall {
            receiver_expr,
            method_name,
            arg_exprs,
            type_args,
            ..
        } => {
            if !arg_exprs.is_empty() {
                panic!(
                    "[BUG] cannot extend because arg_exprs is not empty: {:?}",
                    arg_exprs
                );
            }

            AstExpression {
                primary: false,
                body: AstExpressionBody::MethodCall {
                    receiver_expr,
                    method_name,
                    arg_exprs: args,
                    type_args,
                    has_block,
                    may_have_paren_wo_args: false,
                },
                locs: LocationSpan::todo(),
            }
        }
        AstExpressionBody::BareName(s) => AstExpression {
            primary: false,
            body: AstExpressionBody::MethodCall {
                receiver_expr: None,
                method_name: method_firstname(s),
                arg_exprs: args,
                type_args: vec![],
                has_block,
                may_have_paren_wo_args: false,
            },
            locs: LocationSpan::todo(),
        },
        b => panic!("[BUG] `extend' takes a MethodCall but got {:?}", b),
    }
}
