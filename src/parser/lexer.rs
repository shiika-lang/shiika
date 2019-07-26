use super::token::Token;

pub struct Lexer<'a> {
    pub src: &'a str,
    pub cur: Cursor,
    current_token: Option<Token>,
    next_cur: Option<Cursor>,
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
            next_cur: None,
            current_token: None,
        };
        lexer.read_token();
        lexer
    }

    /// Return a reference to the current token
    ///
    /// # Examples
    ///
    /// ```
    /// use shiika::parser::lexer::Lexer;
    /// use shiika::parser::token::Token;
    ///
    /// let src = "  1";
    /// let mut lexer = Lexer::new(src);
    /// assert_eq!(*lexer.current_token(), Token::Space);
    /// ```
    pub fn current_token(&self) -> &Token {
        self.current_token.as_ref().unwrap()
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
    /// assert_eq!(*lexer.current_token(), Token::Space);
    /// lexer.consume_token();
    /// assert_eq!(*lexer.current_token(), Token::number("1"));
    /// ```
    pub fn consume_token(&mut self) -> Token {
        self.cur = self.next_cur.take().unwrap();
        let tok = self.current_token.take().unwrap();
        self.read_token();
        tok
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
    /// assert_eq!(lexer.peek_next(), Token::number("1"));
    /// assert_eq!(*lexer.current_token(), Token::At);
    /// ```
    pub fn peek_next(&mut self) -> Token {
        let next_cur = self.next_cur.as_ref().unwrap().clone();
        let c = next_cur.peek(self.src);
        let mut next_next_cur = next_cur.clone();
        match self.char_type(c) {
            CharType::Space     => self.read_space(&mut next_next_cur),
            CharType::Separator => self.read_separator(&mut next_next_cur),
            CharType::UpperWord => self.read_upper_word(&mut next_next_cur, Some(&next_cur)),
            CharType::LowerWord => self.read_lower_word(&mut next_next_cur, Some(&next_cur)),
            CharType::Symbol    => self.read_symbol(&mut next_next_cur),
            CharType::Number    => self.read_number(&mut next_next_cur, Some(&next_cur)),
            CharType::Eof       => self.read_eof(),
        }
    }

    fn read_token(&mut self) {
        let c = self.cur.peek(self.src);
        let mut next_cur = self.cur.clone();
        self.current_token = Some(
            match self.char_type(c) {
                CharType::Space     => self.read_space(&mut next_cur),
                CharType::Separator => self.read_separator(&mut next_cur),
                CharType::UpperWord => self.read_upper_word(&mut next_cur, None),
                CharType::LowerWord => self.read_lower_word(&mut next_cur, None),
                CharType::Symbol    => self.read_symbol(&mut next_cur),
                CharType::Number    => self.read_number(&mut next_cur, None),
                CharType::Eof       => self.read_eof(),
            }
        );
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

    fn read_lower_word(&mut self, next_cur: &mut Cursor, cur: Option<&Cursor>) -> Token {
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
        match s {
            "class" => Token::KwClass,
            "end" => Token::KwEnd,
            "def" => Token::KwDef,
            "and" => Token::KwAnd,
            "or" => Token::KwOr,
            "not" => Token::KwNot,
            "if" => Token::KwIf,
            "unless" => Token::KwUnless,
            "then" => Token::KwThen,
            "else" => Token::KwElse,
            "self" => Token::KwSelf,
            _ => Token::LowerWord(s.to_string()),
        }
    }

    fn read_symbol(&mut self, next_cur: &mut Cursor) -> Token {
        let c1 = next_cur.proceed(self.src);
        let c2 = next_cur.peek(self.src);
        match c1 {
            '(' => Token::LParen,
            ')' => Token::RParen,
            '[' => Token::LSqBracket,
            ']' => Token::RSqBracket,
            '<' => Token::LAngBracket,
            '>' => Token::RAngBracket,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '+' => Token::Plus,
            '-' => {
                if c2 == Some('>') {
                    next_cur.proceed(self.src);
                    Token::RightArrow
                }
                else {
                    Token::Minus
                }
            },
            '*' => Token::Mul,
            '/' => Token::Div,
            '%' => Token::Mod,
            '=' => {
                if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    Token::EqEq
                }
                else {
                    Token::Equal
                }
            },
            '!' => Token::Bang,
            '.' => Token::Dot,
            '@' => Token::At,
            '~' => Token::Tilde,
            '?' => Token::Question,
            ',' => Token::Comma,
            ':' => Token::Colon,
            '&' => {
                if c2 == Some('&') {
                    next_cur.proceed(self.src);
                    Token::AndAnd
                }
                else {
                    Token::And
                }
            },
            '|' => {
                if c2 == Some('|') {
                    next_cur.proceed(self.src);
                    Token::OrOr
                }
                else {
                    Token::Or
                }
            },
            c => {
                // TODO: this should be lexing error
                panic!("unknown symbol: {}", c)
            }
        }
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
