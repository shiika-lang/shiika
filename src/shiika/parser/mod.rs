mod location;
mod source;

extern crate backtrace;
use backtrace::Backtrace;
use super::ast;
use super::parser::source::Source;
use super::parser::location::Location;

pub struct Parser {
    pub source: Source
}

#[derive(Debug)]
pub struct ParseError {
    pub msg: String,
    pub location: Location,
    pub backtrace: Backtrace
}

impl Parser {
    fn parse(&mut self) -> Result<ast::Program, ParseError> {
        Ok(ast::Program {
            expr: self.parse_expr()?
        })
    }

    fn parse_expr(&mut self) -> Result<ast::Expression, ParseError> {
        self.parse_additive_expr()
    }

    fn parse_additive_expr(&mut self) -> Result<ast::Expression, ParseError> {
        let left = self.parse_multiplicative_expr()?;
        self.source.skip_ws();

        let c = self.source.peek();
        match c {
            Some('+') | Some('-') => {
                let op = if c == Some('+') { ast::BinOp::Add }
                         else { ast::BinOp::Sub };
                self.source.next();
                self.source.skip_ws();
                let right = self.parse_expr()?;
                Ok(ast::Expression::bin_op_expr(left, op, right))
            },
            _ => Ok(left)
        }
    }

    fn parse_multiplicative_expr(&mut self) -> Result<ast::Expression, ParseError> {
        let left = self.parse_parenthesized_expr()?;
        self.source.skip_ws();

        let c = self.source.peek();
        match c {
            Some('*') | Some('/') | Some('%') => {
                let op = if c == Some('*') { ast::BinOp::Mul }
                         else if c == Some('/') { ast::BinOp::Div }
                         else { ast::BinOp::Mod };
                self.source.next();
                self.source.skip_ws();
                let right = self.parse_multiplicative_expr()?;
                Ok(ast::Expression::bin_op_expr(left, op, right))
            },
            _ => Ok(left)
        }
    }

    fn parse_parenthesized_expr(&mut self) -> Result<ast::Expression, ParseError> {
        if self.source.peek_char()? != '(' {
            return self.parse_decimal_literal();
        }
        self.source.next();
        self.source.skip_ws();
        let expr = self.parse_expr()?;
        if self.source.next_char()? != ')' {
            return Err(self.parseerror("missing `)'"))
        }
        Ok(expr)
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
            location: self.source.location.clone(),
            backtrace: Backtrace::new()
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
    let result = parse("1+2*3");
    println!("{:#?}", result);
    assert_eq!(result.unwrap(), 
      ast::Program {
        expr: ast::Expression::BinOp {
                left: Box::new(ast::Expression::DecimalLiteral {value: 1}),
                op: ast::BinOp::Add,
                right: Box::new(ast::Expression::BinOp {
                    left: Box::new(ast::Expression::DecimalLiteral {value: 2}),
                    op: ast::BinOp::Mul,
                    right: Box::new(ast::Expression::DecimalLiteral {value: 3}),
                })
        }
      }
    )
}
