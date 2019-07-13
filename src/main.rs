use shiika;

fn main() -> Result<(), Box<std::error::Error>> {
    let str = "
class A; def id(x: Int, y: Int) -> Int; y; end; end
putchar 72
putchar 100 + 5";
    let ast = shiika::parser::Parser::parse(str)?;
    let stdlib = shiika::stdlib::create_classes();
    let hir = shiika::hir::Hir::from_ast(ast, &stdlib)?;
    let mut code_gen = shiika::code_gen::CodeGen::new();
    code_gen.gen_program(hir, &stdlib)?;
    code_gen.module.print_to_file("a.ll")?;
    Ok(())
}
