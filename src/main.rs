use anyhow::Result;
use shiika::cli;
use shiika::compiler;
use shiika::config;
use shiika::runner;

fn main() -> Result<()> {
    env_logger::init();
    let args = cli::parse_command_line_args();

    match &args.command {
        cli::Command::Build => {
            compiler::build(std::env::current_dir()?)?;
        }
        cli::Command::BuildCorelib => {
            compiler::build_corelib()?;
        }
        cli::Command::Compile { filepath } => {
            compiler::compile_single(filepath)?;
        }
        cli::Command::Env => {
            config::print();
        }
        cli::Command::Run { filepath } => {
            compiler::compile_single(filepath)?;
            runner::run(filepath)?;
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
