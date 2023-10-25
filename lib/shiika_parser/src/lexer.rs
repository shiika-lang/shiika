use crate::error::Error;
use shiika_ast::{Location, Token};

/// Lexer
#[derive(Debug)]
pub struct Lexer<'a> {
    /// Reference to source code.
    pub src: &'a str,
    /// Current position
    pub cur: Cursor,
    /// A token starts from `cur`
    pub current_token: Token,
    /// Next position when `current_token` is consumed
    next_cur: Option<Cursor>,
    /// Flag to decide +/- etc. is unary or binary
    state: LexerState,
    /// true if the last token is a space
    space_seen: bool,
    /// If true, parse `>>` as `>` + `>`
    pub rshift_is_gtgt: bool,
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
#[derive(Debug, PartialEq, Eq)]
pub enum LexerState {
    /// A new expression begins here
    /// `+`/`-` is always unary
    ExprBegin,
    /// End of an expression
    /// `+`/`-` is always binary
    ExprEnd,
    /// Beginning of a (possible) first paren-less arg of a method call.
    /// `+`/`-` is unary, if with space before it and no space after it (`p -x`)
    ExprArg,

    // Special states
    /// Expects a method name
    /// eg. `+@`, `-@` is allowed only in this state
    MethodName,
    /// In a string literal (with interpolation)
    StrLiteral,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
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

    /// Return the current char (None if eof)
    pub fn peek(&self, src: &str) -> Option<char> {
        src[self.pos..].chars().next()
    }

    /// Peek the second next character.
    /// Must not be called on EOF
    pub fn peek2(&self, src: &str) -> Option<char> {
        if let Some(c) = self.peek(src) {
            let pos = self.pos + c.len_utf8();
            src[pos..].chars().next()
        } else {
            panic!("peek2 must not be called on EOF")
        }
    }

    pub fn peek_n(&self, src: &str, n: usize) -> String {
        src[self.pos..].chars().take(n).collect()
    }

    /// Consume the current char and return it
    pub fn proceed(&mut self, src: &str) -> char {
        let c = src[self.pos..].chars().next().unwrap();
        if c == '\n' {
            self.line += 1;
            self.col = 0
        } else {
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
    Comment,   // From '#' to the next newline
    UpperWord, // identifier which starts with upper-case letter
    LowerWord, // Keyword or identifier which starts with lower-case letter. May suffixed by '?'
    IVar,      // Instance variable (eg. "foo" for @foo)
    Symbol,    // '+', '(', etc.
    Number,    // '0'~'9'
    Str,       // '"'
    Eof,
}

impl<'a> Lexer<'a> {
    /// Create lexer and get the first token
    pub fn new(src: &str) -> Lexer {
        Lexer::new_with_state(src, LexerState::ExprBegin)
    }

    pub fn new_with_state(src: &str, state: LexerState) -> Lexer {
        let mut lexer = Lexer {
            src,
            cur: Cursor::new(),
            next_cur: None,
            current_token: Token::Bof,
            state,
            space_seen: false,
            rshift_is_gtgt: false,
        };
        lexer.read_token().unwrap();
        lexer
    }

    pub fn set_state(&mut self, state: LexerState) {
        self.state = state;
    }

    fn set_current_token(&mut self, token: Token) {
        self.space_seen = self.current_token == Token::Space;
        self.current_token = token;
    }

    /// Returns the current location.
    pub fn location(&self) -> Location {
        Location::new(self.cur.line, self.cur.col, self.cur.pos)
    }

    /// Returns pair of locations which are the beginning and the end of the
    /// current token.
    pub fn location_span(&self) -> (Location, Location) {
        let begin = Location::new(self.cur.line, self.cur.col, self.cur.pos);
        let nc = self.next_cur.as_ref().unwrap();
        let end = Location::new(nc.line, nc.col, nc.pos);
        (begin, end)
    }

    pub fn debug_info(&self) -> String {
        format!("{:?} {:?}", self.current_token, self.state)
    }

    pub fn peek_n(&self, n: usize) -> String {
        self.cur.peek_n(self.src, n)
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
    pub fn consume_token(&mut self) -> Result<Token, Error> {
        self.cur = self.next_cur.take().unwrap();
        let tok = self.current_token.clone(); // PERF: how not to clone?
        self.read_token()?;
        Ok(tok)
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
    pub fn set_position(&mut self, cur: Cursor) -> Result<(), Error> {
        self.cur = cur;
        self.read_token()
    }

    /// Return the next token while keeping the current one
    ///
    /// # Examples
    ///
    /// ```
    /// use shiika::parser::lexer::Lexer;
    /// use shiika::parser::token::Token;
    ///
    /// let src = "+1";
    /// let mut lexer = Lexer::new(src);
    ///
    /// // Return the next token but does not move the position
    /// assert_eq!(lexer.peek_next().unwrap(), Token::number("1"));
    /// assert_eq!(lexer.current_token, Token::UnaryPlus);
    ///
    /// // Return Eof when called on the end
    /// lexer.consume_token().unwrap();
    /// lexer.consume_token().unwrap();
    /// assert_eq!(lexer.peek_next().unwrap(), Token::Eof);
    /// ```
    pub fn peek_next(&mut self) -> Result<Token, Error> {
        let next_cur = self.next_cur.as_ref().unwrap().clone();
        let c = next_cur.peek(self.src);
        let mut next_next_cur = next_cur.clone();
        let (token, _) = match self.char_type(c) {
            CharType::Space => (self.read_space(&mut next_next_cur), None),
            CharType::Separator => (self.read_separator(&mut next_next_cur), None),
            CharType::Comment => (self.read_comment(&mut next_next_cur), None),
            CharType::UpperWord => (
                self.read_upper_word(&mut next_next_cur, Some(&next_cur)),
                None,
            ),
            CharType::LowerWord => self.read_lower_word(&mut next_next_cur, Some(&next_cur)),
            CharType::IVar => (self.read_ivar(&mut next_next_cur, Some(&next_cur)), None),
            CharType::Symbol => self.read_symbol(&mut next_next_cur)?,
            CharType::Number => (self.read_number(&mut next_next_cur, Some(&next_cur))?, None),
            CharType::Str => (self.read_str(&mut next_next_cur, false)?, None),
            CharType::Eof => (self.read_eof(), None),
        };
        Ok(token)
    }

    /// Read a token and set it to `current_token`
    #[allow(clippy::useless_let_if_seq)]
    fn read_token(&mut self) -> Result<(), Error> {
        let c = self.cur.peek(self.src);
        let mut next_cur = self.cur.clone();
        let token;
        let new_state;
        if self.state == LexerState::StrLiteral {
            token = self.read_str(&mut next_cur, true)?;
            new_state = None;
        } else {
            let (t, s) = match self.char_type(c) {
                CharType::Space => (self.read_space(&mut next_cur), None),
                CharType::Separator => (
                    self.read_separator(&mut next_cur),
                    Some(LexerState::ExprBegin),
                ),
                CharType::Comment => (
                    self.read_comment(&mut next_cur),
                    Some(LexerState::ExprBegin),
                ),
                CharType::UpperWord => (
                    self.read_upper_word(&mut next_cur, None),
                    Some(LexerState::ExprEnd),
                ),
                CharType::LowerWord => self.read_lower_word(&mut next_cur, None),
                CharType::IVar => (
                    self.read_ivar(&mut next_cur, None),
                    Some(LexerState::ExprEnd),
                ),
                CharType::Symbol => self.read_symbol(&mut next_cur)?,
                CharType::Number => (
                    self.read_number(&mut next_cur, None)?,
                    Some(LexerState::ExprEnd),
                ),
                CharType::Str => (
                    self.read_str(&mut next_cur, false)?,
                    Some(LexerState::ExprEnd),
                ),
                CharType::Eof => (self.read_eof(), None),
            };
            token = t;
            new_state = s;
        }
        self.set_current_token(token);
        if let Some(state) = new_state {
            self.state = state;
        }
        self.next_cur = Some(next_cur);
        Ok(())
    }

    fn read_space(&mut self, next_cur: &mut Cursor) -> Token {
        while let CharType::Space = self.char_type(next_cur.peek(self.src)) {
            next_cur.proceed(self.src);
        }
        Token::Space
    }

    fn read_separator(&mut self, next_cur: &mut Cursor) -> Token {
        while let CharType::Space | CharType::Separator = self.char_type(next_cur.peek(self.src)) {
            next_cur.proceed(self.src);
        }
        Token::Separator
    }

    fn read_comment(&mut self, next_cur: &mut Cursor) -> Token {
        next_cur.proceed(self.src); // Skip the `#'
        loop {
            let c = next_cur.proceed(self.src);
            if c == '\n' {
                break;
            }
        }
        Token::Separator
    }

    fn read_upper_word(&mut self, next_cur: &mut Cursor, cur: Option<&Cursor>) -> Token {
        while let CharType::UpperWord | CharType::LowerWord | CharType::Number =
            self.char_type(next_cur.peek(self.src))
        {
            next_cur.proceed(self.src);
        }
        let begin = match cur {
            Some(c) => c.pos,
            None => self.cur.pos,
        };
        Token::UpperWord(self.src[begin..next_cur.pos].to_string())
    }

    // Read either of
    // - an identifier starting with a small letter
    //   - May be suffixed by a `?`
    //   - May be suffixed by a `!`
    //   - May be suffixed by a `=` when lexer state is LexerState::MethodName.
    // - a keyword (`if`, `class`, etc.)
    // - a KeyName (`foo:`, etc.).
    fn read_lower_word(
        &mut self,
        next_cur: &mut Cursor,
        cur: Option<&Cursor>,
    ) -> (Token, Option<LexerState>) {
        let begin = match cur {
            Some(c) => c.pos,
            None => self.cur.pos,
        };
        loop {
            let c = next_cur.peek(self.src);
            match self.char_type(c) {
                CharType::UpperWord | CharType::LowerWord | CharType::Number => {
                    // These are allowed in an identifier.
                    next_cur.proceed(self.src);
                }
                CharType::Symbol => {
                    if c == Some(':') {
                        let s = &self.src[begin..next_cur.pos];
                        next_cur.proceed(self.src);
                        return (Token::KeyName(s.to_string()), Some(LexerState::ExprBegin));
                    } else if c == Some('?')
                        || c == Some('!')
                        || (c == Some('=') && self.state == LexerState::MethodName)
                    {
                        next_cur.proceed(self.src);
                        break;
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        let s = &self.src[begin..next_cur.pos];
        let (token, state) = match s {
            "require" => (Token::KwRequire, LexerState::ExprBegin),
            "class" => (Token::KwClass, LexerState::ExprBegin),
            "module" => (Token::KwModule, LexerState::ExprBegin),
            "requirement" => (Token::KwRequirement, LexerState::ExprBegin),
            "enum" => (Token::KwEnum, LexerState::ExprBegin),
            "case" => (Token::KwCase, LexerState::ExprBegin),
            "in" => (Token::KwIn, LexerState::ExprBegin),
            "out" => (Token::KwOut, LexerState::ExprBegin),
            "end" => (Token::KwEnd, LexerState::ExprEnd),
            "def" => (Token::KwDef, LexerState::ExprBegin),
            "let" => (Token::KwLet, LexerState::ExprBegin),
            "var" => (Token::KwVar, LexerState::ExprBegin),
            "and" => (Token::KwAnd, LexerState::ExprBegin),
            "or" => (Token::KwOr, LexerState::ExprBegin),
            "not" => (Token::KwNot, LexerState::ExprBegin),
            "if" => {
                if self.state == LexerState::ExprBegin {
                    (Token::KwIf, LexerState::ExprBegin)
                } else {
                    (Token::ModIf, LexerState::ExprBegin)
                }
            }
            "unless" => {
                if self.state == LexerState::ExprBegin {
                    (Token::KwUnless, LexerState::ExprBegin)
                } else {
                    (Token::ModUnless, LexerState::ExprBegin)
                }
            }
            "match" => (Token::KwMatch, LexerState::ExprBegin),
            "when" => (Token::KwWhen, LexerState::ExprBegin),
            "while" => (Token::KwWhile, LexerState::ExprBegin),
            "break" => (Token::KwBreak, LexerState::ExprEnd),
            "return" => (Token::KwReturn, LexerState::ExprBegin),
            "then" => (Token::KwThen, LexerState::ExprBegin),
            "else" => (Token::KwElse, LexerState::ExprBegin),
            "elsif" => (Token::KwElsif, LexerState::ExprBegin),
            "fn" => (Token::KwFn, LexerState::ExprBegin),
            "do" => (Token::KwDo, LexerState::ExprBegin),
            "self" => (Token::KwSelf, LexerState::ExprEnd),
            "true" => (Token::KwTrue, LexerState::ExprEnd),
            "false" => (Token::KwFalse, LexerState::ExprEnd),
            _ => (Token::LowerWord(s.to_string()), LexerState::ExprEnd),
        };
        (token, Some(state))
    }

    /// Read `@foo`
    fn read_ivar(&mut self, next_cur: &mut Cursor, cur: Option<&Cursor>) -> Token {
        next_cur.proceed(self.src); // Skip '@'
                                    // TODO: First character must not be a number
        while let CharType::UpperWord | CharType::LowerWord | CharType::Number =
            self.char_type(next_cur.peek(self.src))
        {
            next_cur.proceed(self.src);
        }
        // TODO: LexError if no word succeeds '@'
        let begin = match cur {
            Some(c) => c.pos,
            None => self.cur.pos,
        };
        let s = &self.src[begin..next_cur.pos];
        Token::IVar(s.to_string())
    }

    fn read_symbol(&mut self, next_cur: &mut Cursor) -> Result<(Token, Option<LexerState>), Error> {
        let c1 = next_cur.proceed(self.src);
        let c2 = next_cur.peek(self.src);
        match c1 {
            '(' => Ok((Token::LParen, Some(LexerState::ExprBegin))),
            ')' => Ok((Token::RParen, Some(LexerState::ExprEnd))),
            '[' => {
                if self.state == LexerState::MethodName && c2 == Some(']') {
                    next_cur.proceed(self.src);
                    let c3 = next_cur.peek(self.src);
                    if c3 == Some('=') {
                        next_cur.proceed(self.src);
                        Ok((Token::SetMethod, Some(LexerState::ExprBegin)))
                    } else {
                        Ok((Token::GetMethod, Some(LexerState::ExprBegin)))
                    }
                } else {
                    Ok((Token::LSqBracket, Some(LexerState::ExprBegin)))
                }
            }
            ']' => Ok((Token::RSqBracket, Some(LexerState::ExprEnd))),
            '{' => Ok((Token::LBrace, Some(LexerState::ExprBegin))),
            '}' => Ok((Token::RBrace, Some(LexerState::ExprEnd))),
            '+' => {
                if self.state == LexerState::MethodName && c2 == Some('@') {
                    next_cur.proceed(self.src);
                    Ok((Token::UPlusMethod, Some(LexerState::ExprBegin)))
                } else if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    Ok((Token::PlusEq, Some(LexerState::ExprBegin)))
                } else if self.is_unary(c2) {
                    Ok((Token::UnaryPlus, Some(LexerState::ExprBegin)))
                } else {
                    Ok((Token::BinaryPlus, Some(LexerState::ExprBegin)))
                }
            }
            '-' => {
                if self.state == LexerState::MethodName && c2 == Some('@') {
                    next_cur.proceed(self.src);
                    Ok((Token::UMinusMethod, Some(LexerState::ExprBegin)))
                } else if c2 == Some('>') {
                    next_cur.proceed(self.src);
                    Ok((Token::RightArrow, Some(LexerState::ExprBegin)))
                } else if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    Ok((Token::MinusEq, Some(LexerState::ExprBegin)))
                } else if self.is_unary(c2) {
                    Ok((Token::UnaryMinus, Some(LexerState::ExprBegin)))
                } else {
                    Ok((Token::BinaryMinus, Some(LexerState::ExprBegin)))
                }
            }
            '*' => {
                if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    Ok((Token::MulEq, Some(LexerState::ExprBegin)))
                } else {
                    Ok((Token::Mul, Some(LexerState::ExprBegin)))
                }
            }
            '/' => {
                if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    Ok((Token::DivEq, Some(LexerState::ExprBegin)))
                } else {
                    Ok((Token::Div, Some(LexerState::ExprBegin)))
                }
            }
            '%' => {
                if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    Ok((Token::ModEq, Some(LexerState::ExprBegin)))
                } else {
                    Ok((Token::Mod, Some(LexerState::ExprBegin)))
                }
            }
            '=' => {
                if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    Ok((Token::EqEq, Some(LexerState::ExprBegin)))
                } else {
                    Ok((Token::Equal, Some(LexerState::ExprBegin)))
                }
            }
            '!' => {
                if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    Ok((Token::NotEq, Some(LexerState::ExprBegin)))
                } else {
                    Ok((Token::Bang, Some(LexerState::ExprBegin)))
                }
            }
            '<' => {
                if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    Ok((Token::LessEq, Some(LexerState::ExprBegin)))
                } else if c2 == Some('<') {
                    next_cur.proceed(self.src);
                    let c3 = next_cur.peek(self.src);
                    if c3 == Some('=') {
                        next_cur.proceed(self.src);
                        Ok((Token::LShiftEq, Some(LexerState::ExprBegin)))
                    } else {
                        Ok((Token::LShift, Some(LexerState::ExprBegin)))
                    }
                } else if c2 == Some('>') {
                    next_cur.proceed(self.src);
                    Ok((Token::Specialize, Some(LexerState::ExprBegin)))
                } else {
                    Ok((Token::LessThan, Some(LexerState::ExprBegin)))
                }
            }
            '>' => {
                if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    Ok((Token::GreaterEq, Some(LexerState::ExprBegin)))
                } else if c2 == Some('>') {
                    if self.rshift_is_gtgt {
                        // Don't make it RShift (eg. `Array<Array<Int>>`)
                        Ok((Token::GreaterThan, Some(LexerState::ExprBegin)))
                    } else {
                        next_cur.proceed(self.src);
                        let c3 = next_cur.peek(self.src);
                        if c3 == Some('=') {
                            next_cur.proceed(self.src);
                            Ok((Token::RShiftEq, Some(LexerState::ExprBegin)))
                        } else {
                            Ok((Token::RShift, Some(LexerState::ExprBegin)))
                        }
                    }
                } else {
                    Ok((Token::GreaterThan, Some(LexerState::ExprBegin)))
                }
            }
            '.' => Ok((Token::Dot, Some(LexerState::ExprBegin))),
            '@' => Ok((Token::At, Some(LexerState::ExprBegin))),
            '~' => Ok((Token::Tilde, Some(LexerState::ExprBegin))),
            '?' => Ok((Token::Question, Some(LexerState::ExprBegin))),
            ',' => Ok((Token::Comma, Some(LexerState::ExprBegin))),
            ':' => {
                if c2 == Some(':') {
                    next_cur.proceed(self.src);
                    Ok((Token::ColonColon, Some(LexerState::ExprBegin)))
                } else {
                    Ok((Token::Colon, Some(LexerState::ExprBegin)))
                }
            }
            '&' => {
                if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    Ok((Token::AndEq, Some(LexerState::ExprBegin)))
                } else {
                    Ok((Token::And, Some(LexerState::ExprBegin)))
                }
            }
            '|' => {
                if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    Ok((Token::OrEq, Some(LexerState::ExprBegin)))
                } else {
                    Ok((Token::Or, Some(LexerState::ExprBegin)))
                }
            }
            '^' => Ok((Token::Xor, Some(LexerState::ExprBegin))),
            c => Err(self.lex_error(&format!("unknown symbol: {}", c))),
        }
    }

    fn is_unary(&self, next_char: Option<char>) -> bool {
        match self.state {
            LexerState::ExprBegin => true,
            LexerState::ExprEnd => false,
            LexerState::ExprArg => self.current_token == Token::Space && next_char != Some(' '),

            // is_unary does not make sense at these states. Just return false
            LexerState::MethodName => false,
            LexerState::StrLiteral => false,
        }
    }

    fn read_number(&mut self, next_cur: &mut Cursor, cur: Option<&Cursor>) -> Result<Token, Error> {
        loop {
            match self.char_type(next_cur.peek(self.src)) {
                CharType::Number => {
                    next_cur.proceed(self.src);
                }
                CharType::UpperWord | CharType::LowerWord => {
                    // TODO: this should be lexing error
                    return Err(self.lex_error("need space after a number"));
                }
                CharType::Symbol => {
                    if next_cur.peek(self.src) == Some('.') {
                        if self.char_type(next_cur.peek2(self.src)) == CharType::Number {
                            next_cur.proceed(self.src);
                            next_cur.proceed(self.src);
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        let begin = match cur {
            Some(c) => c.pos,
            None => self.cur.pos,
        };
        Ok(Token::Number(self.src[begin..next_cur.pos].to_string()))
    }

    /// Read a string literal
    /// Also parse escape sequences here
    /// - cont: true if reading string after `#{}'
    fn read_str(&mut self, next_cur: &mut Cursor, cont: bool) -> Result<Token, Error> {
        let mut buf = String::new();
        if !cont {
            // Consume the beginning `"'
            next_cur.proceed(self.src);
        }
        loop {
            match next_cur.peek(self.src) {
                None => {
                    return Err(self.lex_error("found unterminated string"));
                }
                Some('"') => {
                    next_cur.proceed(self.src);
                    break;
                }
                Some('\\') => {
                    next_cur.proceed(self.src);
                    let c2 = next_cur.peek(self.src);
                    if c2 == Some('{') {
                        next_cur.proceed(self.src);
                        return Ok(Token::StrWithInterpolation {
                            head: buf,
                            inspect: true,
                        });
                    } else {
                        let c = self._read_escape_sequence(next_cur.peek(self.src))?;
                        next_cur.proceed(self.src);
                        buf.push(c);
                    }
                }
                Some('#') => {
                    next_cur.proceed(self.src);
                    let c2 = next_cur.peek(self.src);
                    if c2 == Some('{') {
                        next_cur.proceed(self.src);
                        return Ok(Token::StrWithInterpolation {
                            head: buf,
                            inspect: false,
                        });
                    } else {
                        buf.push('#');
                    }
                }
                Some(c) => {
                    next_cur.proceed(self.src);
                    buf.push(c);
                }
            }
        }
        Ok(Token::Str(buf))
    }

    /// Return special char written with '\'
    fn _read_escape_sequence(&self, c: Option<char>) -> Result<char, Error> {
        match c {
            None => Err(self.lex_error("found unterminated string")),
            Some('\\') => Ok('\\'),
            Some('"') => Ok('"'),
            Some('n') => Ok('\n'),
            Some('t') => Ok('\t'),
            Some('r') => Ok('\r'),
            Some(c) => Ok(c),
        }
    }

    fn read_eof(&mut self) -> Token {
        Token::Eof
    }

    fn char_type(&self, cc: Option<char>) -> CharType {
        if cc == None {
            return CharType::Eof;
        }
        match cc.unwrap() {
            ' ' | '\t' => CharType::Space,
            '\n' | ';' => CharType::Separator,
            '#' => CharType::Comment,
            '"' => CharType::Str,
            '0'..='9' => CharType::Number,
            '@' => CharType::IVar,
            '(' | ')' | '[' | ']' | '<' | '>' | '{' | '}' | '+' | '-' | '*' | '/' | '%' | '='
            | '!' | '^' | '.' | '~' | '?' | ',' | ':' | '|' | '&' => CharType::Symbol,
            'A'..='Z' => CharType::UpperWord,
            _ => CharType::LowerWord,
        }
    }

    fn lex_error(&self, msg: &str) -> Error {
        Error::LexError {
            msg: msg.to_string(),
            location: self.cur.clone(),
        }
    }
}
