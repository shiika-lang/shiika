mod location;
mod source;
use super::ast;
use super::parser::source::Source;
use super::parser::location::Location;

pub struct Parser {
    pub source: Source
}

#[derive(Debug)]
pub struct ParseError {
    pub msg: String,
    pub location: Location
}

impl Parser {
    fn parse(&mut self) -> Result<ast::Program, ParseError> {
        Ok(ast::Program {
            expr: self.parse_expr()?
        })
    }

    fn parse_expr(&mut self) -> Result<ast::Expression, ParseError> {
        self.parse_bin_op()
    }

    fn parse_bin_op(&mut self) -> Result<ast::Expression, ParseError> {
        let left = Box::new(self.parse_decimal_literal()?);

        let item = self.source.next();
        if item == None || item.unwrap() != '+' {
            return Err(self.parseerror("expected +"))
        }

        let right = Box::new(self.parse_decimal_literal()?);
        Ok(ast::Expression::BinOp{ left: left, right: right })
    }

    fn parse_decimal_literal(&mut self) -> Result<ast::Expression, ParseError> {
        let mut num_str = String::new();
        loop {
            let item = self.source.peek();
            if item == None { break }
            match item.unwrap() {
                '0'...'9' => num_str.push(self.source.next().unwrap()),
                _ => break
            }
        }
        if num_str.is_empty() {
            Err(self.parseerror("expected decimal literal"))
        }
        else {
            Ok(ast::Expression::DecimalLiteral{
                value: num_str.parse().unwrap()
            })
        }
    }

    fn parseerror(&self, msg: &str) -> ParseError {
        ParseError{
            msg: msg.to_string(),
            location: self.source.location.clone()
        }
    }
}

pub fn parse(src: &str) -> Result<ast::Program, ParseError> {
    let mut parser = Parser {
        source: Source::dummy(src)
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
