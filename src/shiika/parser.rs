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
        self.parse_bin_op(chars)
    }

    fn parse_bin_op(&self, chars: &mut Peekable<Chars>) -> ast::Expression {
        let left = Box::new(self.parse_decimal_literal(chars));
        chars.next();
        let right = Box::new(self.parse_decimal_literal(chars));
        ast::Expression::BinOp{ left: left, right: right }
    }

    fn parse_decimal_literal(&self, chars: &mut Peekable<Chars>) -> ast::Expression {
        let mut num_str = String::new();
        loop {
            let item = chars.peek();
            if item == None { break }
            match item.unwrap() {
                '0'...'9' => num_str.push(chars.next().unwrap()),
                _ => break
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
    let result = parse("12+3");
    match result.expr {
        ast::Expression::BinOp {left, right} => {
            match *left {
                ast::Expression::DecimalLiteral {value} => {
                    assert_eq!(value, 12);
                },
                _ => panic!()
            }
            match *right {
                ast::Expression::DecimalLiteral {value} => {
                    assert_eq!(value, 3);
                },
                _ => panic!()
            }
        },
        _ => panic!()
    }
}
