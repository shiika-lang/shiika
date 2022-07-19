use crate::error::Error;
pub use crate::lexer;
pub use crate::lexer::*;
pub use crate::Parser;
use ariadne::{Label, Report, ReportKind, Source};
use std::fs;

use shiika_ast::*;

impl<'a> Parser<'a> {
    // Consume a separator and its surrounding spaces
    pub(super) fn expect_sep(&mut self) -> Result<(), Error> {
        match self.current_token() {
            Token::Separator => {
                self.consume_token()?;
            }
            Token::Eof => (),
            token => return Err(parse_error!(self, "expected separator but got {:?}", token)),
        }
        self.skip_wsn()?;
        Ok(())
    }

    /// Generates error if the current token does not equal to `token`.
    /// Consumes the token if succeed.
    ///
    /// Note: Takes `Token` rather than `&Token` for convenience.
    pub(super) fn expect(&mut self, token: Token) -> Result<Token, Error> {
        if *self.current_token() == token {
            Ok(self.consume_token()?)
        } else {
            Err(parse_error!(
                self,
                "expected {:?} but got {:?}",
                token,
                self.current_token()
            ))
        }
    }

    pub(super) fn skip_wsn(&mut self) -> Result<(), Error> {
        loop {
            match self.current_token() {
                Token::Space | Token::Separator => self.consume_token()?,
                _ => return Ok(()),
            };
        }
    }

    pub(super) fn skip_ws(&mut self) -> Result<(), Error> {
        loop {
            match self.current_token() {
                Token::Space => self.consume_token()?,
                _ => return Ok(()),
            };
        }
    }

    /// Consume the current token and return it
    pub(super) fn consume_token(&mut self) -> Result<Token, Error> {
        let tok = self.current_token();
        self.debug_log(&format!("consume_token {:?}", &tok));
        self.lexer.consume_token()
    }

    /// Consume the current token if it equals to `token`.
    /// Return whether matched and consumed
    pub(super) fn consume(&mut self, token: Token) -> Result<bool, Error> {
        if self.current_token_is(token) {
            self.consume_token()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Return true if the current token is `token`
    ///
    /// Note: Takes `Token` rather than `&Token` for convenience.
    pub(super) fn current_token_is(&mut self, token: Token) -> bool {
        *self.current_token() == token
    }

    pub(super) fn current_token(&self) -> &Token {
        &self.lexer.current_token
    }

    /// Return next token
    pub(super) fn peek_next_token(&mut self) -> Result<Token, Error> {
        self.lexer.peek_next()
    }

    /// Return next token which is not `Token::Space`
    /// Note: newlines are not skipped. (i.e. this function may return Token::Newline)
    pub(super) fn next_nonspace_token(&mut self) -> Result<Token, Error> {
        if self.current_token_is(Token::Space) {
            self.lexer.peek_next()
        } else {
            Ok(self.current_token().clone())
        }
    }

    /// Get the lexer position
    pub(super) fn current_position(&self) -> Cursor {
        self.lexer.cur.clone()
    }

    /// Rewind lexer position (backtrack)
    pub(super) fn rewind_to(&mut self, cur: Cursor) -> Result<(), Error> {
        self.lexer.set_position(cur)
    }

    pub(super) fn set_lexer_state(&mut self, state: LexerState) {
        self.lexer.set_state(state);
    }

    pub(super) fn set_lexer_gtgt_mode(&mut self, mode: bool) {
        self.lexer.rshift_is_gtgt = mode;
    }

    pub(super) fn parseerror(&self, msg: &str) -> Error {
        let (begin, end) = self.lexer.location_span();
        let path = format!("{}", self.ast.filepath.display()); // ariadne 0.1.5 needs Id: Display (zesterer/ariadne#12)
        let span = (&path, begin.pos..end.pos);
        let src = Source::from(fs::read_to_string(&*self.ast.filepath).unwrap_or_default());
        let mut report = vec![];
        Report::build(ReportKind::Error, &path, begin.pos)
            .with_message(msg)
            .with_label(Label::new(span))
            .finish()
            .write((&path, src), &mut report)
            .unwrap();
        Error::ParseError(String::from_utf8_lossy(&report).to_string())
    }

    /// Print parser debug log (uncomment to enable)
    pub(super) fn debug_log(&self, _msg: &str) {
        //println!("{}{} {}", self.lv_space(), _msg, self.lexer.debug_info());
    }
    #[allow(dead_code)]
    fn lv_space(&self) -> String {
        "  ".repeat(self.lv)
    }
}
