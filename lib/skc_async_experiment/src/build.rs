pub mod cargo_builder;
pub mod compiler;
pub mod exe_builder;
pub mod lib_builder;
pub mod linker;
pub mod loader;
pub mod package_builder;
use crate::package::Package;
use std::path::Path;

/// Represents what to compile. (an executable or a library)
pub struct CompileTarget<'a> {
    /// Path to the first .sk file to read
    pub entry_point: &'a Path,
    /// Directory to create the artifact
    pub out_dir: &'a Path,
    /// Direct dependencies
    pub deps: &'a [Package],
    /// Lib or Bin specific information
    pub detail: CompileTargetDetail<'a>,
}

pub enum CompileTargetDetail<'a> {
    Lib {
        package: &'a Package,
    },
    Bin {
        package: Option<&'a Package>,
        /// Topologically sorted indirect dependencies
        total_deps: Vec<String>,
    },
}

impl<'a> CompileTarget<'a> {
    pub fn is_bin(&self) -> bool {
        matches!(self.detail, CompileTargetDetail::Bin { .. })
    }

    fn package(&self) -> Option<&'a Package> {
        match &self.detail {
            CompileTargetDetail::Lib { package } => Some(package),
            CompileTargetDetail::Bin { package, .. } => package.clone(),
        }
    }

    fn is_core_package(&self) -> bool {
        self.package().map_or(false, |pkg| pkg.is_core())
    }

    pub fn package_name(&self) -> Option<String> {
        self.package().map(|x| x.spec.name.clone())
    }
}
