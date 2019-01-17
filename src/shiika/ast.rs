#[derive(Debug)]
pub struct Program {
    pub expr: Expression
}

#[derive(Debug)]
pub enum Expression {
    BinOp {
        left: Box<Expression>,
        //pub op: TODO
        right: Box<Expression>
    },
    DecimalLiteral{ value: i32 },
}

//pub struct DecimalLiteral {
//    pub value: i32
//}
