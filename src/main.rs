use anyhow::{anyhow, Result};
use shiika::cli;
use shiika::runner;
use std::io::{self, Write};

fn main() -> Result<()> {
    match main_() {
        // Print error report if available
        Err(e) => match e.downcast_ref::<shiika_parser::Error>() {
            Some(shiika_parser::Error::ParseError { report, .. }) => {
                io::stderr().write(report).unwrap();
                Err(anyhow!("ParseError"))
            }
            _ => Err(e),
        },
        other => other,
    }
}

fn main_() -> Result<()> {
    env_logger::init();
    let args = cli::parse_command_line_args();

    match &args.command {
        cli::Command::Compile { filepath } => {
            runner::compile(filepath)?;
        }
        cli::Command::Run { filepath } => {
            runner::compile(filepath)?;
            runner::run(filepath)?;
        }
        cli::Command::BuildCorelib => {
            runner::build_corelib()?;
        }
    }

    Ok(())
}

//fn print_err(err: Error) {
//    println!("{}", err.msg);
//    for frame in err.backtrace.frames() {
//        for symbol in frame.symbols() {
//            if let Some(name) = symbol.name() {
//                let s = format!("{}", name);
//                if s.starts_with("shiika") {
//                    println!("- {}", s);
//                }
//            }
//        }
//    }
//}
