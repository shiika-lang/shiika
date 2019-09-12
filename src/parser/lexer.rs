use super::token::Token;

#[derive(Debug)]
pub struct Lexer<'a> {
    pub src: &'a str,
    pub cur: Cursor,
    state: LexerState,
    /// true if the last token is a space
    space_seen: bool,
    pub current_token: Token,
    next_cur: Option<Cursor>,
}

/// Flags to decide a `-`, `+`, etc. is unary or binary.
///
/// - `p(-x)`  # unary minus             ExprBegin
/// - `p(- x)` # unary minus             ExprBegin   
/// - `p( - x)`# unary minus             ExprBegin   
/// - `p- x`   # binary minus (unusual)  ExprEnd
/// - `p-x`    # binary minus            ExprEnd
/// - `p - x`  # binary minus            ExprArg
/// - `p -x`   # unary minus             ExprArg
/// - `1 -2`   # binary minus (unusual)  ExprArg  
#[derive(Debug)]
pub enum LexerState {
    /// A new expression begins here
    /// `+`/`-` is always unary
    ExprBegin,
    /// End of an expression
    /// `+`/`-` is always binary
    ExprEnd,
    /// Beginning of a (possible) first paren-less arg of a method call.
    /// `+`/`-` is unary, if with space before it and no space after it (`p -x`)
    ExprArg
}

#[derive(Debug, PartialEq, Clone)]
pub struct Cursor {
    line: usize,
    col: usize,
    pos: usize, // Number of bytes from the begginning of the file
}

impl Cursor {
    pub fn new() -> Cursor {
        Cursor {
            line: 0,
            col: 0,
            pos: 0,
        }
    }

    pub fn peek(&self, src: &str) -> Option<char> {
        src[self.pos..].chars().next()
    }

    // Peek the second next character.
    // Must not be called on EOF
    pub fn peek2(&self, src: &str) -> Option<char> {
        if let Some(c) = self.peek(src) {
            let pos = self.pos + c.len_utf8();
            src[pos..].chars().next()
        }
        else {
            panic!("peek2 must not be called on EOF")
        }
    }

    pub fn proceed(&mut self, src: &str) -> char {
        let c = src[self.pos..].chars().next().unwrap();
        if c == '\n' {
            self.line += 1;
            self.col = 0
        }
        else {
            self.col += 1
        }
        self.pos += c.len_utf8();
        c
    }
}

#[derive(Debug, PartialEq)]
enum CharType {
    Space,
    Separator, // Newline or ';'
    UpperWord, // identifier which starts with upper-case letter
    LowerWord, // Keyword or identifier which starts with lower-case letter
    Symbol, // '+', '(', etc.
    Number, // '0'~'9'
    Eof,
}

impl<'a> Lexer<'a> {
    /// Create lexer and get the first token
    pub fn new(src: &str) -> Lexer {
        let mut lexer = Lexer {
            src: src,
            cur: Cursor::new(),
            state: LexerState::ExprBegin,
            space_seen: false,
            next_cur: None,
            current_token: Token::Bof,
        };
        lexer.read_token();
        lexer
    }

    pub fn set_state(&mut self, state: LexerState) {
        self.state = state;
    }

    fn set_current_token(&mut self, token: Token) {
        self.space_seen = self.current_token == Token::Space;
        self.current_token = token;
    }

    pub fn debug_info(&self) -> String {
        format!("{:?} {:?}", self.current_token, self.state)
    }

    /// Remove the current token and read next
    ///
    /// # Examples
    ///
    /// ```
    /// use shiika::parser::lexer::Lexer;
    /// use shiika::parser::token::Token;
    ///
    /// let src = "  1";
    /// let mut lexer = Lexer::new(src);
    ///
    /// assert_eq!(lexer.current_token, Token::Space);
    /// lexer.consume_token();
    /// assert_eq!(lexer.current_token, Token::number("1"));
    /// ```
    pub fn consume_token(&mut self) -> Token {
        self.cur = self.next_cur.take().unwrap();
        let tok = self.current_token.clone(); // PERF: how not to clone?
        self.read_token();
        tok
    }

    /// Move lexer position to `cur`
    ///
    /// # Examples
    ///
    /// ```
    /// use shiika::parser::lexer::Lexer;
    /// use shiika::parser::token::Token;
    ///
    /// let src = "1+2";
    /// let mut lexer = Lexer::new(src);
    ///
    /// lexer.consume_token();
    /// let cur = lexer.cur.clone();
    /// lexer.consume_token();
    /// lexer.set_position(cur);
    /// assert_eq!(lexer.current_token, Token::BinaryPlus);
    /// ```
    pub fn set_position(&mut self, cur: Cursor) {
        self.cur = cur;
        self.read_token();
    }

    /// Return the next token while keeping the current one
    ///
    /// # Examples
    ///
    /// ```
    /// use shiika::parser::lexer::Lexer;
    /// use shiika::parser::token::Token;
    ///
    /// let src = "@1";
    /// let mut lexer = Lexer::new(src);
    ///
    /// // Return the next token but does not move the position
    /// assert_eq!(lexer.peek_next(), Token::number("1"));
    /// assert_eq!(lexer.current_token, Token::At);
    ///
    /// // Return Eof when called on the end
    /// lexer.consume_token();
    /// lexer.consume_token();
    /// assert_eq!(lexer.peek_next(), Token::Eof);
    /// ```
    pub fn peek_next(&mut self) -> Token {
        let next_cur = self.next_cur.as_ref().unwrap().clone();
        let c = next_cur.peek(self.src);
        let mut next_next_cur = next_cur.clone();
        let (token, _) = match self.char_type(c) {
            CharType::Space     => (self.read_space(&mut next_next_cur), None),
            CharType::Separator => (self.read_separator(&mut next_next_cur), None),
            CharType::UpperWord => (self.read_upper_word(&mut next_next_cur, Some(&next_cur)), None),
            CharType::LowerWord => self.read_lower_word(&mut next_next_cur, Some(&next_cur)),
            CharType::Symbol    => self.read_symbol(&mut next_next_cur),
            CharType::Number    => (self.read_number(&mut next_next_cur, Some(&next_cur)), None),
            CharType::Eof       => (self.read_eof(), None),
        };
        token
    }

    /// Read a token and set it to `current_token`
    fn read_token(&mut self) {
        let c = self.cur.peek(self.src);
        let mut next_cur = self.cur.clone();
        let (token, new_state) = match self.char_type(c) {
            CharType::Space     => (self.read_space(&mut next_cur),            None),
            CharType::Separator => (self.read_separator(&mut next_cur),        None),
            CharType::UpperWord => (self.read_upper_word(&mut next_cur, None), Some(LexerState::ExprEnd)),
            CharType::LowerWord => self.read_lower_word(&mut next_cur, None),
            CharType::Symbol    => self.read_symbol(&mut next_cur),
            CharType::Number    => (self.read_number(&mut next_cur, None),     Some(LexerState::ExprEnd)),
            CharType::Eof       => (self.read_eof(),                           None),
        };
        self.set_current_token(token);
        if let Some(state) = new_state {
            self.state = state;
        }
        self.next_cur = Some(next_cur)
    }

    fn read_space(&mut self, next_cur: &mut Cursor) -> Token {
        loop {
            match self.char_type(next_cur.peek(self.src)) {
                CharType::Space => next_cur.proceed(self.src),
                _ => break
            };
        }
        Token::Space
    }

    fn read_separator(&mut self, next_cur: &mut Cursor) -> Token {
        loop {
            match self.char_type(next_cur.peek(self.src)) {
                CharType::Space | CharType::Separator => {
                    next_cur.proceed(self.src);
                },
                _ => break
            };
        }
        Token::Separator
    }

    fn read_upper_word(&mut self, next_cur: &mut Cursor, cur: Option<&Cursor>) -> Token {
        loop {
            match self.char_type(next_cur.peek(self.src)) {
                CharType::UpperWord | CharType::LowerWord | CharType::Number => {
                    next_cur.proceed(self.src);
                },
                _ => break
            }
        }
        let begin = match cur { Some(c) => c.pos, None => self.cur.pos };
        Token::UpperWord(self.src[begin..next_cur.pos].to_string())
    }

    fn read_lower_word(&mut self, next_cur: &mut Cursor, cur: Option<&Cursor>) -> (Token, Option<LexerState>) {
        loop {
            match self.char_type(next_cur.peek(self.src)) {
                CharType::UpperWord | CharType::LowerWord | CharType::Number => {
                    next_cur.proceed(self.src);
                },
                _ => break
            }
        }
        let begin = match cur { Some(c) => c.pos, None => self.cur.pos };
        let s = &self.src[begin..next_cur.pos];
        let (token, state) = match s {
            "class" => (Token::KwClass, LexerState::ExprBegin),
            "end" => (Token::KwEnd, LexerState::ExprEnd),
            "def" => (Token::KwDef, LexerState::ExprBegin),
            "and" => (Token::KwAnd, LexerState::ExprBegin),
            "or" => (Token::KwOr, LexerState::ExprBegin),
            "not" => (Token::KwNot, LexerState::ExprBegin),
            "if" => (Token::KwIf, LexerState::ExprBegin),
            "unless" => (Token::KwUnless, LexerState::ExprBegin),
            "then" => (Token::KwThen, LexerState::ExprBegin),
            "else" => (Token::KwElse, LexerState::ExprBegin),
            "self" => (Token::KwSelf, LexerState::ExprEnd),
            "true" => (Token::KwTrue, LexerState::ExprEnd),
            "false" => (Token::KwFalse, LexerState::ExprEnd),
            _ => (Token::LowerWord(s.to_string()), LexerState::ExprEnd),
        };
        (token, Some(state))
    }

    fn read_symbol(&mut self, next_cur: &mut Cursor) -> (Token, Option<LexerState>) {
        let c1 = next_cur.proceed(self.src);
        let c2 = next_cur.peek(self.src);
        let (token, state) = match c1 {
            '(' => (Token::LParen, LexerState::ExprBegin),
            ')' => (Token::RParen, LexerState::ExprEnd),
            '[' => (Token::LSqBracket, LexerState::ExprBegin),
            ']' => (Token::RSqBracket, LexerState::ExprEnd),
            '<' => (Token::LAngBracket, LexerState::ExprBegin),
            '>' => (Token::RAngBracket, LexerState::ExprBegin),
            '{' => (Token::LBrace, LexerState::ExprBegin),
            '}' => (Token::RBrace, LexerState::ExprEnd),
            '+' => {
                if self.is_unary(c2) {
                    (Token::UnaryPlus, LexerState::ExprBegin)
                }
                else {
                    (Token::BinaryPlus, LexerState::ExprBegin)
                }
            }
            '-' => {
                if c2 == Some('>') {
                    next_cur.proceed(self.src);
                    (Token::RightArrow, LexerState::ExprBegin)
                }
                else {
                    if self.is_unary(c2) {
                        (Token::UnaryMinus, LexerState::ExprBegin)
                    }
                    else {
                        (Token::BinaryMinus, LexerState::ExprBegin)
                    }
                }
            },
            '*' => (Token::Mul, LexerState::ExprBegin),
            '/' => (Token::Div, LexerState::ExprBegin),
            '%' => (Token::Mod, LexerState::ExprBegin),
            '=' => {
                if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    (Token::EqEq, LexerState::ExprBegin)
                }
                else {
                    (Token::Equal, LexerState::ExprBegin)
                }
            },
            '!' => (Token::Bang, LexerState::ExprBegin),
            '.' => (Token::Dot, LexerState::ExprBegin),
            '@' => (Token::At, LexerState::ExprBegin),
            '~' => (Token::Tilde, LexerState::ExprBegin),
            '?' => (Token::Question, LexerState::ExprBegin),
            ',' => (Token::Comma, LexerState::ExprBegin),
            ':' => (Token::Colon, LexerState::ExprBegin),
            '&' => {
                if c2 == Some('&') {
                    next_cur.proceed(self.src);
                    (Token::AndAnd, LexerState::ExprBegin)
                }
                else {
                    (Token::And, LexerState::ExprBegin)
                }
            },
            '|' => {
                if c2 == Some('|') {
                    next_cur.proceed(self.src);
                    (Token::OrOr, LexerState::ExprBegin)
                }
                else {
                    (Token::Or, LexerState::ExprBegin)
                }
            },
            c => {
                // TODO: this should be lexing error
                panic!("unknown symbol: {}", c)
            }
        };
        (token, Some(state))
    }

    fn is_unary(&self, next_char: Option<char>) -> bool {
        let ret = match self.state {
            LexerState::ExprBegin => true,
            LexerState::ExprEnd => false,
            LexerState::ExprArg => {
                self.current_token == Token::Space && next_char != Some(' ')
            }
        };
        ret
    }

    fn read_number(&mut self, next_cur: &mut Cursor, cur: Option<&Cursor>) -> Token {
        loop {
            match self.char_type(next_cur.peek(self.src)) {
                CharType::Number => {
                    next_cur.proceed(self.src);
                },
                CharType::UpperWord | CharType::LowerWord => {
                    // TODO: this should be lexing error
                    panic!("need space after a number")
                },
                CharType::Symbol => {
                    if next_cur.peek(self.src) == Some('.') {
                        if self.char_type(next_cur.peek2(self.src)) == CharType::Number {
                            next_cur.proceed(self.src);
                            next_cur.proceed(self.src);
                        }
                        else {
                            break
                        }
                    }
                    else {
                        break
                    }
                }
                _ => break
            }
        }
        let begin = match cur { Some(c) => c.pos, None => self.cur.pos };
        Token::Number(self.src[begin..next_cur.pos].to_string())
    }

    fn read_eof(&mut self) -> Token {
        Token::Eof
    }

    fn char_type(&self, cc: Option<char>) -> CharType {
        if cc == None {
            return CharType::Eof
        }
        match cc.unwrap() {
            ' ' | '\t' => CharType::Space,
            '\n' | ';' => CharType::Separator,
            '0'..='9' => CharType::Number,
            '(' | ')' | '[' | ']' | '<' | '>' | '{' | '}' |
            '+' | '-' | '*' | '/' | '%' | '=' | '!' |
            '.' | '@' | '~' | '?' | ',' | ':' | '|' | '&' => CharType::Symbol,
            'A'..='Z' => CharType::UpperWord,
            _ => CharType::LowerWord,
        }
    }
}
