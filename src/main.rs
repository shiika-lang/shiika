use std::fs;
use std::process::Command;
#[macro_use]
extern crate clap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = load_yaml!("cli.yml");
    let matches = clap::App::from(yaml).get_matches();

    if let Some(ref matches) = matches.subcommand_matches("compile") {
        let filepath = matches.value_of("INPUT").unwrap();
        compile(filepath)?;
    }

    if let Some(ref matches) = matches.subcommand_matches("run") {
        let filepath = matches.value_of("INPUT").unwrap();
        compile(filepath)?;
        run(filepath)?;
    }

    Ok(())
}

fn compile(filepath: &str) -> Result<(), Box<dyn std::error::Error>> {
    let builtin = load_builtin()?;
    let str = builtin + &fs::read_to_string(filepath)?;
    let ast = shiika::parser::Parser::parse(&str)?;
    let corelib = shiika::corelib::Corelib::create();
    let hir = shiika::hir::build(ast, corelib)?;
    shiika::code_gen::run(&hir, filepath)?;
    Ok(())
}

fn load_builtin() -> Result<String, Box<dyn std::error::Error>> {
    let mut s = String::new();
    for item in fs::read_dir("builtin")? {
        let pathbuf = item?.path();
        let path = pathbuf.to_str().expect("Filename not utf8");
        if path.ends_with(".sk") {
            s += &fs::read_to_string(path)?;
        }
    }
    Ok(s)
}

fn run(sk_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ll_path = sk_path.to_string() + ".ll";
    let opt_ll_path = sk_path.to_string() + ".opt.ll";
    let bc_path = sk_path.to_string() + ".bc";
    let asm_path = sk_path.to_string() + ".s";
    let out_path = sk_path.to_string() + ".out";

    let mut cmd = Command::new("opt");
    cmd.arg("-O3");
    cmd.arg(ll_path);
    cmd.arg("-o");
    cmd.arg(bc_path.clone());
    let output = cmd.output()?;
    if !output.stderr.is_empty() {
        println!("{}", String::from_utf8(output.stderr)?);
    }

    let mut cmd = Command::new("llvm-dis");
    cmd.arg(bc_path.clone());
    cmd.arg("-o");
    cmd.arg(opt_ll_path);
    cmd.output()?;

    let mut cmd = Command::new("llc");
    cmd.arg(bc_path.clone());
    let output = cmd.output()?;
    if !output.stderr.is_empty() {
        println!("{}", String::from_utf8(output.stderr)?);
    }

    let mut cmd = Command::new("cc");
    cmd.arg("-I/usr/local/Cellar/bdw-gc/7.6.0/include/");
    cmd.arg("-L/usr/local/Cellar/bdw-gc/7.6.0/lib/");
    cmd.arg("-lgc");
    cmd.arg("-o");
    cmd.arg(out_path.clone());
    cmd.arg(asm_path.clone());
    cmd.output()?;

    fs::remove_file(bc_path)?;
    fs::remove_file(asm_path)?;

    let mut cmd = Command::new(out_path);
    cmd.status()?;

    Ok(())
}
