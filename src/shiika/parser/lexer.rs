pub struct Lexer<'a: 'b, 'b: 'a> {
    pub src: &'a str,
    pub cur: Cursor,
    pub current_token: Option<Token<'b>>,
    next_cur: Option<Cursor>,
}

#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    Word(&'a str),
    Symbol(char),
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

    pub fn peek_token(&mut self) {
        if self.current_token == None {
            self.read_token();
        }
    }

    fn read_token(&mut self) {
        let cc = self.cur.peek(self.src);
        match self.char_type(cc) {
//            CharType::Space => read_space
//            CharType::Newline =>
//            CharType::Word =>
//            CharType::Symbol =>
            CharType::Number => self.read_number(),
            CharType::Eof => {Token::Eof;},
            _ => {Token::Eof;},
        }
    }

    fn read_number(&mut self) {
        let mut next_cur = self.cur.clone();
        loop {
            let item = next_cur.peek(self.src);
            if item == None || !('0'..='9').contains(&item.unwrap()) {
                break
            }
            next_cur.proceed(self.src);
        }
        self.current_token = Some(Token::Number(&self.src[self.cur.pos..next_cur.pos]));
        self.next_cur = Some(next_cur);
    }

    fn char_type(&self, cc: Option<char>) -> CharType {
        if cc == None {
            return CharType::Eof
        }
        match cc.unwrap() {
            ' ' | '\t' => CharType::Space,
            '\n' | ';' => CharType::Separator,
            '0'...'9' => CharType::Number,
            '+' | '-' | '*' | '/' | '%' |
            '(' | ')' | '[' | ']' | '<' | '>' | '{' | '}' => CharType::Symbol,
            _ => CharType::Word,
        }
    }
}
