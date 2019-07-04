pub use crate::ast;
pub use crate::error::*;
pub use crate::parser::Parser;
pub use crate::parser::token::Token;
pub use crate::parser::lexer;
pub use crate::parser::lexer::*;

impl<'a, 'b> Parser<'a, 'b> {
    pub fn new(src: &str) -> Parser {
        Parser {
            lexer: Lexer::new(src)
        }
    }

    pub fn parse(src: &str) -> Result<ast::Program, Error> {
        let mut parser = Parser::new(src);
        parser.parse_program()
    }

    fn parse_program(&mut self) -> Result<ast::Program, Error> {
        self.skip_wsn();
        Ok(ast::Program {
            class_defs: Vec::new(),
            stmts: self.parse_stmts()?,
        })
    }

    pub (in super) fn expect_sep(&mut self) -> Result<(), Error> {
        self.skip_ws();
        match self.current_token() {
            Token::Separator => { self.consume_token(); },
            Token::Eof => (),
            _ => {
                let msg = format!("expected separator but got {:?}", self.current_token());
                return Err(self.parseerror(&msg))
            }
        }
        self.skip_wsn();
        Ok(())
    }

    pub (in super) fn expect(&mut self, token: Token) -> Result<&Token, Error> {
        if self.current_token_is(&token) {
            Ok(self.current_token())
        }
        else {
            let msg = format!("expected {:?} but got {:?}", token, self.current_token());
            Err(self.parseerror(&msg))
        }
    }

    pub (in super) fn skip_wsn(&mut self) {
        loop {
            match self.current_token() {
                Token::Space | Token::Separator => self.consume_token(),
                _ => return
            };
        }
    }

    pub (in super) fn skip_ws(&mut self) {
        loop {
            match self.current_token() {
                Token::Space => self.consume_token(),
                _ => return
            };
        }
    }

    pub (in super) fn consume_token(&mut self) -> Token {
        self.lexer.consume_token()
    }

    pub (in super) fn current_token_is(&mut self, token: &Token) -> bool {
        *self.lexer.current_token() == *token
    }

    pub (in super) fn current_token(&mut self) -> &Token {
        self.lexer.current_token()
    }

    pub (in super) fn parseerror(&self, msg: &str) -> Error {
        Error::ParseError {
            msg: msg.to_string(),
            location: self.lexer.cur.clone(),
            backtrace: backtrace::Backtrace::new()
        }
    }
}
