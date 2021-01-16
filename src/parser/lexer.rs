use super::token::Token;

#[derive(Debug)]
pub struct Lexer<'a> {
    pub src: &'a str,
    pub cur: Cursor,
    pub current_token: Token,
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
#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq, Clone, Default)]
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
    LowerWord, // Keyword or identifier which starts with lower-case letter
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
    /// let src = "+1";
    /// let mut lexer = Lexer::new(src);
    ///
    /// // Return the next token but does not move the position
    /// assert_eq!(lexer.peek_next(), Token::number("1"));
    /// assert_eq!(lexer.current_token, Token::UnaryPlus);
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
            CharType::Space => (self.read_space(&mut next_next_cur), None),
            CharType::Separator => (self.read_separator(&mut next_next_cur), None),
            CharType::Comment => (self.read_comment(&mut next_next_cur), None),
            CharType::UpperWord => (
                self.read_upper_word(&mut next_next_cur, Some(&next_cur)),
                None,
            ),
            CharType::LowerWord => self.read_lower_word(&mut next_next_cur, Some(&next_cur)),
            CharType::IVar => (self.read_ivar(&mut next_next_cur, Some(&next_cur)), None),
            CharType::Symbol => self.read_symbol(&mut next_next_cur),
            CharType::Number => (self.read_number(&mut next_next_cur, Some(&next_cur)), None),
            CharType::Str => (self.read_str(&mut next_next_cur, false), None),
            CharType::Eof => (self.read_eof(), None),
        };
        token
    }

    /// Read a token and set it to `current_token`
    #[allow(clippy::useless_let_if_seq)]
    fn read_token(&mut self) {
        let c = self.cur.peek(self.src);
        let mut next_cur = self.cur.clone();
        let token;
        let new_state;
        if self.state == LexerState::StrLiteral {
            token = self.read_str(&mut next_cur, true);
            new_state = None;
        } else {
            let (t, s) = match self.char_type(c) {
                CharType::Space => (self.read_space(&mut next_cur), None),
                CharType::Separator => (self.read_separator(&mut next_cur), None),
                CharType::Comment => (self.read_comment(&mut next_cur), None),
                CharType::UpperWord => (
                    self.read_upper_word(&mut next_cur, None),
                    Some(LexerState::ExprEnd),
                ),
                CharType::LowerWord => self.read_lower_word(&mut next_cur, None),
                CharType::IVar => (
                    self.read_ivar(&mut next_cur, None),
                    Some(LexerState::ExprEnd),
                ),
                CharType::Symbol => self.read_symbol(&mut next_cur),
                CharType::Number => (
                    self.read_number(&mut next_cur, None),
                    Some(LexerState::ExprEnd),
                ),
                CharType::Str => (
                    self.read_str(&mut next_cur, false),
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
        self.next_cur = Some(next_cur)
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

    fn read_lower_word(
        &mut self,
        next_cur: &mut Cursor,
        cur: Option<&Cursor>,
    ) -> (Token, Option<LexerState>) {
        loop {
            let c = next_cur.peek(self.src);
            match self.char_type(c) {
                CharType::UpperWord | CharType::LowerWord | CharType::Number => {
                    next_cur.proceed(self.src);
                }
                CharType::Symbol if (c == Some('=')) => {
                    if self.state == LexerState::MethodName {
                        next_cur.proceed(self.src);
                    }
                    break;
                }
                _ => break,
            }
        }
        let begin = match cur {
            Some(c) => c.pos,
            None => self.cur.pos,
        };
        let s = &self.src[begin..next_cur.pos];
        let (token, state) = match s {
            "class" => (Token::KwClass, LexerState::ExprBegin),
            "end" => (Token::KwEnd, LexerState::ExprEnd),
            "def" => (Token::KwDef, LexerState::ExprBegin),
            "var" => (Token::KwVar, LexerState::ExprBegin),
            "and" => (Token::KwAnd, LexerState::ExprBegin),
            "or" => (Token::KwOr, LexerState::ExprBegin),
            "not" => (Token::KwNot, LexerState::ExprBegin),
            "if" => (Token::KwIf, LexerState::ExprBegin),
            "unless" => (Token::KwUnless, LexerState::ExprBegin),
            "while" => (Token::KwWhile, LexerState::ExprBegin),
            "break" => (Token::KwBreak, LexerState::ExprEnd),
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
        let s = &self.src[(begin + 1)..next_cur.pos];
        Token::IVar(s.to_string())
    }

    fn read_symbol(&mut self, next_cur: &mut Cursor) -> (Token, Option<LexerState>) {
        let c1 = next_cur.proceed(self.src);
        let c2 = next_cur.peek(self.src);
        let (token, state) = match c1 {
            '(' => (Token::LParen, LexerState::ExprBegin),
            ')' => (Token::RParen, LexerState::ExprEnd),
            '[' => {
                if self.state == LexerState::MethodName && c2 == Some(']') {
                    next_cur.proceed(self.src);
                    let c3 = next_cur.peek(self.src);
                    if c3 == Some('=') {
                        next_cur.proceed(self.src);
                        (Token::SetMethod, LexerState::ExprBegin)
                    } else {
                        (Token::GetMethod, LexerState::ExprBegin)
                    }
                } else {
                    (Token::LSqBracket, LexerState::ExprBegin)
                }
            }
            ']' => (Token::RSqBracket, LexerState::ExprEnd),
            '{' => (Token::LBrace, LexerState::ExprBegin),
            '}' => (Token::RBrace, LexerState::ExprEnd),
            '+' => {
                if self.state == LexerState::MethodName && c2 == Some('@') {
                    next_cur.proceed(self.src);
                    (Token::UPlusMethod, LexerState::ExprBegin)
                } else if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    (Token::PlusEq, LexerState::ExprBegin)
                } else if self.is_unary(c2) {
                    (Token::UnaryPlus, LexerState::ExprBegin)
                } else {
                    (Token::BinaryPlus, LexerState::ExprBegin)
                }
            }
            '-' => {
                if self.state == LexerState::MethodName && c2 == Some('@') {
                    next_cur.proceed(self.src);
                    (Token::UMinusMethod, LexerState::ExprBegin)
                } else if c2 == Some('>') {
                    next_cur.proceed(self.src);
                    (Token::RightArrow, LexerState::ExprBegin)
                } else if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    (Token::MinusEq, LexerState::ExprBegin)
                } else if self.is_unary(c2) {
                    (Token::UnaryMinus, LexerState::ExprBegin)
                } else {
                    (Token::BinaryMinus, LexerState::ExprBegin)
                }
            }
            '*' => {
                if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    (Token::MulEq, LexerState::ExprBegin)
                } else {
                    (Token::Mul, LexerState::ExprBegin)
                }
            }
            '/' => {
                if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    (Token::DivEq, LexerState::ExprBegin)
                } else {
                    (Token::Div, LexerState::ExprBegin)
                }
            }
            '%' => {
                if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    (Token::ModEq, LexerState::ExprBegin)
                } else {
                    (Token::Mod, LexerState::ExprBegin)
                }
            }
            '=' => {
                if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    (Token::EqEq, LexerState::ExprBegin)
                } else {
                    (Token::Equal, LexerState::ExprBegin)
                }
            }
            '!' => {
                if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    (Token::NotEq, LexerState::ExprBegin)
                } else {
                    (Token::Bang, LexerState::ExprBegin)
                }
            }
            '<' => {
                if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    (Token::LessEq, LexerState::ExprBegin)
                } else if c2 == Some('<') {
                    next_cur.proceed(self.src);
                    let c3 = next_cur.peek(self.src);
                    if c3 == Some('=') {
                        next_cur.proceed(self.src);
                        (Token::LShiftEq, LexerState::ExprBegin)
                    } else {
                        (Token::LShift, LexerState::ExprBegin)
                    }
                } else {
                    (Token::LessThan, LexerState::ExprBegin)
                }
            }
            '>' => {
                if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    (Token::GreaterEq, LexerState::ExprBegin)
                } else if c2 == Some('>') {
                    if self.rshift_is_gtgt {
                        // Don't make it RShift (eg. `Array<Array<Int>>`)
                        (Token::GreaterThan, LexerState::ExprBegin)
                    } else {
                        next_cur.proceed(self.src);
                        let c3 = next_cur.peek(self.src);
                        if c3 == Some('=') {
                            next_cur.proceed(self.src);
                            (Token::RShiftEq, LexerState::ExprBegin)
                        } else {
                            (Token::RShift, LexerState::ExprBegin)
                        }
                    }
                } else {
                    (Token::GreaterThan, LexerState::ExprBegin)
                }
            }
            '.' => (Token::Dot, LexerState::ExprBegin),
            '@' => (Token::At, LexerState::ExprBegin),
            '~' => (Token::Tilde, LexerState::ExprBegin),
            '?' => (Token::Question, LexerState::ExprBegin),
            ',' => (Token::Comma, LexerState::ExprBegin),
            ':' => {
                if c2 == Some(':') {
                    next_cur.proceed(self.src);
                    (Token::ColonColon, LexerState::ExprBegin)
                } else {
                    (Token::Colon, LexerState::ExprBegin)
                }
            }
            '&' => {
                if c2 == Some('&') {
                    next_cur.proceed(self.src);
                    let c3 = next_cur.peek(self.src);
                    if c3 == Some('=') {
                        next_cur.proceed(self.src);
                        (Token::AndAndEq, LexerState::ExprBegin)
                    } else {
                        (Token::AndAnd, LexerState::ExprBegin)
                    }
                } else if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    (Token::AndEq, LexerState::ExprBegin)
                } else {
                    (Token::And, LexerState::ExprBegin)
                }
            }
            '|' => {
                if c2 == Some('|') {
                    next_cur.proceed(self.src);
                    let c3 = next_cur.peek(self.src);
                    if c3 == Some('=') {
                        next_cur.proceed(self.src);
                        (Token::OrOrEq, LexerState::ExprBegin)
                    } else {
                        (Token::OrOr, LexerState::ExprBegin)
                    }
                } else if c2 == Some('=') {
                    next_cur.proceed(self.src);
                    (Token::OrEq, LexerState::ExprBegin)
                } else {
                    (Token::Or, LexerState::ExprBegin)
                }
            }
            '^' => (Token::Xor, LexerState::ExprBegin),
            c => {
                // TODO: this should be lexing error
                panic!("unknown symbol: {}", c)
            }
        };
        (token, Some(state))
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

    fn read_number(&mut self, next_cur: &mut Cursor, cur: Option<&Cursor>) -> Token {
        loop {
            match self.char_type(next_cur.peek(self.src)) {
                CharType::Number => {
                    next_cur.proceed(self.src);
                }
                CharType::UpperWord | CharType::LowerWord => {
                    // TODO: this should be lexing error
                    panic!("need space after a number")
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
        Token::Number(self.src[begin..next_cur.pos].to_string())
    }

    /// Read a string literal
    /// Also parse escape sequences here
    /// - cont: true if reading string after `#{}'
    fn read_str(&mut self, next_cur: &mut Cursor, cont: bool) -> Token {
        let mut buf = String::new();
        if !cont {
            // Consume the beginning `"'
            next_cur.proceed(self.src);
        }
        loop {
            match next_cur.peek(self.src) {
                None => {
                    // TODO: should be a LexError
                    panic!("found unterminated string");
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
                        return Token::StrWithInterpolation {
                            head: buf,
                            inspect: true,
                        };
                    } else {
                        let c = self._read_escape_sequence(next_cur.peek(self.src));
                        next_cur.proceed(self.src);
                        buf.push(c);
                    }
                }
                Some('#') => {
                    next_cur.proceed(self.src);
                    let c2 = next_cur.peek(self.src);
                    if c2 == Some('{') {
                        next_cur.proceed(self.src);
                        return Token::StrWithInterpolation {
                            head: buf,
                            inspect: false,
                        };
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
        Token::Str(buf)
    }

    /// Return special char written with '\'
    fn _read_escape_sequence(&self, c: Option<char>) -> char {
        match c {
            None => {
                // TODO: should be a LexError
                panic!("found unterminated string");
            }
            Some('\\') => '\\',
            Some('"') => '"',
            Some('n') => '\n',
            Some('t') => '\t',
            Some('r') => '\r',
            Some(c) => c,
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
}
