macro_rules! parse_error {
    ( $self:ident, $( $arg:expr ),* ) => ({
        let msg = format!( $( $arg ),* );
        $self.parseerror(&msg)
    })
}

mod base;
mod token;
pub mod lexer;
mod definition_parser;
mod statement_parser;
mod expression_parser;
use crate::ast;
use crate::error::Error;
use crate::parser::lexer::Lexer;

pub struct Parser<'a, 'b> {
    pub lexer: Lexer<'a, 'b>
}

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
            toplevel_defs: Vec::new(),
            stmts: self.parse_stmts()?,
        })
    }
}
