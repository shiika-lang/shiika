#[derive(Debug, PartialEq)]
pub struct Program {
    pub expr: Expression
}

#[derive(Debug, PartialEq)]
pub enum Expression {
    BinOp {
        left: Box<Expression>,
        op: BinOp,
        right: Box<Expression>
    },
    DecimalLiteral{ value: i32 },
}

impl Expression {
    pub fn bin_op_expr(left: Expression, op: BinOp, right: Expression) -> Expression {
        Expression::BinOp{
            left: Box::new(left),
            op: op,
            right: Box::new(right)
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}
