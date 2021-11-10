use anyhow::{Context, Result};
use shiika_ast::AstMethodSignature;
use shiika_core::names::{class_fullname, ClassFullname};
use shiika_parser::Parser;
use std::fs;
use std::io::Read;

/// Read provided_methods.json
pub fn parse_rustlib_exports() -> Result<Vec<(ClassFullname, AstMethodSignature)>> {
    let mut f = fs::File::open("lib/skc_rustlib/provided_methods.json")
        .context("builtin exports not found")?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .context("failed to read provided_methods.json")?;
    let lines: Vec<(String, String)> =
        serde_json::from_str(&contents).context("provided_methods.json is broken")?;
    let methods = lines.iter().map(parse_signature).collect();
    Ok(methods)
}

fn parse_signature(item: &(String, String)) -> (ClassFullname, AstMethodSignature) {
    let (classname, sig_str) = item;
    let ast_sig = Parser::parse_signature(sig_str).unwrap();
    (class_fullname(classname), ast_sig)
}
