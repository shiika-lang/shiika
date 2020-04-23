use shiika::hir::*;
use shiika::names::*;
use shiika::ty;
use shiika::corelib::Corelib;

#[test]
fn test_discarding_return_value() -> Result<(), Box<dyn std::error::Error>> {
    let src = "
      class A
        def foo
          42
        end
      end
    ";
    let ast = shiika::parser::Parser::parse(src)?;
    let hir = shiika::hir::Hir::from_ast(ast, Corelib::empty())?;
    println!("sk_methods: {:?}", hir.sk_methods);
    let method = &hir.sk_methods.get(&ClassFullname("A".to_string())).unwrap()[0];
    assert_eq!(method.signature.ret_ty, ty::raw("Void"));
    assert_eq!(method.body, SkMethodBody::ShiikaMethodBody {
        exprs: HirExpressions {
            ty: ty::raw("Int"),
            exprs: vec![ HirExpression {
                ty: ty::raw("Int"),
                node: HirExpressionBase::HirDecimalLiteral { value: 42 },
            }],
        }
    });
    Ok(())
}
