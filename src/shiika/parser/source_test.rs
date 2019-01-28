use super::source::Source;

#[test]
fn test_require_ident() {
    let mut source = Source::dummy("foo123");
    assert_eq!(source.require_ident().unwrap(), "foo123");
    let mut source = Source::dummy("123foo");
    assert!(source.require_ident().is_err());
}

//fn test_require_ascii() {

//fn test_require_sep() {

#[test]
fn test_skip_ws() {
    let mut source = Source::dummy(" \n b");
    source.skip_ws();
    assert_eq!(source.peek(), Some('\n'));
}

#[test]
fn test_skip_wsn() {
    let mut source = Source::dummy(" \n b");
    source.skip_wsn();
    assert_eq!(source.peek(), Some('b'));
}

#[test]
fn test_skip_comment() {
    let mut source = Source::dummy("#a  \nb");
    source.skip_comment();
    assert_eq!(source.peek(), Some('b'));
}

#[test]
fn test_starts_with() {
    let source = Source::dummy("if");
    assert_eq!(source.starts_with("if"), true);
    assert_eq!(source.starts_with("end"), false);
}

#[test]
fn test_read_ascii() {
    let mut source = Source::dummy("if");
    source.read_ascii("if");
    assert_eq!(source.peek(), None)
}

#[test]
fn test_peek_char() {
    let mut source = Source::dummy("");
    assert!(source.peek_char().is_err())
}

#[test]
fn test_next_char() {
    let mut source = Source::dummy("");
    assert!(source.next_char().is_err())
}

#[test]
fn test_peek() {
    let mut source = Source::dummy("1+2");
    assert_eq!(source.peek(), Some('1'));
    assert_eq!(source.peek(), Some('1'));
    assert_eq!(source.loc.col, 0);
}

#[test]
fn test_next() {
    let mut source = Source::dummy("1+2");
    assert_eq!(source.next(), Some('1'));
    assert_eq!(source.loc.col, 1);
    assert_eq!(source.next(), Some('+'));
    assert_eq!(source.loc.col, 2);
}

#[test]
fn test_next_newline() {
    let mut source = Source::dummy("1\n2");
    source.next();
    source.next();
    assert_eq!(source.peek(), Some('2'));
    assert_eq!(source.loc.line, 1);
    assert_eq!(source.loc.col, 0);
}
