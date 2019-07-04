#![feature(range_contains)]
#![feature(nll)]
pub mod ast;
pub mod ty;
pub mod parser;
pub mod hir;
pub mod code_gen;
pub mod stdlib;

fn main() -> Result<(), Box<std::error::Error>> {
    let str = "
putchar 72
putchar 100 + 5";
    let ast = parser::Parser::parse(str)?;
    let stdlib = stdlib::create_classes();
    let hir = hir::Hir::from_ast(ast, &stdlib)?;
    let code_gen = code_gen::CodeGen::new();
    code_gen.gen_program(hir, stdlib)?;
    code_gen.module.print_to_file("a.ll")?;
    Ok(())
}
