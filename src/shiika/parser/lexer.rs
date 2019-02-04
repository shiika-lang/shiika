pub struct Lexer<'a: 'b, 'b: 'a> {
    pub src: &'a str,
    pub cur: Cursor,
    current_token: Option<Token<'b>>,
    next_cur: Option<Cursor>,
}

#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    Space,
    Separator,
    Word(&'a str),
    Symbol(&'a str),
    Number(&'a str),
    Eof,
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

enum CharType {
    Space,
    Separator, // Newline or ';'
    Word, // Keyword or identifier
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
            CharType::Word      => self.read_word(&mut next_cur),
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

    fn read_word(&mut self, next_cur: &mut Cursor) {
        loop {
            match self.char_type(next_cur.peek(self.src)) {
                CharType::Word | CharType::Number => {
                    next_cur.proceed(self.src);
                },
                _ => break
            }
        }
        self.current_token = Some(Token::Word(&self.src[self.cur.pos..next_cur.pos]));
    }

    fn read_symbol(&mut self, next_cur: &mut Cursor) {
        loop {
            match self.char_type(next_cur.peek(self.src)) {
                CharType::Symbol => {
                    next_cur.proceed(self.src);
                },
                _ => break
            }
        }
        self.current_token = Some(Token::Symbol(&self.src[self.cur.pos..next_cur.pos]));
    }

    fn read_number(&mut self, next_cur: &mut Cursor) {
        loop {
            match self.char_type(next_cur.peek(self.src)) {
                CharType::Number => {
                    next_cur.proceed(self.src);
                },
                CharType::Word => {
                    // TODO: this should be lexing error
                    break
                },
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
            '.' | '@' | '~' | '?'  => CharType::Symbol,
            _ => CharType::Word,
        }
    }
}
