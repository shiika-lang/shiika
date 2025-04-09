use anyhow::{Context, Result};
use serde::Deserialize;
use std::io::Read;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Deserialize)]
pub struct PackageSpec {
    pub name: String,
    pub version: String, // TODO: parse it
    pub rust_libs: Option<Vec<String>>,
}

/// Load the package.json5 file from the given path.
/// If the path is a file, it will be used as the package.json5 file.
/// Returns the directory of the package and the parsed PackageSpec.
pub fn load_spec(path: &PathBuf) -> Result<(PathBuf, PackageSpec)> {
    let (dir, package_json5_path) = if path.is_file() {
        (path.parent().unwrap().to_path_buf(), path.clone())
    } else {
        (path.clone(), path.join("package.json5"))
    };
    let spec = parse_package_json5(&package_json5_path)?;
    Ok((dir, spec))
}

pub fn load_core(shiika_root: PathBuf) -> Result<(PathBuf, PackageSpec)> {
    let path = shiika_root.join("packages").join("core");
    let (dir, spec) = load_spec(&path)?;
    Ok((dir, spec))
}

fn parse_package_json5(path: &PathBuf) -> Result<PackageSpec> {
    let mut f = std::fs::File::open(path)?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .context(format!("failed to read {}", path.display()))?;
    let spec: PackageSpec =
        json5::from_str(&contents).context(format!("failed to parse {}", path.display()))?;
    Ok(spec)
}
