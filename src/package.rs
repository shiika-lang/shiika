use anyhow::{Context, Result};
use json5;
use serde::Deserialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Deserialize, PartialEq, Debug)]
pub struct SkPackage {
    dir: PathBuf,
    pub spec: PackageSpec,
}

#[derive(Deserialize, PartialEq, Debug)]
pub struct PackageSpec {
    pub apps: Option<Vec<String>>,
    pub export: Option<LibraryName>,
    pub dependencies: Vec<Dependency>,
}

#[derive(Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
pub struct LibraryName(String);

#[derive(Deserialize, PartialEq, Debug)]
pub struct Dependency {
    pub name: String,
    pub source: PackageSource,
}

#[derive(Deserialize, PartialEq, Debug)]
pub struct PackageSource {
    pub path: String,
}

impl SkPackage {
    pub fn load<P: AsRef<Path>>(dir_: P) -> Result<SkPackage> {
        let dir = dir_.as_ref().to_path_buf();
        let spec_path = dir.join("package.json5");
        let mut f =
            fs::File::open(&spec_path).context(format!("{} not found", spec_path.display()))?;
        let mut contents = String::new();
        f.read_to_string(&mut contents)
            .context(format!("failed to read {}", spec_path.display()))?;
        let spec = json5::from_str(&contents)
            .context(format!("failed to load {}", spec_path.display()))?;
        Ok(SkPackage { dir, spec })
    }

    pub fn dir(&self) -> &PathBuf {
        &self.dir
    }

    pub fn link_files(&self) -> Vec<PathBuf> {
        vec![self.dir.join("index.bc")]
    }

    pub fn export(&self) -> Option<&str> {
        self.spec.export.as_ref().map(|n| &*n.0)
    }
}

impl LibraryName {
    pub fn builtin() -> LibraryName {
        LibraryName("builtin".to_string())
    }

    pub fn to_strs(names: &[LibraryName]) -> Vec<String> {
        names.iter().map(|n| n.0.clone()).collect()
    }
}

impl Dependency {
    pub fn resolve(&self) -> Result<SkPackage> {
        // TODO: Run `git clone` if source is git, etc.
        SkPackage::load(&self.source.path)
    }
}
