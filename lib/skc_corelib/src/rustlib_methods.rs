use anyhow::{Context, Result};
use json5;
use shiika_ast::AstMethodSignature;
use shiika_core::names::{class_fullname, ClassFullname};
use shiika_parser::Parser;
use std::fs;
use std::io::Read;

/// Returns signatures of corelib methods implemented in Rust
pub fn provided_methods() -> Vec<(ClassFullname, AstMethodSignature)> {
    load_methods_json()
        .unwrap()
        .iter()
        .map(parse_signature)
        .collect()
}

// Read provided_methods.json
fn load_methods_json() -> Result<Vec<(String, String)>> {
    let mut f = fs::File::open("lib/skc_rustlib/provided_methods.json5")
        .context("./lib/skc_rustlib/provided_methods.json5 not found")?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .context("failed to read provided_methods.json5")?;
    json5::from_str(&contents).context("provided_methods.json5 is broken")
}

// Parse signature into AstMethodSignature
fn parse_signature(item: &(String, String)) -> (ClassFullname, AstMethodSignature) {
    let (classname, sig_str) = item;
    let ast_sig = Parser::parse_signature(sig_str).unwrap();
    (class_fullname(classname), ast_sig)
}
