use shiika_ast::LocationSpan;
use shiika_core::ty::TermTy;
use skc_error::Label;
use skc_hir::MethodSignature;

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

pub fn method_not_found(msg: String, locs: &LocationSpan) -> anyhow::Error {
    let report = skc_error::build_report(msg.clone(), locs, |r, locs_span| {
        r.with_label(Label::new(locs_span).with_message(msg))
    });
    program_error(report)
}

pub fn unknown_barename(name: &str, locs: &LocationSpan) -> anyhow::Error {
    let msg = format!("variable or method `{}' was not found", name);
    let report = skc_error::build_report(msg.clone(), locs, |r, locs_span| {
        r.with_label(Label::new(locs_span).with_message(msg))
    });
    program_error(report)
}

pub fn unspecified_arg(
    param_name: &str,
    sig: &MethodSignature,
    locs: &LocationSpan,
) -> anyhow::Error {
    let msg = format!("missing argument `{}' of method `{}'", param_name, sig);
    let report = skc_error::build_report(msg.clone(), locs, |r, locs_span| {
        r.with_label(Label::new(locs_span).with_message(msg))
    });
    program_error(report)
}

pub fn extranous_arg(name: &str, sig: &MethodSignature, locs: &LocationSpan) -> anyhow::Error {
    let msg = format!("extranous argument `{}' of method `{}'", name, sig);
    let report = skc_error::build_report(msg.clone(), locs, |r, locs_span| {
        r.with_label(Label::new(locs_span).with_message(msg))
    });
    program_error(report)
}

pub fn named_arg_for_lambda(name: &str, locs: &LocationSpan) -> anyhow::Error {
    let msg = format!(
        "you cannot pass named argument (`{}') to a lambda (may be fixed in the future though.)",
        name
    );
    let report = skc_error::build_report(msg.clone(), locs, |r, locs_span| {
        r.with_label(Label::new(locs_span).with_message(msg))
    });
    program_error(report)
}

pub fn method_call_tyinf_failed(detail: String, locs: &LocationSpan) -> anyhow::Error {
    let report =
        skc_error::build_report("Type inference failed".to_string(), locs, |r, locs_span| {
            r.with_label(Label::new(locs_span.clone()).with_message(detail))
        });
    program_error(report)
}

pub fn not_a_class_expression(ty: &TermTy, locs: &LocationSpan) -> anyhow::Error {
    let detail = format!("{}", ty);
    let report = skc_error::build_report(
        format!("Expected a class but this is {}", ty),
        locs,
        |r, locs_span| r.with_label(Label::new(locs_span).with_message(detail)),
    );
    program_error(report)
}

pub fn if_clauses_type_mismatch(
    then_ty: &TermTy,
    else_ty: &TermTy,
    then_locs: LocationSpan,
    else_locs: LocationSpan,
) -> anyhow::Error {
    let main_msg = "if clauses type mismatch".to_string();
    let report = skc_error::report_builder()
        .annotate(then_locs.clone(), then_ty.to_string())
        .annotate(else_locs, else_ty.to_string())
        .build(main_msg, &then_locs);
    type_error(report)
}
