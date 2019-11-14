#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Token {
    Bof,
    Eof,
    Space,
    Separator, // Newline, ';' or comment
    UpperWord(String),
    LowerWord(String),
    Number(String),
    // Symbols
    LParen,       //  ( 
    RParen,       //  ) 
    LSqBracket,   //  [ 
    RSqBracket,   //  ] 
    LBrace,       //  { 
    RBrace,       //  } 
    UnaryPlus,    //  +a
    BinaryPlus,   //  a + b
    RightArrow,   //  ->
    UnaryMinus,   //  -a
    BinaryMinus,  //  a - b
    Mul,          //  * 
    Div,          //  / 
    Mod,          //  % 
    EqEq,         //  ==
    NotEq,        //  !=
    LessThan,     //  < 
    GraterThan,   //  > 
    LessEq,       //  <=
    GraterEq,     //  >=
    Equal,        //  = 
    Bang,         //  ! 
    Dot,          //  . 
    At,           //  @ 
    Tilde,        //  ~ 
    Question,     //  ? 
    Comma,        //  , 
    Colon,        //  :      
    ColonColon,   //  ::
    And,          //  &
    AndAnd,       //  &&
    Or,           //  |
    OrOr,         //  ||
    // Keywords
    KwClass,
    KwEnd,
    KwDef,
    KwVar,
    KwAnd,
    KwOr,
    KwNot,
    KwIf,
    KwUnless,
    KwWhile,
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

    /// Return true if a value may start with this token
    ///
    /// Must not be called on `Token::Space`
    pub fn value_starts(&self) -> bool {
        match self {
            Token::Bof => false,
            Token::Eof => false,
            Token::Space => panic!("must not called on Space"),
            Token::Separator => false, // Newline or ';'
            Token::UpperWord(_) => true,
            Token::LowerWord(_) => true,
            Token::Number(_) => true,
            // Symbols
            Token::LParen => true,        //  ( 
            Token::RParen => false,       //  ) 
            Token::LSqBracket => false,   //  [ 
            Token::RSqBracket => false,   //  ] 
            Token::LBrace => false,       //  { 
            Token::RBrace => false,       //  } 
            Token::UnaryPlus => true,     //  + 
            Token::BinaryPlus => false,   //  + 
            Token::RightArrow => false,   //  ->
            Token::UnaryMinus => true,    //  -
            Token::BinaryMinus => false,  //  -
            Token::Mul => true,           //  * 
            Token::Div => true,           //  / 
            Token::Mod => false,          //  % 
            Token::EqEq => false,         //  ==
            Token::NotEq => false,        //  !=
            Token::LessThan => false,     //  < 
            Token::GraterThan => false,   //  > 
            Token::LessEq => false,       //  <=
            Token::GraterEq => false,     //  >=
            Token::Equal => false,        //  = 
            Token::Bang => true,          //  ! 
            Token::Dot => false,          //  . 
            Token::At => true,            //  @ 
            Token::Tilde => true,         //  ~ 
            Token::Question => false,     //  ? 
            Token::Comma => false,        //  , 
            Token::Colon => true,         //  :      
            Token::ColonColon => true,    //  ::
            Token::And => true,           //  &
            Token::AndAnd => false,       //  &&
            Token::Or => false,           //  |
            Token::OrOr => false,         //  ||
            // Keywords
            Token::KwClass => false,
            Token::KwEnd => false,
            Token::KwDef => false,
            Token::KwVar => false,
            Token::KwAnd => false,
            Token::KwOr => false,
            Token::KwNot => true,
            Token::KwIf => true,
            Token::KwUnless => true,
            Token::KwWhile => true,
            Token::KwThen => false,
            Token::KwElse => false,
            Token::KwSelf => true,
            Token::KwTrue => true,
            Token::KwFalse => true,
        }
    }
}
