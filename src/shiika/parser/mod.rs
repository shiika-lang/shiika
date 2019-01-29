mod lexer;

extern crate backtrace;
use backtrace::Backtrace;
use super::ast;
use super::parser::lexer::Lexer;
use super::parser::lexer::Token;

pub struct Parser<'a, 'b> {
    pub lexer: Lexer<'a, 'b>
}

#[derive(Debug)]
pub struct ParseError {
    pub msg: String,
    pub location: lexer::Cursor,
    pub backtrace: Backtrace
}

impl<'a, 'b> Parser<'a, 'b> {
    fn parse(&mut self) -> Result<ast::Program, ParseError> {
        Ok(ast::Program {
            expr: self.parse_expr()?
        })
    }

    fn parse_expr(&mut self) -> Result<ast::Expression, ParseError> {
        match self.lexer.current_token() {
            Token::Eof => Err(self.parseerror("unexpected EOF")),
            //Token::Word("if") => self.parse_if_expr(),
            //Some(Token::Number(s)) => self.parse_decimal_literal(s),
            _ => self.parse_additive_expr(),
        }
    }

//    fn parse_if_expr(&mut self) -> Result<ast::Expression, ParseError> {
//        assert_eq!(self.lexer.peek_token(), Token::Word("if"));
//
//        self.lexer.read_ascii("if");
//        self.source.skip_ws();
//        let cond_expr = Box::new(self.parse_expr()?);
//        self.source.require_sep()?;
//        self.source.skip_wsn();
//        let then_expr = Box::new(self.parse_expr()?);
//        self.source.skip_wsn();
//        if self.source.starts_with("else") {
//            self.source.read_ascii("else");
//            self.source.skip_wsn();
//            let else_expr = Some(Box::new(self.parse_expr()?));
//            Ok(ast::Expression::If { cond_expr, then_expr, else_expr })
//        }
//        else {
//            self.source.require_ascii("end")?;
//            let else_expr = None;
//            Ok(ast::Expression::If { cond_expr, then_expr, else_expr })
//        }
//    }
//
//    fn parse_method_call(&mut self) -> Result<ast::Expression, ParseError> {
//        let receiver_expr = self.parse_additive_expr()?;
//        if self.source.peek() == Some('.') {
//            self.source.next();
//            let method_name = self.source.require_ident()?;
//            self.source.require_ascii("(")?;
//            self.source.skip_wsn();
//            let arg_expr = 
//                if self.source.peek() == Some(')') {
//                    None
//                }
//                else {
//                    let tmp = self.parse_expr()?;
//                    self.source.skip_wsn();
//                    self.source.require_ascii(")")?;
//                    Some(Box::new(tmp))
//                };
//            Ok(ast::Expression::MethodCall {
//                receiver_expr: Box::new(receiver_expr),
//                method_name: method_name,
//                arg_expr: arg_expr,
//            })
//        }
//        else {
//            Ok(receiver_expr)
//        }
//    }

    fn parse_additive_expr(&mut self) -> Result<ast::Expression, ParseError> {
        let left = self.parse_decimal_literal()?;  // self.parse_multiplicative_expr()?;
        self.skip_ws();

        match self.lexer.current_token() {
            Token::Symbol(s @ "+") | Token::Symbol(s @ "-") => {
                let op = if *s == "+" { ast::BinOp::Add }
                         else { ast::BinOp::Sub };
                self.lexer.consume();
                self.skip_wsn();
                let right = self.parse_expr()?;
                Ok(ast::Expression::bin_op_expr(left, op, right))
            },
            _ => Ok(left)
        }
    }

    fn parse_multiplicative_expr(&mut self) -> Result<ast::Expression, ParseError> {
        let left = self.parse_parenthesized_expr()?;
        self.skip_ws();

        match self.lexer.current_token() {
            Token::Symbol(s @ "*") | Token::Symbol(s @ "/") | Token::Symbol(s @ "%") => {
                let op = if *s == "*" { ast::BinOp::Mul }
                         else if *s == "/" { ast::BinOp::Div }
                         else { ast::BinOp::Mod };
                self.lexer.consume();
                self.skip_wsn();
                let right = self.parse_multiplicative_expr()?;
                Ok(ast::Expression::bin_op_expr(left, op, right))
            },
            _ => Ok(left)
        }
    }

    fn parse_parenthesized_expr(&mut self) -> Result<ast::Expression, ParseError> {
        if *self.lexer.current_token() != Token::Symbol("(") {
            return self.parse_decimal_literal();
        }
        self.lexer.consume();
        self.skip_wsn();
        let expr = self.parse_expr()?;
        self.skip_wsn();
        self.expect(Token::Symbol(")"))?;
        Ok(expr)
    }

    fn parse_decimal_literal(&mut self) -> Result<ast::Expression, ParseError> {
        match self.lexer.current_token() {
            Token::Number(s) => {
                let value = s.parse().unwrap();
                self.lexer.consume();
                Ok(ast::Expression::DecimalLiteral{ value })
            },
            _ => {
                Err(self.parseerror("expected decimal literal"))
            }
        }
    }

    fn expect(&mut self, token: Token) -> Result<(), ParseError> {
        if *self.lexer.current_token() == token {
            Ok(())
        }
        else {
            let msg = format!("expected {:?} but got {:?}", token, self.lexer.current_token());
            Err(self.parseerror(&msg))
        }
    }

    fn skip_wsn(&mut self) {
        loop {
            match self.lexer.current_token() {
                Token::Space | Token::Separator => self.lexer.consume(),
                _ => return
            };
        }
    }

    fn skip_ws(&mut self) {
        loop {
            match self.lexer.current_token() {
                Token::Space => self.lexer.consume(),
                _ => return
            };
        }
    }

    fn parseerror(&self, msg: &str) -> ParseError {
        ParseError{
            msg: msg.to_string(),
            location: self.lexer.cur.clone(),
            backtrace: Backtrace::new()
        }
    }
}

pub fn parse(src: &str) -> Result<ast::Program, ParseError> {
    let mut parser = Parser {
        lexer: Lexer::new(src)
    };
    parser.parse()
}

//#[test]
//fn test_parser() {
//    //let result = parse("1+2*3");
//    let result = parse("hello.world(1)");
//    println!("{:#?}", result);
//    assert_eq!(result.unwrap(), 
//      ast::Program {
//        expr: ast::Expression::BinOp {
//                left: Box::new(ast::Expression::DecimalLiteral {value: 1}),
//                op: ast::BinOp::Add,
//                right: Box::new(ast::Expression::BinOp {
//                    left: Box::new(ast::Expression::DecimalLiteral {value: 2}),
//                    op: ast::BinOp::Mul,
//                    right: Box::new(ast::Expression::DecimalLiteral {value: 3}),
//                })
//        }
//      }
//    )
//}
