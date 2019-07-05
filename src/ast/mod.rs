#[derive(Debug, PartialEq)]
pub struct Program {
    pub class_defs: Vec<Definition>,
    pub stmts: Vec<Statement>,
}

#[derive(Debug, PartialEq)]
pub enum Definition {
    ClassDefinition {
        name: String,
        defs: Vec<Definition>,
    },
    InitializerDefinition {
        name: String,
        body_stmts: Vec<Statement>,
    },
    InstanceMethodDefinition {
        name: String,
        params: Vec<Param>,
        body_stmts: Vec<Statement>,
    },
    ClassMethodDefinition {
        name: String,
        body_stmts: Vec<Statement>,
    }
}

#[derive(Debug, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Ty,
}

#[derive(Debug, PartialEq)]
pub struct Ty {
    pub name: String,
}

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

#[derive(Debug, PartialEq)]
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
    BinOpExpression {
        left: Box<Expression>,
        op: BinOp,
        right: Box<Expression>
    },
    FloatLiteral {
        value: f32,
    },
    DecimalLiteral {
        value: i32,
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
    pub fn method_name(&self) -> String {
        (match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Mod => "%",
        }).to_string()
    }
}
