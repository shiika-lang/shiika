use shiika;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let str = "
class A
  def fib(n: Int) -> Int
    if n < 3
      1
    else
      fib(n-1) + fib(n-2)
    end
  end
end
a = A.new
putchar a.fib(40)
putchar 20
putchar 79
putchar 107";
    let ast = shiika::parser::Parser::parse(str)?;
    let stdlib = shiika::stdlib::Stdlib::create();
    let hir = shiika::hir::Hir::from_ast(ast, stdlib)?;
    let mut code_gen = shiika::code_gen::CodeGen::new();
    code_gen.gen_program(hir)?;
    code_gen.module.print_to_file("a.ll")?;
    Ok(())
}
