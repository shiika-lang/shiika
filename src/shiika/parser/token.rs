#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    Space,
    Separator,
    Word(&'a str),
    Symbol(&'a str),
    Number(&'a str),
    Eof,
}


