use std::process::Command;

#[test]
fn test_compile_and_run() -> Result<(), Box<dyn std::error::Error>> {
    let src = "
class A; def foo; end; end
putchar 72
putchar 100 + 5";
    let ast = shiika::parser::Parser::parse(src)?;
    let corelib = shiika::corelib::Corelib::create();
    let hir = shiika::hir::Hir::from_ast(ast, corelib)?;
    let mut code_gen = shiika::code_gen::CodeGen::new(&hir);
    code_gen.gen_program(&hir)?;
    code_gen.module.print_to_file("tests/out.ll")?;

    let mut cmd = Command::new("llc");
    cmd.arg("tests/out.ll");
    cmd.output().unwrap();

    let mut cmd = Command::new("cc");
    cmd.arg("-I/usr/local/Cellar/bdw-gc/7.6.0/include/");
    cmd.arg("-L/usr/local/Cellar/bdw-gc/7.6.0/lib/");
    cmd.arg("-lgc");
    cmd.arg("-otests/out");
    cmd.arg("tests/out.s");
    cmd.output().unwrap();

    let mut cmd = Command::new("tests/out");
    let output = cmd.output().expect("failed to execute process");
    let stdout = String::from_utf8(output.stdout).expect("invalid utf8 in stdout");
    let stderr = String::from_utf8(output.stderr).expect("invalid utf8 in stderr");
    assert_eq!(stderr, "");
    assert_eq!(stdout, "Hi");
    Ok(())
}

