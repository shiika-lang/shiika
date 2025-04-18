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
        let p = package::Package::new(&self, filepath)?;
        build::cargo_builder::run(self, &p)?;
        build::lib_builder::build(self, &p)?;
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

    pub fn lib_exports_path(&self, spec: &package::PackageSpec) -> PathBuf {
        self.lib_target_dir(spec).join("exports.json")
    }

    pub fn lib_target_dir(&self, spec: &package::PackageSpec) -> PathBuf {
        self.package_work_dir(spec).join("lib")
    }

    pub fn rust_artifact_path(&self, spec: &package::PackageSpec, _rust_lib: &str) -> PathBuf {
        let name = "ext"; // TODO: read Cargo.toml
        self.cargo_target_dir(spec)
            .join("debug")
            .join(if cfg!(target_os = "windows") {
                format!("{}.lib", name)
            } else {
                format!("lib{}.a", name)
            })
    }

    pub fn cargo_target_dir(&self, spec: &package::PackageSpec) -> PathBuf {
        self.package_work_dir(spec).join("cargo_target")
    }

    pub fn package_work_dir(&self, spec: &package::PackageSpec) -> PathBuf {
        self.shiika_work
            .join("packages")
            .join(format!("{}-{}", &spec.name, &spec.version))
    }

    pub fn log(&mut self, s: impl AsRef<str>) {
        self.log_file.write_all(s.as_ref().as_bytes()).unwrap();
    }
}
