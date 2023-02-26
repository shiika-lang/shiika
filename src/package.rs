use anyhow::{Context, Result};
use json5;
use serde::Deserialize;
use std::fs;
use std::io::Read;
use std::path::Path;

#[derive(Deserialize, PartialEq, Debug)]
pub struct SkPackage {
    pub apps: Option<Vec<String>>,
    pub export: Option<String>,
    pub dependencies: Vec<Dependency>,
}

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
    pub fn load<P: AsRef<Path>>(path_: P) -> Result<SkPackage> {
        let path = path_.as_ref();
        let mut f = fs::File::open(path).context(format!("{} not found", path.display()))?;
        let mut contents = String::new();
        f.read_to_string(&mut contents)
            .context(format!("failed to read {}", path.display()))?;
        json5::from_str(&contents).context(format!("failed to load {}", path.display()))
    }
}
