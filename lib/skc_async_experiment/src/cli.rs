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

    /// Build a package.
    pub fn build(&mut self, filepath: &PathBuf) -> Result<()> {
        let (dir, spec) = package::load_spec(filepath)?;
        build::cargo_builder::run(self, &dir, &spec)?;
        Ok(())
    }

    /// Build and run a single .sk file.
    pub fn run(&mut self, filepath: &PathBuf) -> Result<()> {
        let bin_path = build::exe_builder::run(self, filepath)?;
        let mut cmd = std::process::Command::new(bin_path);
        cmd.status()?;
        Ok(())
    }

    /// Like `run`, but only builds the executable.
    pub fn compile(&mut self, filepath: &PathBuf) -> Result<()> {
        build::exe_builder::run(self, filepath)?;
        Ok(())
    }

    pub fn package_build_dir(&self, spec: &package::PackageSpec) -> PathBuf {
        self.shiika_work
            .join("packages")
            .join(format!("{}-{}", &spec.name, &spec.version))
    }

    pub fn built_core(&self) -> Result<PathBuf> {
        let (_, spec) = package::load_core(self.shiika_root.clone())?;
        let name = if cfg!(target_os = "windows") {
            "ext.lib"
        } else {
            "libext.a"
        };
        Ok(self.package_build_dir(&spec).join("debug").join(name))
    }

    pub fn log(&mut self, s: impl AsRef<str>) {
        self.log_file.write_all(s.as_ref().as_bytes()).unwrap();
    }
}
