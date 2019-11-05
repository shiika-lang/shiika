use crate::names::*;
use crate::parser::token::Token;

#[derive(Debug, PartialEq)]
pub struct Program {
    pub toplevel_defs: Vec<Definition>,
    pub exprs: Vec<AstExpression>,
}

#[derive(Debug, PartialEq)]
pub enum Definition {
    ClassDefinition {
        name: ClassFirstname,
        defs: Vec<Definition>,
    },
    InitializerDefinition {
        sig: InitializerSig,
        body_exprs: Vec<AstExpression>,
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
        name: ConstFirstname,
        expr: AstExpression,
    }
}

#[derive(Debug, PartialEq)]
pub struct AstMethodSignature {
    pub name: MethodFirstname,
    pub params: Vec<Param>,
    pub ret_typ: Typ,
}

#[derive(Debug, PartialEq)]
pub struct InitializerSig {
    pub params: Vec<IParam>,
    pub ret_typ: Typ,
}

#[derive(Debug, PartialEq)]
pub struct Param {
    pub name: String,
    pub typ: Typ,
}

#[derive(Debug, PartialEq)]
pub struct IParam {
    pub name: String,
    pub typ: Typ,
}

#[derive(Debug, PartialEq)]
pub struct Typ {
    pub name: String,
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
        then_expr: Box<AstExpression>,
        else_expr: Option<Box<AstExpression>> // Box is needed to aboid E0072
    },
    LVarAssign {
        name: String,
        rhs: Box<AstExpression>,
    },
    ConstAssign {
        names: Vec<String>,
        rhs: Box<AstExpression>,
    },
    MethodCall {
        receiver_expr: Option<Box<AstExpression>>, // Box is needed to aboid E0072
        method_name: MethodFirstname,
        arg_exprs: Vec<AstExpression>,
        may_have_paren_wo_args: bool,
    },
    // Local variable reference or method call with implicit receiver(self)
    BareName(String),
    ConstRef(Vec<String>),
    PseudoVariable(Token),
    FloatLiteral {
        value: f64,
    },
    DecimalLiteral {
        value: i32,
    }
}

impl AstExpression {
    pub fn may_have_paren_wo_args(&self) -> bool {
        match self.body {
            AstExpressionBody::MethodCall { may_have_paren_wo_args, .. } => may_have_paren_wo_args,
            AstExpressionBody::BareName(_) => true,
            _ => false,
        }
    }

    /// True if this can be the left hand side of an assignment
    pub fn is_lhs(&self) -> bool {
        if self.may_have_paren_wo_args() {
            return true;
        }
        match self.body {
            AstExpressionBody::ConstRef(_) => true,
            // TODO: a[b]
            _ => false
        }
    }
}

pub fn logical_not(expr: AstExpression) -> AstExpression {
    non_primary_expression(
        AstExpressionBody::LogicalNot {
            expr: Box::new(expr),
        }
    )
}

pub fn logical_and(left: AstExpression, right: AstExpression) -> AstExpression {
    non_primary_expression(
        AstExpressionBody::LogicalAnd {
            left: Box::new(left),
            right: Box::new(right),
        }
    )
}

pub fn logical_or(left: AstExpression, right: AstExpression) -> AstExpression {
    non_primary_expression(
        AstExpressionBody::LogicalOr {
            left: Box::new(left),
            right: Box::new(right),
        }
    )
}

pub fn if_expr(cond_expr: AstExpression, then_expr: AstExpression, else_expr: Option<AstExpression>) -> AstExpression {
    non_primary_expression(
        AstExpressionBody::If {
            cond_expr: Box::new(cond_expr),
            then_expr: Box::new(then_expr),
            else_expr: else_expr.map(|e| Box::new(e)),
        }
    )
}

/// Create an expression for an assigment
pub fn assignment(lhs: AstExpression, rhs: AstExpression) -> AstExpression {
    let body = match lhs.body {
        AstExpressionBody::BareName(s) =>  {
            AstExpressionBody::LVarAssign { name: s.to_string(), rhs: Box::new(rhs) } 
        },
        // ToDo: IVarRef =>
        // ToDo: CVarRef =>
        AstExpressionBody::ConstRef(names) => {
            AstExpressionBody::ConstAssign { names: names, rhs: Box::new(rhs) }
        },
        AstExpressionBody::MethodCall { receiver_expr, method_name, arg_exprs, .. } => {
            AstExpressionBody::MethodCall {
                receiver_expr,
                method_name: method_name.append("="),
                arg_exprs,
                may_have_paren_wo_args: false,
            }
        },
        _ => panic!("[BUG] unexpectd lhs: {:?}", lhs.body)
    };
    non_primary_expression(body)
}

pub fn method_call(receiver_expr: Option<AstExpression>,
                   method_name: &str,
                   arg_exprs: Vec<AstExpression>,
                   primary: bool,
                   may_have_paren_wo_args: bool) -> AstExpression {
    AstExpression {
        primary: primary,
        body: AstExpressionBody::MethodCall {
            receiver_expr: receiver_expr.map(|e| Box::new(e)),
            method_name: MethodFirstname(method_name.to_string()),
            arg_exprs,
            may_have_paren_wo_args,
        }
    }
}

pub fn bare_name(name: &str) -> AstExpression {
    primary_expression(AstExpressionBody::BareName(name.to_string()))
}

pub fn const_ref(names: Vec<String>) -> AstExpression {
    primary_expression(AstExpressionBody::ConstRef(names))
}

pub fn unary_expr(expr: AstExpression, op: &str) -> AstExpression {
    primary_expression(AstExpressionBody::MethodCall {
        receiver_expr: Some(Box::new(expr)),
        method_name: MethodFirstname(op.to_string()),
        arg_exprs: vec![],
        may_have_paren_wo_args: false,
    })
}

pub fn bin_op_expr(left: AstExpression, op: &str, right: AstExpression) -> AstExpression {
    non_primary_expression(AstExpressionBody::MethodCall {
        receiver_expr: Some(Box::new(left)),
        method_name: MethodFirstname(op.to_string()),
        arg_exprs: vec![right],
        may_have_paren_wo_args: false,
    })
}

pub fn pseudo_variable(token: Token) -> AstExpression {
    primary_expression(AstExpressionBody::PseudoVariable(token))
}

pub fn float_literal(value: f64) -> AstExpression {
    primary_expression(AstExpressionBody::FloatLiteral{ value })
}

pub fn decimal_literal(value: i32) -> AstExpression {
    primary_expression(AstExpressionBody::DecimalLiteral{ value })
}

pub fn primary_expression(body: AstExpressionBody) -> AstExpression {
    AstExpression { primary: true, body: body }
}

pub fn non_primary_expression(body: AstExpressionBody) -> AstExpression {
    AstExpression { primary: false, body: body }
}

/// Extend `foo.bar` to `foo.bar args`
/// (expr must be a MethodCall or a BareName)
pub fn set_method_call_args(expr: AstExpression, args: Vec<AstExpression>) -> AstExpression {
    match expr.body {
        AstExpressionBody::MethodCall { receiver_expr, method_name, arg_exprs, .. } => {
            if !arg_exprs.is_empty() {
                panic!("[BUG] cannot extend because arg_exprs is not empty: {:?}", arg_exprs);
            }

            AstExpression {
                primary: false,
                body: AstExpressionBody::MethodCall {
                    receiver_expr,
                    method_name,
                    arg_exprs: args,
                    may_have_paren_wo_args: false,
                }
            }
        },
        AstExpressionBody::BareName(s) => {
            AstExpression {
                primary: false,
                body: AstExpressionBody::MethodCall {
                    receiver_expr: None,
                    method_name: MethodFirstname(s.to_string()),
                    arg_exprs: args,
                    may_have_paren_wo_args: false,
                }
            }
        },
        b => panic!("[BUG] `extend' takes a MethodCall but got {:?}", b)
    }
}
