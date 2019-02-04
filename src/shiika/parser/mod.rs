mod lexer;
mod parser_test;

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
        self.skip_wsn();
        Ok(ast::Program {
            expr: self.parse_expr()?
        })
    }

    fn parse_expr(&mut self) -> Result<ast::Expression, ParseError> {
        match self.current_token() {
            Token::Eof => Err(self.parseerror("unexpected EOF")),
            Token::Word("if") => self.parse_if_expr(),
            _ => self.parse_additive_expr(),
        }
    }

    fn parse_if_expr(&mut self) -> Result<ast::Expression, ParseError> {
        assert_eq!(*self.current_token(), Token::Word("if"));

        self.consume_token();
        self.skip_ws();
        let cond_expr = Box::new(self.parse_expr()?);
        self.skip_ws();
        if self.current_token_is(&Token::Word("then")) {
            self.consume_token();
            self.skip_wsn();
        }
        else {
            self.expect(Token::Separator)?;
        }
        let then_expr = Box::new(self.parse_expr()?);
        self.skip_wsn();
        if self.current_token_is(&Token::Word("else")) {
            self.consume_token();
            self.skip_wsn();
            let else_expr = Some(Box::new(self.parse_expr()?));
            Ok(ast::Expression::If { cond_expr, then_expr, else_expr })
        }
        else {
            self.expect(Token::Word("end"))?;
            let else_expr = None;
            Ok(ast::Expression::If { cond_expr, then_expr, else_expr })
        }
    }

    fn parse_additive_expr(&mut self) -> Result<ast::Expression, ParseError> {
        let left = self.parse_multiplicative_expr()?;
        self.skip_ws();

        match self.current_token() {
            Token::Symbol(s @ "+") | Token::Symbol(s @ "-") => {
                let op = if *s == "+" { ast::BinOp::Add }
                         else { ast::BinOp::Sub };
                self.consume_token();
                self.skip_wsn();
                let right = self.parse_expr()?;
                Ok(ast::bin_op_expr(left, op, right))
            },
            _ => Ok(left)
        }
    }

    fn parse_multiplicative_expr(&mut self) -> Result<ast::Expression, ParseError> {
        let left = self.parse_method_call()?;
        self.skip_ws();

        match self.current_token() {
            Token::Symbol(s @ "*") | Token::Symbol(s @ "/") | Token::Symbol(s @ "%") => {
                let op = if *s == "*" { ast::BinOp::Mul }
                         else if *s == "/" { ast::BinOp::Div }
                         else { ast::BinOp::Mod };
                self.consume_token();
                self.skip_wsn();
                let right = self.parse_multiplicative_expr()?;
                Ok(ast::bin_op_expr(left, op, right))
            },
            _ => Ok(left)
        }
    }

    fn parse_method_call(&mut self) -> Result<ast::Expression, ParseError> {
        let mut receiver_expr;
        let receiver_has_paren;
        match self.current_token() {
            Token::Word(s) => {
                receiver_expr = ast::Expression::Name(s.to_string());
                self.consume_token();
                receiver_has_paren = false;
            },
            Token::Symbol("(") => {
                receiver_expr = self.parse_parenthesized_expr()?;
                receiver_has_paren = true;
            },
            _ => {
                receiver_expr = self.parse_parenthesized_expr()?;
                receiver_has_paren = false;
            }
        }

        match self.current_token() {
            Token::Space => {
                if receiver_has_paren {
                    // (foo) ...
                    return Ok(receiver_expr);
                }
                else {
                    let method_name;
                    if let ast::Expression::Name(s) = &receiver_expr {
                        // foo ...
                        method_name = s;
                    }
                    else {
                        // 1 ...
                        return Ok(receiver_expr);
                    }
                    match self.parse_method_call_args()? {
                        None => Ok(receiver_expr),
                        Some(arg_exprs) => {
                            Ok(ast::Expression::MethodCall{
                                receiver_expr: None,
                                method_name: method_name.to_string(),
                                arg_exprs: arg_exprs
                            })
                        }
                    }
                }
            },
            Token::Symbol(".") => {
                self.consume_token();
                let mut method_name;
                match self.current_token() {
                    Token::Word(s) => {
                        method_name = s.to_string();
                        self.consume_token();
                    },
                    token => {
                        let msg = format!("expected ident but got {:?}", token);
                        return Err(self.parseerror(&msg))
                    }
                };
                // foo.bar
                let arg_exprs = match self.parse_method_call_args()? {
                                    None => Vec::new(),
                                    Some(v) => v
                                };
                Ok(ast::Expression::MethodCall{ 
                    receiver_expr: Some(Box::new(receiver_expr)),
                    method_name: method_name,
                    arg_exprs: arg_exprs
                })
            },
            Token::Symbol("(") => {
                // foo(
                match self.parse_method_call_args()? {
                    None => Ok(receiver_expr),
                    Some(arg_exprs) => {
                        let method_name = if let ast::Expression::Name(s) = receiver_expr {
                                            s
                                          } else { panic!() };
                        Ok(ast::Expression::MethodCall{
                            receiver_expr: None,
                            method_name: method_name.to_string(),
                            arg_exprs: arg_exprs
                        })
                    }
                }
            },
            Token::Symbol(_) => {
                // foo+
                Ok(receiver_expr)
            },
            Token::Separator | Token:: Eof => {
                // foo;
                Ok(receiver_expr)
            },
            Token::Word(_) => {
                // (foo)bar
                Err(self.parseerror("unexpected ident"))
            },
            Token::Number(_) => {
                // (foo)123
                Err(self.parseerror("unexpected number"))
            },
        }
    }

    fn parse_method_call_args(&mut self) -> Result<Option<Vec<ast::Expression>>, ParseError> {
        self.skip_ws();
        let has_paren;
        match self.current_token() {
            Token::Space => panic!(),
            Token::Separator | Token::Eof => {
                // foo ;
                // foo.bar;
                return Ok(None)
            }
            Token::Symbol("(") => {
                // foo(
                // foo (...
                // foo.bar(
                has_paren = true
            }
            Token::Symbol(_) => {
                // foo +
                // foo.bar+
                return Ok(None)
            },
            Token::Word(_) | Token::Number(_) => {
                // foo bar
                // foo 123
                has_paren = false
            }
        }

        let mut arg_exprs: Vec<ast::Expression> = Vec::new();
        loop {
            arg_exprs.push(self.parse_expr()?);
            self.skip_ws();
            match self.current_token() {
                Token::Space => panic!(),
                Token::Separator | Token::Eof => {
                    break
                },
                Token::Symbol(",") => {
                    self.consume_token();
                    self.skip_ws();
                },
                Token::Symbol(")") => {
                    if has_paren {
                        self.consume_token();
                        break
                    }
                    else {
                        return Err(self.parseerror("unexpected `)'"));
                    }
                },
                _ => {
                    let msg = format!("unexpected token: {:?}", self.current_token());
                    return Err(self.parseerror(&msg));
                }
            }
        }
        Ok(Some(arg_exprs))
    }

    fn parse_parenthesized_expr(&mut self) -> Result<ast::Expression, ParseError> {
        if *self.current_token() != Token::Symbol("(") {
            return self.parse_decimal_literal();
        }
        self.consume_token();
        self.skip_wsn();
        let expr = self.parse_expr()?;
        self.skip_wsn();
        self.expect(Token::Symbol(")"))?;
        Ok(expr)
    }

    fn parse_decimal_literal(&mut self) -> Result<ast::Expression, ParseError> {
        match self.current_token() {
            Token::Number(s) => {
                let value = s.parse().unwrap();
                self.consume_token();
                Ok(ast::Expression::DecimalLiteral{ value })
            },
            _ => {
                Err(self.parseerror("expected decimal literal"))
            }
        }
    }

    fn expect_sep(&mut self) -> Result<(), ParseError> {
        self.skip_ws();
        self.expect(Token::Separator)?;
        self.skip_wsn();
        Ok(())
    }

    fn expect(&mut self, token: Token) -> Result<&Token, ParseError> {
        if self.current_token_is(&token) {
            Ok(self.current_token())
        }
        else {
            let msg = format!("expected {:?} but got {:?}", token, self.current_token());
            Err(self.parseerror(&msg))
        }
    }

    fn skip_wsn(&mut self) {
        loop {
            match self.current_token() {
                Token::Space | Token::Separator => self.consume_token(),
                _ => return
            };
        }
    }

    fn skip_ws(&mut self) {
        loop {
            match self.current_token() {
                Token::Space => self.consume_token(),
                _ => return
            };
        }
    }

    fn consume_token(&mut self) {
        self.lexer.consume_token();
    }

    fn current_token_is(&mut self, token: &Token) -> bool {
        *self.lexer.current_token() == *token
    }

    fn current_token(&mut self) -> &Token {
        self.lexer.current_token()
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
