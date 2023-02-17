use anyhow::Result;
use shiika::cli;
use shiika::compiler;
use shiika::runner;

fn main() -> Result<()> {
    env_logger::init();
    let args = cli::parse_command_line_args();

    match &args.command {
        cli::Command::Compile { filepath } => {
            compiler::compile(filepath)?;
        }
        cli::Command::Run { filepath } => {
            compiler::compile(filepath)?;
            runner::run(filepath)?;
        }
        cli::Command::BuildCorelib => {
            compiler::build_corelib()?;
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
