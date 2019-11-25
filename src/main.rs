use std::fs;
#[macro_use]
extern crate clap;
use shiika;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = load_yaml!("cli.yml");
    let matches = clap::App::from(yaml).get_matches();

    if let Some(ref matches) = matches.subcommand_matches("compile") {
        let filepath = matches.value_of("INPUT").unwrap();
        let str = fs::read_to_string(filepath)?;
        let ast = shiika::parser::Parser::parse(&str)?;
        let stdlib = shiika::stdlib::Stdlib::create();
        let hir = shiika::hir::Hir::from_ast(ast, stdlib)?;
        let mut code_gen = shiika::code_gen::CodeGen::new();
        code_gen.gen_program(hir)?;
        code_gen.module.print_to_file(filepath.to_string() + ".ll")?;
    }

    Ok(())
}
