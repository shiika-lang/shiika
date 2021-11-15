use anyhow::{Context, Result};
use std::fs;
use std::io::Read;
use shiika_parser::Parser;
use shiika_ast::AstMethodSignature;
use shiika_core::names::{class_fullname, ClassFullname};

/// Returns signatures of corelib methods implemented in Rust
pub fn provided_methods() -> Vec<(ClassFullname, AstMethodSignature)> {
    load_methods_json().unwrap().iter().map(parse_signature).collect()
}

// Read provided_methods.json
fn load_methods_json() -> Result<Vec<(String, String)>> {
    let mut f = fs::File::open("lib/skc_rustlib/provided_methods.json")
        .context("./lib/skc_rustlib/provided_methods.json not found")?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .context("failed to read provided_methods.json")?;
    serde_json::from_str(&contents).context("provided_methods.json is broken")
}

// Parse signature into AstMethodSignature
fn parse_signature(item: &(String, String)) -> (ClassFullname, AstMethodSignature) {
    let (classname, sig_str) = item;
    let ast_sig = Parser::parse_signature(sig_str).unwrap();
    (class_fullname(classname), ast_sig)
}

