mod base;
mod token;
pub mod lexer;
mod statement_parser;
mod expression_parser;
//mod definition_parser;

#[derive(Debug)]
pub struct ParseError {
    pub msg: String,
    pub location: lexer::Cursor,
    pub backtrace: backtrace::Backtrace
}
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}
impl std::error::Error for ParseError {}


use lexer::Lexer;
pub struct Parser<'a, 'b> {
    pub lexer: Lexer<'a, 'b>
}
