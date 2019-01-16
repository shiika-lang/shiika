use crate::shiika::ast;
use std::str::Chars;
use std::iter::Peekable;
//use std::borrow::Cow;

pub struct Parser {
    pub src: String
}

impl Parser {
    fn parse(&self) -> ast::Program {
        let mut chars = self.src.chars().peekable();
        ast::Program {
            expr: self.parse_expr(&mut chars)
        }
    }

    fn parse_expr(&self, chars: &mut Peekable<Chars>) -> ast::Expression {
        ast::Expression::BinOp{
            left: Box::new(self.parse_decimal_literal(chars)),
            right: Box::new(self.parse_decimal_literal(chars)),
        }
    }

    fn parse_decimal_literal(&self, chars: &mut Peekable<Chars>) -> ast::Expression {
        let mut num_str = String::new();
        loop {
            if let Some(c) = chars.peek() {
                match *c {
                    '0'...'9' => {
                        let tmp = *c;
                        chars.next();
                        num_str.push(tmp)
                    }
                    _ => break
                }
            }
            else {
                break
            }
        }
        ast::Expression::DecimalLiteral{
            value: num_str.parse().unwrap()
        }
    }
}

pub fn parse(src: &str) -> ast::Program {
    let parser = Parser {
        src: src.to_string(),
    };
    parser.parse()
}

#[test]
fn test_parser() {
    let result = parse("1+2");
    match result.expr {
        ast::Expression::BinOp {left, right} => {
            match *left {
                ast::Expression::DecimalLiteral {value} => {
                    assert_eq!(value, 1);
                },
                _ => panic!()
            }
            match *right {
                ast::Expression::DecimalLiteral {value} => {
                    assert_eq!(value, 2);
                },
                _ => panic!()
            }
        },
        _ => panic!()
    }
}
