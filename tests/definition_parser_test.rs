use shiika::ast;
use shiika::parser::Parser;
use shiika::names::*;

fn parse_definitions(src: &str) -> Result<Vec<ast::Definition>, shiika::error::Error> {
    let mut parser = Parser::new(src);
    parser.parse_definitions()
}

#[test]
fn test_emtpy_class() {
    let result = parse_definitions("class A; end");
    assert_eq!(result.unwrap(), vec![ 
        ast::Definition::ClassDefinition {
            name: ClassFirstName("A".to_string()),
            defs: vec![]
        }
    ])
}

#[test]
fn test_class_with_empty_method() {
    let result = parse_definitions("class A; def foo; end; end");
    assert_eq!(result.unwrap(), vec![
        ast::Definition::ClassDefinition {
            name: ClassFirstName("A".to_string()),
            defs: vec![
                ast::Definition::InstanceMethodDefinition {
                    sig: ast::AstMethodSignature {
                        name: MethodFirstName("foo".to_string()),
                        params: vec![],
                        ret_typ: ast::Typ { name: "Void".to_string() },
                    },
                    body_exprs: vec![],
                }
            ]
        }
    ])
}

#[test]
fn test_method_with_params() {
    let mut parser = Parser::new("def foo(a: Int, b: Float); end");
    let result = parser.parse_method_definition();
    assert_eq!(result.unwrap(), ast::Definition::InstanceMethodDefinition {
        sig: ast::AstMethodSignature {
            name: MethodFirstName("foo".to_string()),
            params: vec![
                ast::Param { name: "a".to_string(), typ: ast::Typ { name: "Int".to_string() }},
                ast::Param { name: "b".to_string(), typ: ast::Typ { name: "Float".to_string() }},
            ],
            ret_typ: ast::Typ { name: "Void".to_string() },
        },
        body_exprs: vec![],
    })
}

#[test]
fn test_method_with_explicit_return_type() {
    let mut parser = Parser::new("def foo() -> Int; end");
    let result = parser.parse_method_definition();
    assert_eq!(result.unwrap(), ast::Definition::InstanceMethodDefinition {
        sig: ast::AstMethodSignature {
            name: MethodFirstName("foo".to_string()),
            params: vec![],
            ret_typ: ast::Typ { name: "Int".to_string() },
        },
        body_exprs: vec![],
    })
}

#[test]
fn test_class_method_def() {
    let mut parser = Parser::new("def self.foo; end");
    let result = parser.parse_method_definition();
    assert_eq!(result.unwrap(), ast::Definition::ClassMethodDefinition {
        sig: ast::AstMethodSignature {
            name: MethodFirstName("foo".to_string()),
            params: vec![],
            ret_typ: ast::Typ { name: "Void".to_string() },
        },
        body_exprs: vec![],
    })
}
