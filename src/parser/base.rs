/// Provides utilities for *_parser.rs
pub use crate::ast;
pub use crate::error::*;
pub use crate::parser::Parser;
pub use crate::parser::token::Token;
pub use crate::parser::lexer;
pub use crate::parser::lexer::*;

impl<'a> Parser<'a> {
    // Consume a separator and its surrounding spaces
    pub (in super) fn expect_sep(&mut self) -> Result<(), Error> {
        match self.current_token() {
            Token::Separator => { self.consume_token(); },
            Token::Eof => (),
            token => return Err(parse_error!(self, "expected separator but got {:?}", token))
        }
        self.skip_wsn();
        Ok(())
    }

    /// Generates error if the current token does not equal to `token`.
    ///
    /// Note: Takes `Token` rather than `&Token` for convenience.
    pub (in super) fn expect(&mut self, token: Token) -> Result<Token, Error> {
        if *self.current_token() == token {
            Ok(self.consume_token())
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

    /// Consume the current token and return it
    pub (in super) fn consume_token(&mut self) -> Token {
        let tok = self.current_token();
        self.debug_log(&format!("consume_token {:?}", &tok));
        self.lexer.consume_token()
    }

    /// Consume the current token if it equals to `token`.
    /// Return whether matched and consumed
    pub (in super) fn consume(&mut self, token: Token) -> bool {
        if self.current_token_is(token) {
            self.consume_token();
            true
        }
        else {
            false
        }
    }

    /// Return true if the current token is `token`
    ///
    /// Note: Takes `Token` rather than `&Token` for convenience.
    pub (in super) fn current_token_is(&mut self, token: Token) -> bool {
        *self.current_token() == token
    }

    pub (in super) fn current_token(&self) -> &Token {
        &self.lexer.current_token
    }

    /// Return next token
    pub (in super) fn peek_next_token(&mut self) -> Token {
        self.lexer.peek_next()
    }

    /// Return next token which is not `Token::Space`
    /// Note: newlines are not skipped. (i.e. this function may return Token::Newline)
    pub (in super) fn next_nonspace_token(&mut self) -> Token {
        if self.current_token_is(Token::Space) {
            self.lexer.peek_next()
        }
        else {
            self.current_token().clone()
        }
    }

    /// Get the lexer position
    pub (in super) fn current_position(&self) -> Cursor {
        self.lexer.cur.clone()
    }

    /// Rewind lexer position (backtrack)
    pub (in super) fn rewind_to(&mut self, cur: Cursor) {
        self.lexer.set_position(cur);
    }

    pub (in super) fn set_lexer_state(&mut self, state: LexerState) {
        self.lexer.set_state(state);
    }

    pub (in super) fn parseerror(&self, msg: &str) -> Error {
        Error {
            msg: msg.to_string(),
            backtrace: backtrace::Backtrace::new(),
            details: ErrorDetails::ParseError {
                location: self.lexer.cur.clone(),
            }
        }
    }

    /// Print parser debug log (uncomment to enable)
    pub (in super) fn debug_log(&self, _msg: &str) {
        //println!("{}{} {}", self.lv_space(), _msg, self.lexer.debug_info());
    }
    #[allow(dead_code)]
    fn lv_space(&self) -> String {
        "  ".repeat(self.lv)
    }
}
