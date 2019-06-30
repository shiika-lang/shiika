mod to_hir;

#[derive(Debug, PartialEq)]
pub struct Program {
    pub class_defs: Vec<ClassDefinition>,
    pub expr: Expression
}

pub trait Definition {}

#[derive(Debug, PartialEq)]
pub struct ClassDefinition {
    pub name: String,
    pub instance_method_defs: Vec<InstanceMethodDefinition>,
}
impl Definition for ClassDefinition {}

#[derive(Debug, PartialEq)]
pub struct InstanceMethodDefinition {
    pub name: String,
    pub body_stmts: Vec<Statement>,
}
impl Definition for InstanceMethodDefinition {}

#[derive(Debug, PartialEq)]
pub enum Statement {
//    WhileStatement {
//        cond_expr: Box<Expression>,
//        body_stmts: Vec<Statement>,
//    },
    ExpressionStatement {
        expr: Box<Expression>,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    If {
        cond_expr: Box<Expression>,
        then_expr: Box<Expression>,
        else_expr: Option<Box<Expression>>
    },
    MethodCall {
        receiver_expr: Option<Box<Expression>>,
        method_name: String,
        arg_exprs: Vec<Expression>
    },
    // Local variable reference or method call with implicit receiver(self)
    Name(String),
    BinOp {
        left: Box<Expression>,
        op: BinOp,
        right: Box<Expression>
    },
    FloatLiteral {
        value: f32,
    },
    DecimalLiteral {
        value: usize,
    }
}

impl Expression {
    pub fn to_statement(self) -> Statement
    {
        Statement::ExpressionStatement {
            expr: Box::new(self)
        }
    }
}

pub fn bin_op_expr(left: Expression, op: BinOp, right: Expression) -> Expression {
    Expression::BinOp{
        left: Box::new(left),
        op: op,
        right: Box::new(right)
    }
}

pub fn float_literal(value: f32) -> Expression {
    Expression::FloatLiteral{ value }
}

pub fn decimal_literal(value: usize) -> Expression {
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
