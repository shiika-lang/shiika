use clap::{Parser, Subcommand};

#[derive(clap::Parser, Debug)]
#[clap(name = "shiika", author, version, about)]
pub struct Arguments {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Compile shiika program
    Compile { filepath: String },
    /// Compile and execute shiika program
    Run { filepath: String },
    /// Build corelib
    BuildCorelib,
}

pub fn parse_command_line_args() -> Arguments {
    Arguments::parse()
}
