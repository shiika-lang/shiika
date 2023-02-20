use clap::{Parser, Subcommand};

#[derive(clap::Parser, Debug)]
#[clap(name = "subcommand", author, version, about)]
pub struct Arguments {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Build corelib
    BuildCorelib,
    /// Compile shiika program
    Compile { filepath: String },
    /// Compile shiika library
    CompileLib { path: String },
    /// Print configured env
    Env,
    /// Compile and execute shiika program
    Run { filepath: String },
}

pub fn parse_command_line_args() -> Arguments {
    Arguments::parse()
}
