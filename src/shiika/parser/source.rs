use backtrace::Backtrace;
use super::location::Location;

// Represents a source file and a cursor in it
pub struct Source {
    pub filepath: String,
    pub src: String,
    pub location: Location,
    pos: usize,
}

impl Source {
    pub fn dummy(src: &str) -> Source {
        Source {
            filepath: "(dummy)".to_string(),
            src: src.to_string(),
            location: Location::new(),
            pos: 0,
        }
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

    pub fn skip_n(&mut self, n_chars: usize) {
        for _ in 1..n_chars {
            self.next().unwrap();
        }
    }

    pub fn starts_with(&self, s: &str) -> bool {
        self.src[self.pos..].starts_with(s)
    }

    pub fn read_ascii(&mut self, s: &str) {
        assert!(self.starts_with(s));
        self.skip_n(s.len())
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
        self.src[self.pos..].chars().next()
    }

    pub fn next(&mut self) -> Option<char> {
        let ret = self.src[self.pos..].chars().next();
        if let Some(c) = ret {
            self.pos += c.len_utf8();
            if c == '\n' {
                self.location.line += 1;
                self.location.col = 0
            }
            else {
                self.location.col += 1
            }
        }
        ret
    }

    fn parseerror(&self, msg: &str) -> super::ParseError {
        super::ParseError{
            msg: msg.to_string(),
            location: self.location.clone(),
            backtrace: Backtrace::new()
        }
    }
}

#[test]
fn test_source() {
    let mut source = Source::dummy("1+2");
    assert_eq!(source.peek(), Some('1'));
    assert_eq!(source.peek(), Some('1'));
    assert_eq!(source.location.col, 0);
    assert_eq!(source.next(), Some('1'));
    assert_eq!(source.location.col, 1);
    assert_eq!(source.next(), Some('+'));
    assert_eq!(source.location.col, 2);
}

#[test]
fn test_newline() {
    let mut source = Source::dummy("1\n2");
    source.next();
    source.next();
    assert_eq!(source.peek(), Some('2'));
    assert_eq!(source.location.line, 1);
    assert_eq!(source.location.col, 0);
}

#[test]
fn test_skip_ws() {
    let mut source = Source::dummy("a  b");
    source.next();
    source.skip_ws();
    assert_eq!(source.peek(), Some('b'));
}

#[test]
fn test_skip_comment() {
    let mut source = Source::dummy("#a  \nb");
    source.skip_comment();
    assert_eq!(source.peek(), Some('b'));
}
