mod command_line_options;
use crate::{build, codegen, mir, package, prelude};
use anyhow::{bail, Context, Result};
pub use command_line_options::{Command, CommandLineOptions};
use shiika_parser::SourceFile;
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};

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

    pub fn run<P: AsRef<Path>>(&mut self, filepath: P) -> Result<()> {
        let path = filepath.as_ref();
        let txt = std::fs::read_to_string(path)
            .context(format!("failed to read {}", &path.to_string_lossy()))?;
        let src = SourceFile::new(path.to_path_buf(), txt);
        let mut mir = build::compiler::run(self, src)?;

        for (name, fun_ty) in prelude::core_externs() {
            mir.program.externs.push(mir::Extern { name, fun_ty });
        }
        mir.program.funcs.append(&mut prelude::funcs());

        self.log(&format!("# -- verifier input --\n{}\n", mir.program));
        mir::verifier::run(&mir.program)?;

        let bc_path = path.with_extension("bc");
        let ll_path = path.with_extension("ll");
        codegen::run(&bc_path, Some(&ll_path), mir)?;
        build::linker::run(bc_path)?;
        Ok(())
    }

    pub fn log(&mut self, s: impl AsRef<str>) {
        self.log_file.write_all(s.as_ref().as_bytes()).unwrap();
    }
}
