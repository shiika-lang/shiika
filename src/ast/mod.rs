use crate::names::*;

#[derive(Debug, PartialEq)]
pub struct Program {
    pub toplevel_defs: Vec<Definition>,
    pub exprs: Vec<Expression>,
}

#[derive(Debug, PartialEq)]
pub enum Definition {
    ClassDefinition {
        name: ClassName,
        defs: Vec<Definition>,
    },
    InitializerDefinition {
        name: String,
        body_exprs: Vec<Expression>,
    },
    InstanceMethodDefinition {
        sig: MethodSignature,
        body_exprs: Vec<Expression>,
    },
    ClassMethodDefinition {
        sig: MethodSignature,
        body_exprs: Vec<Expression>,
    }
}

#[derive(Debug, PartialEq)]
pub struct MethodSignature {
    pub name: MethodName,
    pub params: Vec<Param>,
    pub ret_typ: Typ,
}

#[derive(Debug, PartialEq)]
pub struct Param {
    pub name: String,
    pub typ: Typ,
}

#[derive(Debug, PartialEq)]
pub struct Typ {
    pub name: String,
}

#[derive(Debug, PartialEq)]
pub enum Expression {
    If {
        cond_expr: Box<Expression>,
        then_expr: Box<Expression>,
        else_expr: Option<Box<Expression>>
    },
    MethodCall {
        receiver_expr: Option<Box<Expression>>,
        method_name: MethodName,
        arg_exprs: Vec<Expression>
    },
    BinOpExpression {
        left: Box<Expression>,
        op: BinOp,
        right: Box<Expression>
    },
    // Local variable reference or method call with implicit receiver(self)
    BareName(String),
    FloatLiteral {
        value: f32,
    },
    DecimalLiteral {
        value: i32,
    }
}

pub fn bin_op_expr(left: Expression, op: BinOp, right: Expression) -> Expression {
    Expression::BinOpExpression {
        left: Box::new(left),
        op: op,
        right: Box::new(right)
    }
}

pub fn float_literal(value: f32) -> Expression {
    Expression::FloatLiteral{ value }
}

pub fn decimal_literal(value: i32) -> Expression {
    Expression::DecimalLiteral{ value }
}

#[derive(Debug, PartialEq, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}
impl BinOp {
    pub fn method_name(&self) -> MethodName {
        MethodName((match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Mod => "%",
        }).to_string())
    }
}
