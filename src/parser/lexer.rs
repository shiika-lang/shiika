use super::token::Token;

pub struct Lexer<'a: 'b, 'b: 'a> {
    pub src: &'a str,
    pub cur: Cursor,
    current_token: Option<Token<'b>>,
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

impl<'a, 'b> Lexer<'a, 'b> {
    pub fn new(src: &str) -> Lexer {
        Lexer {
            src: src,
            cur: Cursor::new(),
            next_cur: None,
            current_token: None,
        }
    }

    pub fn current_token(&mut self) -> &Token {
        if self.current_token == None {
            self.read_token();
        }
        self.current_token.as_ref().unwrap()
    }

    pub fn consume_token(&mut self) -> Token {
        assert!(self.current_token.is_some());
        self.cur = self.next_cur.take().unwrap();
        self.current_token.take().unwrap()
    }

    fn read_token(&mut self) {
        let cc = self.cur.peek(self.src);
        let mut next_cur = self.cur.clone();
        match self.char_type(cc) {
            CharType::Space     => self.read_space(&mut next_cur),
            CharType::Separator => self.read_separator(&mut next_cur),
            CharType::UpperWord => self.read_upper_word(&mut next_cur),
            CharType::LowerWord => self.read_lower_word(&mut next_cur),
            CharType::Symbol    => self.read_symbol(&mut next_cur),
            CharType::Number    => self.read_number(&mut next_cur),
            CharType::Eof       => self.read_eof(),
        }
        self.next_cur = Some(next_cur)
    }

    fn read_space(&mut self, next_cur: &mut Cursor) {
        loop {
            match self.char_type(next_cur.peek(self.src)) {
                CharType::Space => next_cur.proceed(self.src),
                _ => break
            };
        }
        self.current_token = Some(Token::Space);
    }

    fn read_separator(&mut self, next_cur: &mut Cursor) {
        loop {
            match self.char_type(next_cur.peek(self.src)) {
                CharType::Space | CharType::Separator => {
                    next_cur.proceed(self.src);
                },
                _ => break
            };
        }
        self.current_token = Some(Token::Separator);
    }

    fn read_upper_word(&mut self, next_cur: &mut Cursor) {
        loop {
            match self.char_type(next_cur.peek(self.src)) {
                CharType::UpperWord | CharType::LowerWord | CharType::Number => {
                    next_cur.proceed(self.src);
                },
                _ => break
            }
        }
        self.current_token = Some(Token::UpperWord(&self.src[self.cur.pos..next_cur.pos]));
    }

    fn read_lower_word(&mut self, next_cur: &mut Cursor) {
        loop {
            match self.char_type(next_cur.peek(self.src)) {
                CharType::UpperWord | CharType::LowerWord | CharType::Number => {
                    next_cur.proceed(self.src);
                },
                _ => break
            }
        }
        self.current_token = Some(Token::LowerWord(&self.src[self.cur.pos..next_cur.pos]));
    }

    fn read_symbol(&mut self, next_cur: &mut Cursor) {
        let c1 = next_cur.proceed(self.src);
        let c2 = next_cur.peek(self.src);
        match self.char_type(c2) {
            CharType::Symbol => {
                if c1 == '-' && c2 == Some('>') ||
                   c1 == '=' && c2 == Some('=') {
                    next_cur.proceed(self.src);
                }
            },
            _ => ()
        }
        self.current_token = Some(Token::Symbol(&self.src[self.cur.pos..next_cur.pos]));
    }

    fn read_number(&mut self, next_cur: &mut Cursor) {
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
        self.current_token = Some(Token::Number(&self.src[self.cur.pos..next_cur.pos]));
    }

    fn read_eof(&mut self) {
        self.current_token = Some(Token::Eof)
    }

    fn char_type(&self, cc: Option<char>) -> CharType {
        if cc == None {
            return CharType::Eof
        }
        match cc.unwrap() {
            ' ' | '\t' => CharType::Space,
            '\n' | ';' => CharType::Separator,
            '0'...'9' => CharType::Number,
            '(' | ')' | '[' | ']' | '<' | '>' | '{' | '}' |
            '+' | '-' | '*' | '/' | '%' | '=' | '!' |
            '.' | '@' | '~' | '?' | ',' | ':' => CharType::Symbol,
            'A'...'Z' => CharType::UpperWord,
            _ => CharType::LowerWord,
        }
    }
}
