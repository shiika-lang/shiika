use shiika::ast;
use shiika::parser::Parser;
use shiika::parser::ParseError;

fn parse_expr(src: &str) -> Result<ast::Expression, ParseError> {
    let mut parser = Parser::new(src);
    parser.parse_expr()
}

#[test]
fn test_if_expr() {
    let result = parse_expr("if 1 then 2 else 3 end");
    assert_eq!(result.unwrap(), 
        ast::Expression::If {
            cond_expr: Box::new(ast::decimal_literal(1)),
            then_expr: Box::new(ast::decimal_literal(2)),
            else_expr: Some(Box::new(ast::decimal_literal(3))),
        }
    )
}

#[test]
fn test_additive_expr() {
    let result = parse_expr("1+2*3");
    assert_eq!(result.unwrap(), 
        ast::Expression::BinOpExpression {
            left: Box::new(ast::decimal_literal(1)),
            op: ast::BinOp::Add,
            right: Box::new(ast::Expression::BinOpExpression {
                left: Box::new(ast::decimal_literal(2)),
                op: ast::BinOp::Mul,
                right: Box::new(ast::decimal_literal(3)),
            }),
        }
    )
}

#[test]
fn test_multiplicative_expr() {
    let result = parse_expr("1%2");
    assert_eq!(result.unwrap(), 
        ast::Expression::BinOpExpression {
            left: Box::new(ast::decimal_literal(1)),
            op: ast::BinOp::Mod,
            right: Box::new(ast::decimal_literal(2)),
        }
    )
}

#[test]
fn test_multiplicative_with_method_call() {
    let result = parse_expr("1.foo * 2");
    assert_eq!(result.unwrap(), 
        ast::Expression::BinOpExpression {
            left: Box::new(ast::Expression::MethodCall {
                receiver_expr: Some(Box::new(ast::decimal_literal(1))),
                method_name: "foo".to_string(),
                arg_exprs: vec![],
            }),
            op: ast::BinOp::Mul,
            right: Box::new(ast::decimal_literal(2)),
        }
    )
}

#[test]
fn test_method_call_with_dot_and_paren() {
    let result = parse_expr("1.foo(2)");
    assert_eq!(result.unwrap(), 
        ast::Expression::MethodCall {
            receiver_expr: Some(Box::new(ast::decimal_literal(1))),
            method_name: "foo".to_string(),
            arg_exprs: vec![ast::decimal_literal(2)],
        }
    )
}

#[test]
fn test_method_call_with_dot() {
    let result = parse_expr("1.foo 2");
    assert_eq!(result.unwrap(), 
        ast::Expression::MethodCall {
            receiver_expr: Some(Box::new(ast::decimal_literal(1))),
            method_name: "foo".to_string(),
            arg_exprs: vec![ast::decimal_literal(2)],
        }
    )
}

#[test]
fn test_method_call_with_paren() {
    let result = parse_expr("foo(2)");
    assert_eq!(result.unwrap(), 
        ast::Expression::MethodCall {
            receiver_expr: None,
            method_name: "foo".to_string(),
            arg_exprs: vec![ast::decimal_literal(2)],
        }
    )
}

#[test]
fn test_method_call_no_paren_or_dot() {
    let result = parse_expr("foo 2");
    assert_eq!(result.unwrap(), 
        ast::Expression::MethodCall {
            receiver_expr: None,
            method_name: "foo".to_string(),
            arg_exprs: vec![ast::decimal_literal(2)],
        }
    )
}

#[test]
fn test_method_call_with_binop() {
    let result = parse_expr("foo 1+2");
    assert_eq!(result.unwrap(), 
        ast::Expression::MethodCall {
            receiver_expr: None,
            method_name: "foo".to_string(),
            arg_exprs: vec![
                ast::Expression::BinOpExpression {
                    left: Box::new(ast::decimal_literal(1)),
                    op: ast::BinOp::Add,
                    right: Box::new(ast::decimal_literal(2)),
                }
            ]
        }
    )
}

#[test]
fn test_method_call_with_args() {
    let result = parse_expr("foo 1, 2");
    assert_eq!(result.unwrap(), 
        ast::Expression::MethodCall {
            receiver_expr: None,
            method_name: "foo".to_string(),
            arg_exprs: vec![
                ast::decimal_literal(1),
                ast::decimal_literal(2),
            ]
        }
    )
}

#[test]
fn test_parenthesized_expr() {
    let result = parse_expr("(123)");
    assert_eq!(result.unwrap(), 
        ast::decimal_literal(123),
    )
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
