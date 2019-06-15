use super::super::ast;
use super::Parser;

#[test]
fn test_if_expr() {
    let result = Parser::parse("if 1 then 2 else 3 end");
    assert_eq!(result.unwrap(), 
        ast::Program {
            expr: ast::Expression::If {
                cond_expr: Box::new(ast::decimal_literal(1)),
                then_expr: Box::new(ast::decimal_literal(2)),
                else_expr: Some(Box::new(ast::decimal_literal(3))),
            }
        }
    )
}

#[test]
fn test_additive_expr() {
    let result = Parser::parse("1+2*3");
    assert_eq!(result.unwrap(), 
        ast::Program {
            expr: ast::Expression::BinOp {
                left: Box::new(ast::decimal_literal(1)),
                op: ast::BinOp::Add,
                right: Box::new(ast::Expression::BinOp {
                    left: Box::new(ast::decimal_literal(2)),
                    op: ast::BinOp::Mul,
                    right: Box::new(ast::decimal_literal(3)),
                }),
            },
        }
    )
}

#[test]
fn test_multiplicative_expr() {
    let result = Parser::parse("1%2");
    assert_eq!(result.unwrap(), 
        ast::Program {
            expr: ast::Expression::BinOp {
                left: Box::new(ast::decimal_literal(1)),
                op: ast::BinOp::Mod,
                right: Box::new(ast::decimal_literal(2)),
            },
        }
    )
}

#[test]
fn test_multiplicative_with_method_call() {
    let result = Parser::parse("1.foo * 2");
    assert_eq!(result.unwrap(), 
        ast::Program {
            expr: ast::Expression::BinOp {
                left: Box::new(ast::Expression::MethodCall {
                    receiver_expr: Some(Box::new(ast::decimal_literal(1))),
                    method_name: "foo".to_string(),
                    arg_exprs: vec![],
                }),
                op: ast::BinOp::Mul,
                right: Box::new(ast::decimal_literal(2)),
            }
        }
    )
}

#[test]
fn test_method_call_with_dot_and_paren() {
    let result = Parser::parse("1.foo(2)");
    assert_eq!(result.unwrap(), 
        ast::Program {
            expr: ast::Expression::MethodCall {
                receiver_expr: Some(Box::new(ast::decimal_literal(1))),
                method_name: "foo".to_string(),
                arg_exprs: vec![ast::decimal_literal(2)],
            }
        }
    )
}

#[test]
fn test_method_call_with_dot() {
    let result = Parser::parse("1.foo 2");
    assert_eq!(result.unwrap(), 
        ast::Program {
            expr: ast::Expression::MethodCall {
                receiver_expr: Some(Box::new(ast::decimal_literal(1))),
                method_name: "foo".to_string(),
                arg_exprs: vec![ast::decimal_literal(2)],
            }
        }
    )
}

#[test]
fn test_method_call_with_paren() {
    let result = Parser::parse("foo(2)");
    assert_eq!(result.unwrap(), 
        ast::Program {
            expr: ast::Expression::MethodCall {
                receiver_expr: None,
                method_name: "foo".to_string(),
                arg_exprs: vec![ast::decimal_literal(2)],
            }
        }
    )
}

#[test]
fn test_method_call_no_paren_or_dot() {
    let result = Parser::parse("foo 2");
    assert_eq!(result.unwrap(), 
        ast::Program {
            expr: ast::Expression::MethodCall {
                receiver_expr: None,
                method_name: "foo".to_string(),
                arg_exprs: vec![ast::decimal_literal(2)],
            }
        }
    )
}

#[test]
fn test_method_call_with_binop() {
    let result = Parser::parse("foo 1+2");
    assert_eq!(result.unwrap(), 
        ast::Program {
            expr: ast::Expression::MethodCall {
                receiver_expr: None,
                method_name: "foo".to_string(),
                arg_exprs: vec![
                    ast::Expression::BinOp {
                        left: Box::new(ast::decimal_literal(1)),
                        op: ast::BinOp::Add,
                        right: Box::new(ast::decimal_literal(2)),
                    }
                ]
            }
        }
    )
}

#[test]
fn test_method_call_with_args() {
    let result = Parser::parse("foo 1, 2");
    assert_eq!(result.unwrap(), 
        ast::Program {
            expr: ast::Expression::MethodCall {
                receiver_expr: None,
                method_name: "foo".to_string(),
                arg_exprs: vec![
                    ast::decimal_literal(1),
                    ast::decimal_literal(2),
                ]
            }
        }
    )
}

#[test]
fn test_parenthesized_expr() {
    let result = Parser::parse("(123)");
    assert_eq!(result.unwrap(), 
        ast::Program {
            expr: ast::decimal_literal(123),
        }
    )
}

#[test]
fn test_float_literal() {
    let result = Parser::parse("1.23");
    assert_eq!(result.unwrap(), 
        ast::Program {
            expr: ast::float_literal(1.23),
        }
    )
}

#[test]
fn test_decimal_literal() {
    let result = Parser::parse("123");
    assert_eq!(result.unwrap(), 
        ast::Program {
            expr: ast::decimal_literal(123),
        }
    )
}
