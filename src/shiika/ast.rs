#[derive(Debug, PartialEq)]
pub struct Program {
    pub expr: Expression
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
    Name(String), // Local variable reference or method call with implicit receiver(self)
    BinOp {
        left: Box<Expression>,
        op: BinOp,
        right: Box<Expression>
    },
    DecimalLiteral{ value: usize },
}

pub fn bin_op_expr(left: Expression, op: BinOp, right: Expression) -> Expression {
    Expression::BinOp{
        left: Box::new(left),
        op: op,
        right: Box::new(right)
    }
}

pub fn decimal_literal(value: usize) -> Expression {
    Expression::DecimalLiteral{ value }
}

#[derive(Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}
