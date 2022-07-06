// Resolve "require"
use anyhow::{Context, Result};
use shiika_parser::SourceFile;
use std::fs;
use std::path::{Path, PathBuf};

/// Read a .sk file (and those require'd by it)
pub fn load(path: &Path) -> Result<Vec<SourceFile>> {
    let mut files = vec![];
    let mut loading_files = vec![];
    load_file(path, &mut files, &mut loading_files)?;
    Ok(files)
}

fn load_file(path: &Path, files: &mut Vec<SourceFile>, loading_files: &mut Vec<PathBuf>) -> Result<()> {
    if loading_files.contains(&path.into()) {
        return Ok(());
    }
    loading_files.push(path.into());

    // Load require'd files first
    let content = fs::read_to_string(path).context(format!("failed to load {}", path.display()))?;
    let newpaths = resolve_requires(path, &content);
    for newpath in newpaths {
        load_file(&newpath, files, loading_files)?;
    }

    files.push(SourceFile::new(path.into(), content));
    Ok(())
}

/// Read require'd files into `files`
fn resolve_requires(path: &Path, content: &str) -> Vec<PathBuf> {
    let mut paths = vec![];
    for line in content.lines() {
        if line.trim_start().starts_with("require") {
            paths.push(parse_require(line, path));
        } else {
            break;
        }
    }
    paths
}

/// Expand filepath in require
fn parse_require(line: &str, path: &Path) -> PathBuf {
    let s = line
        .trim_start()
        .trim_start_matches("require")
        .trim_start()
        .trim_start_matches('"')
        .trim_end()
        .trim_end_matches('"');
    path.with_file_name(s)
}
