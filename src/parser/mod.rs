/// Parser
///
/// Implementaion rules
/// - Call `skip_ws`/`skip_wsn` before calling other `parse_xx`

macro_rules! parse_error {
    ( $self:ident, $( $arg:expr ),* ) => ({
        let msg = format!( $( $arg ),* );
        $self.parseerror(&msg)
    })
}

mod base;
pub mod token;
pub mod lexer;
mod definition_parser;
mod expression_parser;
use crate::ast;
use crate::error::Error;
use crate::parser::lexer::Lexer;
pub use crate::parser::token::Token;

pub struct Parser<'a> {
    pub lexer: Lexer<'a>,
    /// For debug print
    pub lv: usize,
}

impl<'a> Parser<'a> {
    pub fn new(src: &str) -> Parser {
        Parser {
            lexer: Lexer::new(src),
            lv: 0,
        }
    }

    pub fn parse(src: &str) -> Result<ast::Program, Error> {
        let mut parser = Parser::new(src);
        parser.parse_program()
    }

    fn parse_program(&mut self) -> Result<ast::Program, Error> {
        self.skip_wsn();
        let (toplevel_defs, exprs) = self.parse_toplevel_items()?;
        // No tokens should be left 
        if *self.current_token() != Token::Eof {
            return Err(parse_error!(self, "unexpected token: {:?}", self.current_token()))
        }
        Ok(ast::Program { toplevel_defs, exprs })
    }

    fn parse_toplevel_items(&mut self) -> Result<(Vec<ast::Definition>, Vec<ast::AstExpression>), Error> {
        let mut defs = vec![];
        let mut exprs = vec![];
        loop {
            match self.current_token() {
                Token::KwClass => defs.push(self.parse_class_definition()?),
                Token::KwDef => defs.push(self.parse_method_definition()?),
                Token::Eof | Token::KwEnd => break,
                _ => exprs.push(self.parse_expr()?),
            }
            self.skip_wsn();
        }
        Ok((defs, exprs))
    }
}
