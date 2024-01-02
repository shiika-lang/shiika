#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Token {
    Bof,
    Eof,
    Space,
    Semicolon,
    Newline,
    UpperWord(String),
    LowerWord(String),
    IVar(String),
    Number(String),
    Str(String),
    KeyName(String), // Such as `foo:`
    StrWithInterpolation {
        head: String,  // Contents before `#{'
        inspect: bool, // true if `\{}', which calls .inspect instead of .to_s
    },
    // Symbols
    LParen,      //  (
    RParen,      //  )
    LSqBracket,  //  [
    RSqBracket,  //  ]
    LBrace,      //  {
    RBrace,      //  }
    UnaryPlus,   //  +a
    BinaryPlus,  //  a + b
    RightArrow,  //  ->
    UnaryMinus,  //  -a
    BinaryMinus, //  a - b
    Mul,         //  *
    Div,         //  /
    Mod,         //  %
    EqEq,        //  ==
    NotEq,       //  !=
    LessThan,    //  <
    GreaterThan, //  >
    LessEq,      //  <=
    GreaterEq,   //  >=
    Equal,       //  =
    Bang,        //  !
    Dot,         //  .
    At,          //  @
    Tilde,       //  ~
    Question,    //  ?
    Comma,       //  ,
    Colon,       //  :
    ColonColon,  //  ::
    AndAnd,      //  &&
    OrOr,        //  ||
    And,         //  &
    Or,          //  |
    Xor,         //  ^
    LShift,      //  <<
    RShift,      //  >>
    PlusEq,      //  +=
    MinusEq,     //  -=
    MulEq,       //  *=
    DivEq,       //  /=
    ModEq,       //  %=
    LShiftEq,    //  <<=
    RShiftEq,    //  >>=
    AndEq,       //  &=
    OrEq,        //  |=
    XorEq,       //  ^=
    AndAndEq,    //  &&=
    OrOrEq,      //  ||=
    // Method name only
    UPlusMethod,  //  +@
    UMinusMethod, //  -@
    GetMethod,    //  []
    SetMethod,    //  []=
    Specialize,   //  <> (used internally)
    // Keywords
    KwRequire,
    KwClass,
    KwModule,
    KwRequirement,
    KwEnum,
    KwCase,
    KwIn,
    KwOut,
    KwEnd,
    KwDef,
    KwLet,
    KwVar,
    KwAnd,
    KwOr,
    KwNot,
    KwIf,
    KwUnless,
    KwMatch,
    KwWhen,
    KwWhile,
    KwBreak,
    KwReturn,
    KwThen,
    KwElse,
    KwElsif,
    KwFn,
    KwDo,
    KwSelf,
    KwTrue,
    KwFalse,
    // Keywords (modifier version)
    ModIf,
    ModUnless,
}

impl Token {
    pub fn is_assignment_token(&self) -> bool {
        matches!(
            self,
            Token::Equal
                | Token::PlusEq
                | Token::MinusEq
                | Token::MulEq
                | Token::DivEq
                | Token::ModEq
                | Token::LShiftEq
                | Token::RShiftEq
                | Token::AndEq
                | Token::OrEq
                | Token::XorEq
                | Token::AndAndEq
                | Token::OrOrEq
        )
    }

    /// Return true if a value may start with this token
    ///
    /// Must not be called on `Token::Space`
    pub fn value_starts(&self) -> bool {
        match self {
            Token::Bof => false,
            Token::Eof => false,
            Token::Space => panic!("must not called on Space"),
            Token::Semicolon => false,
            Token::Newline => false,
            Token::UpperWord(_) => true,
            Token::LowerWord(_) => true,
            Token::IVar(_) => true,
            Token::Number(_) => true,
            Token::Str(_) => true,
            Token::KeyName(_) => false,
            Token::StrWithInterpolation { .. } => true,
            // Symbols
            Token::LParen => true,       //  (
            Token::RParen => false,      //  )
            Token::LSqBracket => true,   //  [
            Token::RSqBracket => false,  //  ]
            Token::LBrace => false,      //  {
            Token::RBrace => false,      //  }
            Token::UnaryPlus => true,    //  +
            Token::BinaryPlus => false,  //  +
            Token::RightArrow => false,  //  ->
            Token::UnaryMinus => true,   //  -
            Token::BinaryMinus => false, //  -
            Token::Mul => false,         //  *
            Token::Div => false,         //  /
            Token::Mod => false,         //  %
            Token::EqEq => false,        //  ==
            Token::NotEq => false,       //  !=
            Token::LessThan => false,    //  <
            Token::GreaterThan => false, //  >
            Token::LessEq => false,      //  <=
            Token::GreaterEq => false,   //  >=
            Token::Equal => false,       //  =
            Token::Bang => true,         //  !
            Token::Dot => false,         //  .
            Token::At => true,           //  @
            Token::Tilde => true,        //  ~
            Token::Question => false,    //  ?
            Token::Comma => false,       //  ,
            Token::Colon => true,        //  :
            Token::ColonColon => true,   //  ::
            Token::AndAnd => false,      //  &&
            Token::OrOr => false,        //  ||
            Token::And => false,         //  &
            Token::Or => false,          //  |
            Token::Xor => false,         //  ^
            Token::LShift => false,      //  <<
            Token::RShift => false,      //  >>
            Token::PlusEq => false,      //  +=
            Token::MinusEq => false,     //  -=
            Token::MulEq => false,       //  *=
            Token::DivEq => false,       //  /=
            Token::ModEq => false,       //  %=
            Token::LShiftEq => false,    //  <<=
            Token::RShiftEq => false,    //  >>=
            Token::AndEq => false,       //  &=
            Token::OrEq => false,        //  |=
            Token::XorEq => false,       //  ^=
            Token::AndAndEq => false,    //  &&=
            Token::OrOrEq => false,      //  ||=
            // Method name only
            Token::UPlusMethod => false,  //  +@
            Token::UMinusMethod => false, //  -@
            Token::GetMethod => false,    //  []
            Token::SetMethod => false,    //  []=
            Token::Specialize => false,   //  <>
            // Keywords
            Token::KwRequire => false,
            Token::KwClass => false,
            Token::KwModule => false,
            Token::KwRequirement => false,
            Token::KwEnum => false,
            Token::KwCase => false,
            Token::KwIn => false,
            Token::KwOut => false,
            Token::KwEnd => false,
            Token::KwDef => false,
            Token::KwLet => false,
            Token::KwVar => false,
            Token::KwAnd => false,
            Token::KwOr => false,
            Token::KwNot => true,
            Token::KwIf => true,
            Token::KwUnless => true,
            Token::KwMatch => true,
            Token::KwWhen => false,
            Token::KwWhile => true,
            Token::KwBreak => false,
            Token::KwReturn => false,
            Token::KwThen => false,
            Token::KwElse => false,
            Token::KwElsif => false,
            Token::KwFn => true,
            Token::KwDo => false,
            Token::KwSelf => true,
            Token::KwTrue => true,
            Token::KwFalse => true,
            // Keywords (modifier version)
            Token::ModIf => false,
            Token::ModUnless => false,
        }
    }
}
