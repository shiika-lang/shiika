use shiika::error::Error;
use shiika::runner;
#[macro_use]
extern crate clap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let yaml = load_yaml!("cli.yml");
    let matches = clap::App::from(yaml).get_matches();

    if let Some(ref matches) = matches.subcommand_matches("compile") {
        let filepath = matches.value_of("INPUT").unwrap();
        compile(filepath);
    }

    if let Some(ref matches) = matches.subcommand_matches("run") {
        let filepath = matches.value_of("INPUT").unwrap();
        if compile(filepath) {
            run(filepath);
        }
    }

    Ok(())
}

fn compile(filepath: &str) -> bool {
    match runner::compile(filepath) {
        Ok(_) => true,
        Err(err) => {
            print_err(err);
            false
        }
    }
}

fn run(filepath: &str) {
    runner::run(filepath).unwrap_or_else(|err| {
        print_err(err);
    });
}

fn print_err(err: Error) {
    println!("{:?}: {}", err.details, err.msg);
    for frame in err.backtrace.frames() {
        for symbol in frame.symbols() {
            if let Some(name) = symbol.name() {
                let s = format!("{}", name);
                if s.starts_with("shiika") {
                    println!("- {}", s);
                }
            }
        }
    }
}
