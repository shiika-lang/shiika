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
