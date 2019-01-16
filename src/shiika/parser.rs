use crate::shiika::ast;
use std::str::Chars;
use std::iter::Peekable;

pub struct Parser {
    pub src: String,
    pub loc: Location
}

pub struct Location {
    pub file: String,
    pub line: usize,
    pub col: usize
}

impl Location {
    fn new() -> Location {
        Location {
            file: "".to_string(),
            line: 0,
            col: 0
        }
    }
}

#[derive(Debug)]
pub struct ParseError {
    pub msg: String
}

impl Parser {
    fn parse(&self) -> Result<ast::Program, ParseError> {
        let mut chars = self.src.chars().peekable();
        Ok(ast::Program {
            expr: self.parse_expr(&mut chars)?
        })
    }

    fn parse_expr(&self, chars: &mut Peekable<Chars>) -> Result<ast::Expression, ParseError> {
        self.parse_bin_op(chars)
    }

    fn parse_bin_op(&self, chars: &mut Peekable<Chars>) -> Result<ast::Expression, ParseError> {
        let left = Box::new(self.parse_decimal_literal(chars)?);

        let item = chars.next();
        if item == None || item.unwrap() != '+' {
            return Err(parseerror("expected +"))
        }

        let right = Box::new(self.parse_decimal_literal(chars)?);
        Ok(ast::Expression::BinOp{ left: left, right: right })
    }

    fn parse_decimal_literal(&self, chars: &mut Peekable<Chars>) -> Result<ast::Expression, ParseError> {
        let mut num_str = String::new();
        loop {
            let item = chars.peek();
            if item == None { break }
            match item.unwrap() {
                '0'...'9' => num_str.push(chars.next().unwrap()),
                _ => break
            }
        }
        if num_str.is_empty() {
            Err(parseerror("expected decimal literal"))
        }
        else {
            Ok(ast::Expression::DecimalLiteral{
                value: num_str.parse().unwrap()
            })
        }
    }
}

fn parseerror(msg: &str) -> ParseError {
    ParseError{ msg: msg.to_string() }
}

pub fn parse(src: &str) -> Result<ast::Program, ParseError> {
    let parser = Parser {
        src: src.to_string(),
        loc: Location::new(),
    };
    parser.parse()
}

#[test]
fn test_parser() {
    let result = parse("12+3");
    match result.unwrap().expr {
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
