use backtrace::Backtrace;
pub use super::Parser;
pub use super::super::ast;
pub use super::token::Token;
pub use super::lexer;
pub use super::lexer::*;

#[derive(Debug)]
pub struct ParseError {
    pub msg: String,
    pub location: lexer::Cursor,
    pub backtrace: Backtrace
}

impl<'a, 'b> Parser<'a, 'b> {
    pub fn parse(src: &str) -> Result<ast::Program, ParseError> {
        let mut parser = Parser {
            lexer: Lexer::new(src)
        };
        parser.parse_program()
    }

    pub (in super) fn parse_program(&mut self) -> Result<ast::Program, ParseError> {
        self.skip_wsn();
        Ok(ast::Program {
            expr: self.parse_expr()?
        })
    }

    pub (in super) fn expect_sep(&mut self) -> Result<(), ParseError> {
        self.skip_ws();
        self.expect(Token::Separator)?;
        self.skip_wsn();
        Ok(())
    }

    pub (in super) fn expect(&mut self, token: Token) -> Result<&Token, ParseError> {
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

    pub (in super) fn parseerror(&self, msg: &str) -> ParseError {
        ParseError{
            msg: msg.to_string(),
            location: self.lexer.cur.clone(),
            backtrace: Backtrace::new()
        }
    }
}
