mod base;
mod token;
pub mod lexer;
mod expression_parser;
mod expression_parser_test;
//mod definition_parser;

use lexer::Lexer;
pub struct Parser<'a, 'b> {
    pub lexer: Lexer<'a, 'b>
}
