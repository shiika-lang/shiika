//use std::collections::HashSet;
use backtrace::Backtrace;
use super::location::Location;
use super::token::Token;

// Represents a source file and a cursor in it
pub struct Source<'a> {
    pub filepath: String,
    pub src: String,
    pub loc: Location,
    pos: usize,
    current_token: Option<Token<'a>>,
    next_loc: Option<Location>,
}

const SYMBOLS: [char;13] = [
    '+', '-', '*', '/', '%',
    '(', ')', '[', ']', '<', '>', '{', '}', 
];

impl<'a> Source<'a> {
    pub fn dummy(src: &str) -> Source {
        Source {
            filepath: "(dummy)".to_string(),
            src: src.to_string(),
            loc: Location::new(),
            pos: 0,
            current_token: None,
            next_loc: None,
        }
    }

    pub fn require_ident(&mut self) -> Result<String, super::ParseError> {
        let mut str = String::new();
        match self.peek_char()? {
            'a'...'z' => (),
            _ => return Err(self.parseerror("expected ident"))
        }
        str.push(self.next().unwrap());

        loop {
            match self.peek() {
                Some(c) => {
                    match c {
                        'a'...'z' | '0'...'9' => str.push(self.next().unwrap()),
                        _ => break
                    }
                },
                _ => break
            }
        }
        Ok(str)
    }


    pub fn require_ascii(&mut self, word: &str) -> Result<(), super::ParseError> {
        if self.starts_with(word) {
            self.read_ascii(word);
            Ok(())
        }
        else {
            Err(self.parseerror("expected #{word}"))
        }
    }

    // Skip a separator (`;' or newline). Return error if none
    pub fn require_sep(&mut self) -> Result<(), super::ParseError> {
        loop {
            match self.next() {
                Some(';') | Some('\n') => break,
                Some(' ') => (),
                Some('#') => {self.skip_comment(); break},
                _ => return Err(self.parseerror("missing separator")),
            }
        }
        Ok(())
    }

    // Skip whitespace and tab
    pub fn skip_ws(&mut self) {
        loop {
            match self.peek() {
                Some(' ') | Some('\t') => self.next(),
                _ => break
            };
        }
    }

    // Skip whitespace, tab, newline and comments
    pub fn skip_wsn(&mut self) {
        loop {
            match self.peek() {
                Some(' ') | Some('\t') | Some('\n') => {self.next();},
                Some('#') => self.skip_comment(),
                _ => break
            }
        }
    }

    // Skip comments (must be called at the '#')
    pub fn skip_comment(&mut self) {
        assert_eq!(Some('#'), self.next());
        loop {
            match self.next() {
                Some('\n') | None => break,
                _ => ()
            }
        }
    }

    pub fn starts_with(&self, s: &str) -> bool {
        self.src[self.pos..].starts_with(s)
    }

    pub fn read_ascii(&mut self, s: &str) {
        assert!(self.starts_with(s));
        self.skip_n(s.len())
    }

    fn skip_n(&mut self, n_chars: usize) {
        for _ in 0..n_chars {
            self.next().unwrap();
        }
    }

    // Returns a token from current position. Does not consume input
    // Returns None if EOF
    pub fn peek_token(&mut self) -> Option<Token> {
       let ret;
       match self.peek() {
           None => None,
           Some(c) => {
               let mut loc = self.loc.clone();
               ret = if SYMBOLS.contains(&c) {
                         self.proceed(&mut loc);
                         Token::Symbol(c)
                     }
                     else {
                         match c {
                             '0'...'9' => self.read_number(&mut loc),
                             _ => Token::Symbol('_') // self.read_word(&mut loc),
                         }
                     };
               self.current_token = Some(ret);
               self.next_loc = Some(loc);
               self.current_token
           }
       }
    }

//    fn read_word(&mut self, _loc: &mut Location) -> Token {
//        Token::Word("asdf")
//    }

    // Read a number at the given location
    fn read_number(&mut self, mut loc: &mut Location) -> Token {
        let mut end = loc.pos;
        loop {
            let item = self.peek_at(loc.pos);
            if item == None || !('0'..='9').contains(&item.unwrap()) {
                break
            }
            let c = self.proceed(&mut loc);
            end += c.len_utf8();
        }
        Token::Number(&self.src[loc.pos..end])
    }

    fn proceed(&self, loc: &mut Location) -> char {
        let c = self.src[loc.pos..].chars().next().unwrap();
        loc.pos += c.len_utf8();
        if c == '\n' {
            loc.line += 1;
            loc.col = 0
        }
        else {
            loc.col += 1
        }
        c
    }

    fn peek_at(&mut self, pos: usize) -> Option<char> {
        self.src[pos..].chars().next()
    }

    // Consume a token
    pub fn next_token(&mut self) -> Result<Token, super::ParseError> {
        if self.current_token == None {
            if self.peek_token() == None {
                return Err(self.parseerror("unexpected EOF"))
            }
        }
        self.loc = self.next_loc.unwrap();
        Ok(self.current_token.unwrap())
    }

    pub fn peek_char(&mut self) -> Result<char, super::ParseError> {
        match self.peek() {
            Some(c) => Ok(c),
            None => Err(self.parseerror("unexpected EOF"))
        }
    }

    pub fn next_char(&mut self) -> Result<char, super::ParseError> {
        match self.next() {
            Some(c) => Ok(c),
            None => Err(self.parseerror("unexpected EOF"))
        }
    }

    pub fn peek(&mut self) -> Option<char> {
        self.peek_at(self.pos)
    }

    pub fn next(&mut self) -> Option<char> {
        let ret = self.src[self.pos..].chars().next();
        if let Some(c) = ret {
            self.pos += c.len_utf8();
            if c == '\n' {
                self.loc.line += 1;
                self.loc.col = 0
            }
            else {
                self.loc.col += 1
            }
        }
        ret
    }

    fn parseerror(&self, msg: &str) -> super::ParseError {
        super::ParseError{
            msg: msg.to_string(),
            loc: self.loc.clone(),
            backtrace: Backtrace::new()
        }
    }
}
