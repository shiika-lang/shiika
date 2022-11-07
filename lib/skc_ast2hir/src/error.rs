use shiika_ast::LocationSpan;
use skc_error::Label;

#[derive(thiserror::Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    #[error("{msg})")]
    SyntaxError { msg: String },
    /// Errors of types
    #[error("{msg}")]
    TypeError { msg: String },
    /// Invalid name
    #[error("{msg}")]
    NameError { msg: String },
    /// Syntactically correct but invalid program
    #[error("{msg}")]
    ProgramError { msg: String },
}

pub fn syntax_error(msg: &str) -> anyhow::Error {
    Error::SyntaxError {
        msg: msg.to_string(),
    }
    .into()
}

pub fn type_error(msg: impl Into<String>) -> anyhow::Error {
    Error::TypeError { msg: msg.into() }.into()
}

pub fn name_error(msg: &str) -> anyhow::Error {
    Error::NameError {
        msg: msg.to_string(),
    }
    .into()
}

pub fn program_error(msg: impl Into<String>) -> anyhow::Error {
    Error::ProgramError { msg: msg.into() }.into()
}

pub fn lvar_redeclaration(name: &str, locs: &LocationSpan) -> anyhow::Error {
    let msg = format!(
        "variable `{}' already exists (shadowing is not allowed in Shiika)",
        name
    );
    let report = skc_error::build_report(msg.clone(), locs, |r, locs_span| {
        r.with_label(Label::new(locs_span).with_message(msg))
    });
    program_error(report)
}

pub fn assign_to_undeclared_lvar(name: &str, locs: &LocationSpan) -> anyhow::Error {
    let msg = format!(
        "variable `{}' not declared (hint: `let {} = ...`)",
        name, name
    );
    let report = skc_error::build_report(msg.clone(), locs, |r, locs_span| {
        r.with_label(Label::new(locs_span).with_message(msg))
    });
    program_error(report)
}

pub fn ivar_decl_outside_initializer(name: &str, locs: &LocationSpan) -> anyhow::Error {
    let msg = format!(
        "instance variable (`{}') can only be declared in #initialize",
        name
    );
    let report = skc_error::build_report(msg.clone(), locs, |r, locs_span| {
        r.with_label(Label::new(locs_span).with_message(msg))
    });
    program_error(report)
}

pub fn assign_to_undeclared_ivar(name: &str, locs: &LocationSpan) -> anyhow::Error {
    let msg = format!(
        "variable `{}' not declared (hint: `let {} = ...`)",
        name, name
    );
    let report = skc_error::build_report(msg.clone(), locs, |r, locs_span| {
        r.with_label(Label::new(locs_span).with_message(msg))
    });
    program_error(report)
}

