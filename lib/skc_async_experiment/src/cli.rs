mod command_line_options;
use crate::{build, package};
use anyhow::{bail, Result};
pub use command_line_options::{Command, CommandLineOptions};
use std::env;
use std::io::Write;
use std::path::PathBuf;

const SHIIKA_ROOT: &str = "SHIIKA_ROOT";
const SHIIKA_WORK: &str = "SHIIKA_WORK";

fn shiika_root() -> Result<PathBuf> {
    let Ok(shiika_root) = env::var(SHIIKA_ROOT) else {
        bail!("please set {} (where you cloned shiika repo)", SHIIKA_ROOT);
    };
    let path = PathBuf::from(shiika_root);
    if !path.exists() {
        bail!("{} does not exist", path.display());
    }
    Ok(path)
}

fn shiika_work() -> Result<PathBuf> {
    let Ok(shiika_work) = env::var(SHIIKA_WORK) else {
        bail!("please set {} (eg. ~/.shiika)", SHIIKA_WORK);
    };
    let path = PathBuf::from(shiika_work);
    if !path.exists() {
        bail!("{} does not exist", path.display());
    }
    Ok(path)
}

pub struct Cli {
    pub log_file: std::fs::File,
    pub shiika_root: PathBuf,
    pub shiika_work: PathBuf,
}

impl Cli {
    pub fn init() -> Result<Self> {
        Ok(Self {
            log_file: std::fs::File::create("log.milikac").unwrap(),
            shiika_root: shiika_root()?,
            shiika_work: shiika_work()?,
        })
    }

    pub fn build(&mut self, filepath: &PathBuf) -> Result<()> {
        let (dir, spec) = package::load_spec(filepath)?;
        build::cargo_builder::run(self, &dir, &spec)?;
        Ok(())
    }

    pub fn run(&mut self, filepath: &PathBuf) -> Result<()> {
        build::exe_builder::run(self, filepath)
    }

    pub fn log(&mut self, s: impl AsRef<str>) {
        self.log_file.write_all(s.as_ref().as_bytes()).unwrap();
    }
}
