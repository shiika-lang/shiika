use crate::names::*;
use crate::parser::token::Token;

#[derive(Debug, PartialEq)]
pub struct Program {
    pub toplevel_items: Vec<TopLevelItem>,
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
        typarams: Vec<String>,
        superclass: Option<ConstName>,
        defs: Vec<Definition>,
    },
    EnumDefinition {
        name: ClassFirstname,
        typarams: Vec<String>,
        cases: Vec<EnumCase>,
    },
    InstanceMethodDefinition {
        sig: AstMethodSignature,
        body_exprs: Vec<AstExpression>,
    },
    ClassMethodDefinition {
        sig: AstMethodSignature,
        body_exprs: Vec<AstExpression>,
    },
    ConstDefinition {
        name: String,
        expr: AstExpression,
    },
}

#[derive(Debug, PartialEq)]
pub struct EnumCase {
    pub name: ClassFirstname,
    pub typarams: Vec<String>,
    pub params: Vec<Param>,
}

#[derive(Debug, PartialEq)]
pub struct AstMethodSignature {
    pub name: MethodFirstname,
    pub typarams: Vec<String>,
    pub params: Vec<Param>,
    pub ret_typ: Typ,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Param {
    pub name: String,
    pub typ: Typ,
    pub is_iparam: bool, // eg. `def initialize(@a: Int)`
}

#[derive(Debug, PartialEq, Clone)]
pub struct Typ {
    pub name: String,
    pub typ_args: Vec<Typ>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AstExpression {
    pub body: AstExpressionBody,
    pub primary: bool,
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
        type_args: Vec<ConstName>,
        may_have_paren_wo_args: bool,
    },
    LambdaExpr {
        params: Vec<Param>,
        exprs: Vec<AstExpression>,
        /// true if this is from `fn(){}`. false if this is a block (do-end/{})
        is_fn: bool,
    },
    // Local variable reference or method call with implicit receiver(self)
    BareName(String),
    IVarRef(String),
    ConstRef(ConstName),
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

impl Definition {
    pub fn is_initializer(&self) -> bool {
        match self {
            Definition::InstanceMethodDefinition { sig, .. } => sig.name.0 == "initialize",
            _ => false,
        }
    }
}

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
            AstExpressionBody::ConstRef(_) => true,
            AstExpressionBody::MethodCall { method_name, .. } => method_name.0 == "[]",
            _ => false,
        }
    }
}

pub fn logical_not(expr: AstExpression) -> AstExpression {
    non_primary_expression(AstExpressionBody::LogicalNot {
        expr: Box::new(expr),
    })
}

pub fn logical_and(left: AstExpression, right: AstExpression) -> AstExpression {
    non_primary_expression(AstExpressionBody::LogicalAnd {
        left: Box::new(left),
        right: Box::new(right),
    })
}

pub fn logical_or(left: AstExpression, right: AstExpression) -> AstExpression {
    non_primary_expression(AstExpressionBody::LogicalOr {
        left: Box::new(left),
        right: Box::new(right),
    })
}

pub fn if_expr(
    cond_expr: AstExpression,
    then_exprs: Vec<AstExpression>,
    else_exprs: Option<Vec<AstExpression>>,
) -> AstExpression {
    non_primary_expression(AstExpressionBody::If {
        cond_expr: Box::new(cond_expr),
        then_exprs,
        else_exprs,
    })
}

pub fn while_expr(cond_expr: AstExpression, body_exprs: Vec<AstExpression>) -> AstExpression {
    non_primary_expression(AstExpressionBody::While {
        cond_expr: Box::new(cond_expr),
        body_exprs,
    })
}

pub fn break_expr() -> AstExpression {
    non_primary_expression(AstExpressionBody::Break {})
}

pub fn return_expr(arg: Option<AstExpression>) -> AstExpression {
    non_primary_expression(AstExpressionBody::Return {
        arg: arg.map(Box::new),
    })
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
        AstExpressionBody::ConstRef(const_name) => AstExpressionBody::ConstAssign {
            names: const_name.names,
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
    type_args: Vec<ConstName>,
    primary: bool,
    may_have_paren_wo_args: bool,
) -> AstExpression {
    AstExpression {
        primary,
        body: AstExpressionBody::MethodCall {
            receiver_expr: receiver_expr.map(Box::new),
            method_name: method_firstname(method_name),
            arg_exprs,
            type_args,
            may_have_paren_wo_args,
        },
    }
}

pub fn bare_name(name: &str) -> AstExpression {
    primary_expression(AstExpressionBody::BareName(name.to_string()))
}

pub fn ivar_ref(name: String) -> AstExpression {
    primary_expression(AstExpressionBody::IVarRef(name))
}

pub fn const_ref(name: ConstName) -> AstExpression {
    primary_expression(AstExpressionBody::ConstRef(name))
}

pub fn unary_expr(expr: AstExpression, op: &str) -> AstExpression {
    primary_expression(AstExpressionBody::MethodCall {
        receiver_expr: Some(Box::new(expr)),
        method_name: method_firstname(op),
        arg_exprs: vec![],
        type_args: vec![],
        may_have_paren_wo_args: false,
    })
}

pub fn bin_op_expr(left: AstExpression, op: &str, right: AstExpression) -> AstExpression {
    non_primary_expression(AstExpressionBody::MethodCall {
        receiver_expr: Some(Box::new(left)),
        method_name: method_firstname(op),
        arg_exprs: vec![right],
        type_args: vec![],
        may_have_paren_wo_args: false,
    })
}

pub fn lambda_expr(params: Vec<Param>, exprs: Vec<AstExpression>, is_fn: bool) -> AstExpression {
    primary_expression(AstExpressionBody::LambdaExpr {
        params,
        exprs,
        is_fn,
    })
}

pub fn pseudo_variable(token: Token) -> AstExpression {
    primary_expression(AstExpressionBody::PseudoVariable(token))
}

pub fn array_literal(exprs: Vec<AstExpression>) -> AstExpression {
    primary_expression(AstExpressionBody::ArrayLiteral(exprs))
}

pub fn float_literal(value: f64) -> AstExpression {
    primary_expression(AstExpressionBody::FloatLiteral { value })
}

pub fn decimal_literal(value: i64) -> AstExpression {
    primary_expression(AstExpressionBody::DecimalLiteral { value })
}

pub fn string_literal(content: String) -> AstExpression {
    primary_expression(AstExpressionBody::StringLiteral { content })
}

pub fn primary_expression(body: AstExpressionBody) -> AstExpression {
    AstExpression {
        primary: true,
        body,
    }
}

pub fn non_primary_expression(body: AstExpressionBody) -> AstExpression {
    AstExpression {
        primary: false,
        body,
    }
}

/// Extend `foo.bar` to `foo.bar args`
/// (expr must be a MethodCall or a BareName)
pub fn set_method_call_args(expr: AstExpression, args: Vec<AstExpression>) -> AstExpression {
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
                    may_have_paren_wo_args: false,
                },
            }
        }
        AstExpressionBody::BareName(s) => AstExpression {
            primary: false,
            body: AstExpressionBody::MethodCall {
                receiver_expr: None,
                method_name: method_firstname(&s),
                arg_exprs: args,
                type_args: vec![],
                may_have_paren_wo_args: false,
            },
        },
        b => panic!("[BUG] `extend' takes a MethodCall but got {:?}", b),
    }
}
