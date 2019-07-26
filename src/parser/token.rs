#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Eof,
    Space,
    Separator, // Newline or ';'
    UpperWord(String),
    LowerWord(String),
    Number(String),
    // Symbols
    LParen,       //  ( 
    RParen,       //  ) 
    LSqBracket,   //  [ 
    RSqBracket,   //  ] 
    LAngBracket,  //  < 
    RAngBracket,  //  > 
    LBrace,       //  { 
    RBrace,       //  } 
    Plus,         //  + 
    RightArrow,   //  ->
    Minus,        //  - 
    Mul,          //  * 
    Div,          //  / 
    Mod,          //  % 
    EqEq,         //  ==
    Equal,        //  = 
    Bang,         //  ! 
    Dot,          //  . 
    At,           //  @ 
    Tilde,        //  ~ 
    Question,     //  ? 
    Comma,        //  , 
    Colon,        //  :      
    And,          //  &
    AndAnd,       //  &&
    Or,           //  |
    OrOr,         //  ||
    // Keywords
    KwClass,
    KwEnd,
    KwDef,
    KwAnd,
    KwOr,
    KwNot,
    KwIf,
    KwUnless,
    KwThen,
    KwElse,
    KwSelf,
    KwTrue,
    KwFalse,
}

impl Token {
    pub fn upper_word(s: &str) -> Token { Token::UpperWord(s.to_string()) }
    pub fn lower_word(s: &str) -> Token { Token::LowerWord(s.to_string()) }
    pub fn number(s: &str) -> Token { Token::Number(s.to_string()) }
}
