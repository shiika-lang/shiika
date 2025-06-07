use crate::cli::Cli;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::io::Read;
use std::path::PathBuf;

pub struct Package {
    pub dir: PathBuf,
    /// Path to the package.json5 file
    pub spec_path: PathBuf,
    pub spec: PackageSpec,
    /// Paths of binaries to be linked
    pub artifacts: Vec<PathBuf>,
}

#[derive(Debug, PartialEq, Deserialize, Clone)]
pub struct PackageSpec {
    pub name: String,
    pub version: String, // TODO: parse
    pub rust_libs: Option<Vec<String>>,
    //#[serde(default)]
    //pub deps: Vec<String>, // TODO: parse
}

impl Package {
    /// Load the package.json5 file from the given path.
    /// If the path is a file, it will be used as the package.json5 file.
    pub fn new(cli: &Cli, path: &PathBuf) -> Result<Self> {
        let (spec_path, spec) = load_spec(path)?;
        let artifacts = spec
            .rust_libs
            .as_ref()
            .map(|libs| {
                libs.iter()
                    .flat_map(|lib| {
                        vec![
                            cli.rust_artifact_path(&spec, lib),
                            cli.lib_artifact_path(&spec),
                        ]
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Package {
            dir: spec_path.parent().unwrap().to_path_buf(),
            spec_path,
            spec,
            artifacts,
        })
    }

    /// Load the `core` package in $SHIIKA_ROOT.
    pub fn load_core(cli: &Cli) -> Result<Self> {
        Self::new(cli, &cli.shiika_root.clone().join("packages").join("core"))
    }

    /// True if this package is the `core` package.
    pub fn is_core(&self) -> bool {
        self.spec.name == "core"
    }

    pub fn entry_point(&self) -> PathBuf {
        self.dir.join("index.sk")
    }

    pub fn export_files(&self) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = vec![];
        if let Some(libs) = self.spec.rust_libs.as_ref() {
            v.append(
                &mut libs
                    .iter()
                    .map(|s| self.rust_exports_json5_path(s))
                    .collect(),
            );
        }
        v
    }

    fn rust_exports_json5_path(&self, rust_lib: &str) -> PathBuf {
        self.dir.join(rust_lib).join("exports.json5")
    }
}

/// Returns the path of the package.json5 file and the parsed PackageSpec.
fn load_spec(path: &PathBuf) -> Result<(PathBuf, PackageSpec)> {
    let package_json5_path = if path.is_file() {
        path.clone()
    } else {
        path.join("package.json5")
    };
    let spec = load_package_json5(&package_json5_path)?;
    Ok((package_json5_path, spec))
}

fn load_package_json5(path: &PathBuf) -> Result<PackageSpec> {
    let mut f = std::fs::File::open(path).context(format!("{} not found", path.display()))?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .context(format!("failed to read {}", path.display()))?;
    let spec: PackageSpec =
        json5::from_str(&contents).context(format!("failed to parse {}", path.display()))?;
    Ok(spec)
}

pub fn load_exports_json5(json_path: &std::path::Path) -> Result<Vec<(String, String, bool)>> {
    let mut f =
        std::fs::File::open(json_path).context(format!("{} not found", json_path.display()))?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .context(format!("failed to read {}", json_path.display()))?;
    json5::from_str(&contents).context(format!("{} is broken", json_path.display()))
}
