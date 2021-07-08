use shiika::error::Error;
use shiika::runner;
#[macro_use]
extern crate clap;

fn main() {
    env_logger::init();

    main_().unwrap_or_else(|err| {
        print_err(err);
        std::process::exit(1);
    })
}

fn main_() -> Result<(), Error> {
    let yaml = load_yaml!("cli.yml");
    let matches = clap::App::from(yaml).get_matches();

    if let Some(matches) = matches.subcommand_matches("compile") {
        let filepath = matches.value_of("INPUT").unwrap();
        runner::compile(filepath)?;
    }

    if let Some(matches) = matches.subcommand_matches("run") {
        let filepath = matches.value_of("INPUT").unwrap();
        runner::compile(filepath)?;
        runner::run(filepath)?;
    }

    if matches.subcommand_matches("build_corelib").is_some() {
        runner::build_corelib()?;
    }

    Ok(())
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
