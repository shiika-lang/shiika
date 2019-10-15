use shiika::ast;
use shiika::parser::Parser;

fn parse_expr(src: &str) -> Result<ast::Expression, shiika::error::Error> {
    let mut parser = Parser::new(src);
    parser.parse_expr()
}

#[test]
fn test_if_expr() {
    let result = parse_expr("if 1 then 2 else 3 end");
    assert_eq!(result.unwrap(),
    ast::if_expr(
        ast::decimal_literal(1),
        ast::decimal_literal(2),
        Some(ast::decimal_literal(3))))
}

#[test]
fn test_const_assign() {
    let result = parse_expr("X = 1");
    assert_eq!(result.unwrap(),
    ast::assignment(
        ast::const_ref(vec!["X".to_string()]),
        ast::decimal_literal(1)))
}

#[test]
fn test_additive_expr() {
    let result = parse_expr("1+2*3");
    assert_eq!(result.unwrap(),
    ast::method_call(
        Some(ast::decimal_literal(1)),
        "+",
        vec![ast::method_call(
            Some(ast::decimal_literal(2)),
            "*",
            vec![ast::decimal_literal(3)],
            false,
            false)],
        false,
        false))
}

#[test]
fn test_multiplicative_with_method_call() {
    let result = parse_expr("1.foo * 2");

    let left = ast::method_call(
        Some(ast::decimal_literal(1)),
        "foo",
        vec![],
        true,
        true);

    assert_eq!(result.unwrap(), 
    ast::method_call(
        Some(left),
        "*",
        vec![ast::decimal_literal(2)],
        false,
        false))
}

#[test]
fn test_unary() {
    let result = parse_expr("p -1");
    let minus1 = ast::method_call(
        Some(ast::decimal_literal(1)),
        "-@",
        vec![],
        true,
        false);

    assert_eq!(result.unwrap(), 
    ast::method_call(
        None,
        "p",
        vec![minus1],
        false,
        false))
}

#[test]
fn test_binary() {
    let result = parse_expr("p - 1");
    assert_eq!(result.unwrap(), 
    ast::method_call(
        Some(ast::bare_name("p")),
        "-",
        vec![ast::decimal_literal(1)],
        false,
        false))
}

#[test]
fn test_method_call_with_paren_and_dot() {
    let result = parse_expr("foo bar().baz");

    let call_bar = ast::method_call(
        None,
        "bar",
        vec![],
        true,
        false);

    let right = ast::method_call(
        Some(call_bar),
        "baz",
        vec![],
        true,
        true);

    assert_eq!(result.unwrap(), 
    ast::method_call(
        None,
        "foo",
        vec![right],
        false,
        false));
}

#[test]
fn test_float_literal() {
    let result = parse_expr("1.23");
    assert_eq!(result.unwrap(), 
        ast::float_literal(1.23),
    )
}

#[test]
fn test_decimal_literal() {
    let result = parse_expr("123");
    assert_eq!(result.unwrap(), 
        ast::decimal_literal(123),
    )
}

//
// Method call (0 args)
//

#[test]
fn test_bare_name() {
    let result = parse_expr("foo");
    assert_eq!(result.unwrap(), ast::bare_name("foo"))
}

#[test]
fn test_call_with_paren_0() {
    let result = parse_expr("foo()");
    assert_eq!(result.unwrap(),
    ast::method_call(
        None,
        "foo",
        vec![],
        true,
        false))
}

#[test]
fn test_call_with_dot() {
    let result = parse_expr("1.foo");
    assert_eq!(result.unwrap(),
    ast::method_call(
        Some(ast::decimal_literal(1)),
        "foo",
        vec![],
        true,
        true))
}

//
// Method call (1 arg)
//

#[test]
fn test_call_with_paren_1() {
    let result = parse_expr("foo(1)");
    assert_eq!(result.unwrap(),
    ast::method_call(
        None,
        "foo",
        vec![ast::decimal_literal(1)],
        true,
        false))
}

#[test]
fn test_call_with_space_1() {
    let result = parse_expr("foo 1");
    assert_eq!(result.unwrap(),
    ast::method_call(
        None,
        "foo",
        vec![ast::decimal_literal(1)],
        false,
        false))
}

//
// Method call (2 args)
//

#[test]
fn test_call_with_paren_2() {
    let result = parse_expr("foo(1, 2)");
    assert_eq!(result.unwrap(),
    ast::method_call(
        None,
        "foo",
        vec![
            ast::decimal_literal(1),
            ast::decimal_literal(2),
        ],
        true,
        false))
}

#[test]
fn test_call_with_space_2() {
    let result = parse_expr("foo 1, 2");
    assert_eq!(result.unwrap(),
    ast::method_call(
        None,
        "foo",
        vec![
            ast::decimal_literal(1),
            ast::decimal_literal(2),
        ],
        false,
        false))
}
