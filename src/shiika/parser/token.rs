#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    Word(&'a str),
    Symbol(char),
    Number(&'a str),
}
