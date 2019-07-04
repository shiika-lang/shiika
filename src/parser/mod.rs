mod base;
mod token;
pub mod lexer;
mod statement_parser;
mod expression_parser;

pub struct Parser<'a, 'b> {
    pub lexer: lexer::Lexer<'a, 'b>
}
