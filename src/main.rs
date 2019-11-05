use shiika;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let str = "
# hello
H = 72
class A
  def self.foo -> Int; 72; end

  def fib(n: Int) -> Int
    if n < 3
      1
    else
      fib(n-1) + fib(n-2)
    end
  end
end
putchar A.new.fib(40)
putchar H
putchar 100 + 5";
    let ast = shiika::parser::Parser::parse(str)?;
    let stdlib = shiika::stdlib::Stdlib::create();
    let hir = shiika::hir::Hir::from_ast(ast, stdlib)?;
    let mut code_gen = shiika::code_gen::CodeGen::new();
    code_gen.gen_program(hir)?;
    code_gen.module.print_to_file("a.ll")?;
    Ok(())
}
