#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    Space,
    Separator,
    UpperWord(&'a str),
    LowerWord(&'a str),
    Symbol(&'a str),
    Number(&'a str),
    Eof,
}


