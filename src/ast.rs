use crate::names::*;
use crate::parser::token::Token;

#[derive(Debug, PartialEq)]
pub struct Program {
    pub toplevel_defs: Vec<Definition>,
    pub exprs: Vec<Expression>,
}

#[derive(Debug, PartialEq)]
pub enum Definition {
    ClassDefinition {
        name: ClassFirstname,
        defs: Vec<Definition>,
    },
    InitializerDefinition {
        sig: InitializerSig,
        body_exprs: Vec<Expression>,
    },
    InstanceMethodDefinition {
        sig: AstMethodSignature,
        body_exprs: Vec<Expression>,
    },
    ClassMethodDefinition {
        sig: AstMethodSignature,
        body_exprs: Vec<Expression>,
    },
    ConstDefinition {
        name: ConstFirstname,
        expr: Expression,
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

#[derive(Debug, PartialEq)]
pub struct Expression {
    pub body: ExpressionBody,
    pub primary: bool,
}

#[derive(Debug, PartialEq)]
pub enum ExpressionBody {
    LogicalNot {
        expr: Box<Expression>,
    },
    LogicalAnd {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    LogicalOr {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    If {
        cond_expr: Box<Expression>,
        then_expr: Box<Expression>,
        else_expr: Option<Box<Expression>> // Box is needed to aboid E0072
    },
    LVarAssign {
        name: String,
        rhs: Box<Expression>,
    },
    ConstAssign {
        name: String,
        rhs: Box<Expression>,
    },
    MethodCall {
        receiver_expr: Option<Box<Expression>>, // Box is needed to aboid E0072
        method_name: MethodFirstname,
        arg_exprs: Vec<Expression>,
        may_have_paren_wo_args: bool,
    },
    // Local variable reference or method call with implicit receiver(self)
    BareName(String),
    ConstRef(String),
    PseudoVariable(Token),
    FloatLiteral {
        value: f64,
    },
    DecimalLiteral {
        value: i32,
    }
}

impl Expression {
    pub fn may_have_paren_wo_args(&self) -> bool {
        match self.body {
            ExpressionBody::MethodCall { may_have_paren_wo_args, .. } => may_have_paren_wo_args,
            ExpressionBody::BareName(_) => true,
            _ => false,
        }
    }

    /// True if this can be the left hand side of an assignment
    pub fn is_lhs(&self) -> bool {
        if self.may_have_paren_wo_args() {
            return true;
        }
        match self.body {
            ExpressionBody::ConstRef(_) => true,
            // TODO: a[b] / A::B
            _ => false
        }
    }
}

pub fn logical_not(expr: Expression) -> Expression {
    non_primary_expression(
        ExpressionBody::LogicalNot {
            expr: Box::new(expr),
        }
    )
}

pub fn logical_and(left: Expression, right: Expression) -> Expression {
    non_primary_expression(
        ExpressionBody::LogicalAnd {
            left: Box::new(left),
            right: Box::new(right),
        }
    )
}

pub fn logical_or(left: Expression, right: Expression) -> Expression {
    non_primary_expression(
        ExpressionBody::LogicalOr {
            left: Box::new(left),
            right: Box::new(right),
        }
    )
}

pub fn if_expr(cond_expr: Expression, then_expr: Expression, else_expr: Option<Expression>) -> Expression {
    non_primary_expression(
        ExpressionBody::If {
            cond_expr: Box::new(cond_expr),
            then_expr: Box::new(then_expr),
            else_expr: else_expr.map(|e| Box::new(e)),
        }
    )
}

/// Create an expression for an assigment
pub fn assignment(lhs: Expression, rhs: Expression) -> Expression {
    let body = match lhs.body {
        ExpressionBody::BareName(s) =>  {
            ExpressionBody::LVarAssign { name: s.to_string(), rhs: Box::new(rhs) } 
        },
        // ToDo: IVarRef =>
        // ToDo: CVarRef =>
        ExpressionBody::ConstRef(s) => {
            ExpressionBody::ConstAssign { name: s.to_string(), rhs: Box::new(rhs) }
        },
        ExpressionBody::MethodCall { receiver_expr, method_name, arg_exprs, .. } => {
            ExpressionBody::MethodCall {
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

pub fn method_call(receiver_expr: Option<Expression>,
                   method_name: &str,
                   arg_exprs: Vec<Expression>,
                   primary: bool,
                   may_have_paren_wo_args: bool) -> Expression {
    Expression {
        primary: primary,
        body: ExpressionBody::MethodCall {
            receiver_expr: receiver_expr.map(|e| Box::new(e)),
            method_name: MethodFirstname(method_name.to_string()),
            arg_exprs,
            may_have_paren_wo_args,
        }
    }
}

pub fn bare_name(name: &str) -> Expression {
    primary_expression(ExpressionBody::BareName(name.to_string()))
}

pub fn const_ref(name: &str) -> Expression {
    primary_expression(ExpressionBody::ConstRef(name.to_string()))
}

pub fn unary_expr(expr: Expression, op: &str) -> Expression {
    primary_expression(ExpressionBody::MethodCall {
        receiver_expr: Some(Box::new(expr)),
        method_name: MethodFirstname(op.to_string()),
        arg_exprs: vec![],
        may_have_paren_wo_args: false,
    })
}

pub fn bin_op_expr(left: Expression, op: &str, right: Expression) -> Expression {
    non_primary_expression(ExpressionBody::MethodCall {
        receiver_expr: Some(Box::new(left)),
        method_name: MethodFirstname(op.to_string()),
        arg_exprs: vec![right],
        may_have_paren_wo_args: false,
    })
}

pub fn pseudo_variable(token: Token) -> Expression {
    primary_expression(ExpressionBody::PseudoVariable(token))
}

pub fn float_literal(value: f64) -> Expression {
    primary_expression(ExpressionBody::FloatLiteral{ value })
}

pub fn decimal_literal(value: i32) -> Expression {
    primary_expression(ExpressionBody::DecimalLiteral{ value })
}

pub fn primary_expression(body: ExpressionBody) -> Expression {
    Expression { primary: true, body: body }
}

pub fn non_primary_expression(body: ExpressionBody) -> Expression {
    Expression { primary: false, body: body }
}

/// Extend `foo.bar` to `foo.bar args`
/// (expr must be a MethodCall or a BareName)
pub fn set_method_call_args(expr: Expression, args: Vec<Expression>) -> Expression {
    match expr.body {
        ExpressionBody::MethodCall { receiver_expr, method_name, arg_exprs, .. } => {
            if !arg_exprs.is_empty() {
                panic!("[BUG] cannot extend because arg_exprs is not empty: {:?}", arg_exprs);
            }

            Expression {
                primary: false,
                body: ExpressionBody::MethodCall {
                    receiver_expr,
                    method_name,
                    arg_exprs: args,
                    may_have_paren_wo_args: false,
                }
            }
        },
        ExpressionBody::BareName(s) => {
            Expression {
                primary: false,
                body: ExpressionBody::MethodCall {
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
