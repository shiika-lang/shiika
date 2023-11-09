use crate::error;
use crate::hir_maker::extract_lvars;
use crate::hir_maker::HirMaker;
use crate::hir_maker_context::HirMakerContext;
use anyhow::Result;
use shiika_ast::*;
use shiika_core::{names::*, ty, ty::*};
use skc_hir::pattern_match::{Component, MatchClause};
use skc_hir::*;

pub fn expand_result_try(
    mk: &mut HirMaker,
    receiver_hir: HirExpression,
    locs: &LocationSpan,
) -> Result<HirExpression> {
    let tmp_name = mk.generate_lvar_name("expr");
    let tmp_ref = Hir::lvar_ref(receiver_hir.ty.clone(), tmp_name.clone(), locs.clone());

    // `when Ok(v) then v`
    let ok_tmp = mk.generate_lvar_name("v");
    let ok_pattern = AstPattern::ExtractorPattern {
        names: vec!["Ok".to_string()],
        params: vec![AstPattern::VariablePattern(ok_tmp.clone())],
    };
    let ok_body = AstExpression {
        body: AstExpressionBody::BareName(ok_tmp),
        primary: true,
        locs: locs.clone(),
    };
    let ok_clause = (ok_pattern, vec![ok_body]);

    // `when e then return e.unsafe_cast(Fail)`
    let err_tmp = mk.generate_lvar_name("e");
    let err_pattern = AstPattern::VariablePattern(err_tmp.clone());
    let err_ref = AstExpression {
        body: AstExpressionBody::BareName(err_tmp),
        primary: true,
        locs: locs.clone(),
    };
    let the_err = AstExpression {
        body: AstExpressionBody::CapitalizedName(UnresolvedConstName(vec!["Fail".to_string()])),
        primary: true,
        locs: locs.clone(),
    };
    let err_cast = AstExpression {
        body: AstExpressionBody::MethodCall(AstMethodCall {
            receiver_expr: Some(Box::new(err_ref)),
            method_name: method_firstname("unsafe_cast"),
            args: AstCallArgs::single_unnamed(the_err),
            type_args: Default::default(),
            may_have_paren_wo_args: false,
        }),
        primary: true,
        locs: locs.clone(),
    };
    let err_body = AstExpression {
        body: AstExpressionBody::Return {
            arg: Some(Box::new(err_cast)),
        },
        primary: false,
        locs: locs.clone(),
    };
    let err_clause = (err_pattern, vec![err_body]);

    let clauses = vec![
        convert_match_clause(mk, &tmp_ref, &ok_clause)?,
        convert_match_clause(mk, &tmp_ref, &err_clause)?,
    ];

    _convert_match_expr(mk, tmp_name, receiver_hir, clauses)
}

/// Convert a match expression into Hir::match_expression
pub fn convert_match_expr(
    mk: &mut HirMaker,
    cond: &AstExpression,
    ast_clauses: &[AstMatchClause],
) -> Result<HirExpression> {
    let cond_expr = mk.convert_expr(cond)?;
    let tmp_name = mk.generate_lvar_name("expr");
    let tmp_ref = Hir::lvar_ref(cond_expr.ty.clone(), tmp_name.clone(), LocationSpan::todo());
    let mut clauses = ast_clauses
        .iter()
        .map(|clause| convert_match_clause(mk, &tmp_ref, clause))
        .collect::<Result<Vec<MatchClause>>>()?;
    let panic_msg = Hir::string_literal(
        mk.register_string_literal("no matching clause found"),
        LocationSpan::todo(),
    );
    clauses.push(MatchClause {
        components: vec![],
        body_hir: Hir::expressions(vec![Hir::method_call(
            ty::raw("Never"),
            Hir::decimal_literal(0, LocationSpan::todo()), // whatever.
            method_fullname_raw("Object", "panic"),
            vec![panic_msg],
            Default::default(),
        )]),
        lvars: Default::default(),
    });

    _convert_match_expr(mk, tmp_name, cond_expr, clauses)
}
pub fn _convert_match_expr(
    mk: &mut HirMaker,
    tmp_name: String,
    cond_expr: HirExpression,
    mut clauses: Vec<MatchClause>,
) -> Result<HirExpression> {
    let lvar = HirLVar {
        name: tmp_name.clone(),
        ty: cond_expr.ty.clone(),
        captured: false, // This lvar cannot be captured because its name is hidden to the programmer.
    };
    let readonly = true;
    mk.ctx_stack.declare_lvar(&lvar.name, lvar.ty, readonly);

    let result_ty = calc_result_ty(mk, &mut clauses)?;
    let tmp_assign = Hir::lvar_assign(tmp_name, cond_expr, LocationSpan::todo());
    Ok(Hir::match_expression(
        result_ty,
        tmp_assign,
        clauses,
        LocationSpan::todo(),
    ))
}

/// Convert a match clause into a big `if` expression
fn convert_match_clause(
    mk: &mut HirMaker,
    value: &HirExpression,
    (pat, body): &(AstPattern, Vec<AstExpression>),
) -> Result<MatchClause> {
    let components = convert_match(mk, value, pat)?;
    let (body_hir, lvars) = compile_body(mk, &components, body)?;
    Ok(MatchClause {
        components,
        body_hir,
        lvars,
    })
}

/// Compile clause body into HIR
fn compile_body(
    mk: &mut HirMaker,
    components: &[Component],
    body: &[AstExpression],
) -> Result<(HirExpression, HirLVars)> {
    mk.ctx_stack.push(HirMakerContext::match_clause());
    // Declare lvars introduced by matching
    for component in components {
        if let Component::Bind(name, expr) = component {
            let readonly = true;
            mk.ctx_stack.declare_lvar(name, expr.ty.clone(), readonly);
        }
    }
    let hir_exprs = mk.convert_exprs(body)?;
    let mut clause_ctx = mk.ctx_stack.pop_match_clause_ctx();
    Ok((hir_exprs, extract_lvars(&mut clause_ctx.lvars)))
}

/// Calculate the type of the match expression from clauses
fn calc_result_ty(mk: &HirMaker, clauses_: &mut [MatchClause]) -> Result<TermTy> {
    debug_assert!(!clauses_.is_empty());
    let mut clauses = clauses_
        .iter_mut()
        .filter(|c| !c.body_hir.ty.is_never_type())
        .collect::<Vec<_>>();
    if clauses.is_empty() {
        // All clauses are type `Never`.
        Ok(ty::raw("Never"))
    } else if clauses.iter().any(|c| c.body_hir.ty.is_void_type()) {
        for c in clauses.iter_mut() {
            if !c.body_hir.ty.is_void_type() {
                let mut tmp = Hir::expressions(vec![]);
                std::mem::swap(&mut tmp, &mut c.body_hir);
                c.body_hir = tmp.voidify();
            }
        }
        Ok(ty::raw("Void"))
    } else {
        let mut ty = clauses[0].body_hir.ty.clone();
        for c in &clauses {
            if let Some(t) = mk.class_dict.nearest_common_ancestor(&ty, &c.body_hir.ty) {
                ty = t;
            } else {
                return Err(error::match_clauses_type_mismatch(
                    &ty,
                    clauses[0].body_hir.locs.clone(),
                    &c.body_hir.ty,
                    c.body_hir.locs.clone(),
                ));
            }
        }
        for c in clauses.iter_mut() {
            if !c.body_hir.ty.equals_to(&ty) {
                bitcast_match_clause_body(c, ty.clone());
            }
        }
        Ok(ty)
    }
}

/// Destructively bitcast body_hir
fn bitcast_match_clause_body(c: &mut MatchClause, ty: TermTy) {
    let mut tmp = Hir::expressions(Default::default());
    std::mem::swap(&mut tmp, &mut c.body_hir);
    tmp = Hir::bit_cast(ty, tmp);
    std::mem::swap(&mut tmp, &mut c.body_hir);
}

/// Create components for match against a pattern
fn convert_match(
    mk: &mut HirMaker,
    value: &HirExpression,
    pat: &AstPattern,
) -> Result<Vec<Component>> {
    match &pat {
        AstPattern::ExtractorPattern { names, params } => {
            convert_extractor(mk, value, names, params)
        }
        AstPattern::VariablePattern(name) => {
            if name == "_" {
                Ok(vec![])
            } else {
                Ok(vec![Component::Bind(name.to_string(), value.clone())])
            }
        }
        AstPattern::BooleanLiteralPattern(b) => {
            check_ty_raw(value, "Bool")?;
            let hir_bool = Hir::boolean_literal(*b, LocationSpan::todo());
            Ok(vec![make_eq_test(value, "Bool", hir_bool)])
        }
        AstPattern::IntegerLiteralPattern(i) => {
            check_ty_raw(value, "Int")?;
            let hir_int = Hir::decimal_literal(*i, LocationSpan::todo());
            Ok(vec![make_eq_test(value, "Int", hir_int)])
        }
        AstPattern::FloatLiteralPattern(f) => {
            check_ty_raw(value, "Float")?;
            let hir_int = Hir::float_literal(*f, LocationSpan::todo());
            Ok(vec![make_eq_test(value, "Float", hir_int)])
        }
        AstPattern::StringLiteralPattern(s) => {
            check_ty_raw(value, "String")?;
            let hir_str = mk.convert_string_literal(s, &LocationSpan::todo());
            Ok(vec![make_eq_test(value, "String", hir_str)])
        }
    }
}

/// Check the type of `value` is `ty::raw(name)`
fn check_ty_raw(value: &HirExpression, name: &str) -> Result<()> {
    if value.ty != ty::raw(name) {
        return Err(error::type_error(&format!(
            "expr of `{}' never matches to `{}'",
            value.ty, name
        )));
    }
    Ok(())
}

/// Make `lhs == rhs`
fn make_eq_test(value: &HirExpression, name: &str, rhs: HirExpression) -> Component {
    let test = Hir::method_call(
        ty::raw("Bool"),
        value.clone(),
        method_fullname_raw(name, "=="),
        vec![rhs],
        Default::default(),
    );
    Component::Test(test)
}

/// Create components for match against extractor pattern
fn convert_extractor(
    mk: &mut HirMaker,
    value: &HirExpression,
    names: &[String],
    param_patterns: &[AstPattern],
) -> Result<Vec<Component>> {
    // eg. `ty::raw("Maybe::Some")`
    let pat_base_ty = get_base_ty(mk, names)?;
    let pat_ty = infer_pat_ty(mk, &pat_base_ty, &value.ty);
    if !mk.class_dict.conforms(&pat_ty, &value.ty) {
        return Err(error::type_error(&format!(
            "expr of `{}' never matches to `{}'",
            &value.ty, pat_ty
        )));
    }
    let cast_value = Hir::bit_cast(pat_ty.clone(), value.clone());
    let mut components = extract_props(mk, &cast_value, &pat_ty, param_patterns)?;

    let test = Component::Test(test_class(mk, value, &pat_ty));
    components.insert(0, test);
    Ok(components)
}

fn get_base_ty(mk: &mut HirMaker, names: &[String]) -> Result<Erasure> {
    let expr =
        mk.convert_capitalized_name(&UnresolvedConstName(names.to_vec()), &LocationSpan::todo())?;
    if expr.ty.is_metaclass() || expr.ty.is_typaram_ref() {
        return Ok(expr.ty.instance_ty().erasure());
    }
    if let Some(cls) = mk
        .class_dict
        .lookup_class(&expr.ty.fullname.to_class_fullname())
    {
        if cls.const_is_obj {
            return Ok(expr.ty.erasure()); // eg. Void, None, etc.
        }
    }
    Err(error::type_error(&format!(
        "a class expected but got {:?}",
        &expr.ty
    )))
}

// Infer pattern type. eg. for `when Pair(a, b)`, infer the types of
// `a` and `b` from the type of the value to match.
fn infer_pat_ty(mk: &mut HirMaker, pat_base_ty: &Erasure, value_ty: &TermTy) -> TermTy {
    match &value_ty.body {
        TyBody::TyRaw(LitTy { type_args, .. }) => {
            let sk_type = mk.class_dict.get_type(&pat_base_ty.to_type_fullname());
            sk_type.term_ty().substitute(type_args, &[])
        }
        _ => pat_base_ty.to_term_ty(),
    }
}

fn class_props(mk: &HirMaker, cls: &TermTy) -> Result<Vec<(String, TermTy)>> {
    let found = mk.class_dict.lookup_method(
        cls,
        &method_firstname("initialize"),
        &LocationSpan::internal(),
    )?;
    let sig = found.sig.specialize(&cls.tyargs(), Default::default());
    Ok(sig
        .params
        .iter()
        .map(|x| (x.name.to_string(), x.ty.clone()))
        .collect())
}

/// Create components for each param of an extractor pattern
fn extract_props(
    mk: &mut HirMaker,
    value: &HirExpression,
    pat_ty: &TermTy,
    patterns: &[AstPattern],
) -> Result<Vec<Component>> {
    let ivars = class_props(mk, pat_ty)?; // eg. ("value", ty::spe("Maybe", "Int"))
    if ivars.len() != patterns.len() {
        return Err(error::program_error(&format!(
            "this match for {} needs {} pattern(s) but {} there",
            pat_ty,
            ivars.len(),
            patterns.len()
        )));
    }
    let mut components = vec![];
    for i in 0..ivars.len() {
        let (name_, ty) = &ivars[i];
        let name = name_.replace('@', "");
        // eg. `value.foo`
        let ivar_ref = Hir::method_call(
            ty.clone(),
            value.clone(),
            method_fullname(pat_ty.base_class_name().into(), name),
            vec![],
            Default::default(),
        );
        components.append(&mut convert_match(mk, &ivar_ref, &patterns[i])?);
    }
    Ok(components)
}

/// Create `expr.class == cls`
/// If the pattern is a constant enum case (eg. `Maybe::None`), create
/// `Object#==(expr, None)` instead.
fn test_class(mk: &mut HirMaker, value: &HirExpression, pat_ty: &TermTy) -> HirExpression {
    let pat_erasure = pat_ty.erasure();
    let t = mk.class_dict.get_class(&pat_erasure.to_class_fullname());
    if t.const_is_obj {
        let const_ref = Hir::const_ref(
            pat_ty.clone(),
            pat_ty.fullname.to_const_fullname(),
            LocationSpan::todo(),
        );
        Hir::method_call(
            ty::raw("Bool"),
            const_ref,
            method_fullname_raw("Object", "=="),
            vec![value.clone()],
            Default::default(),
        )
    } else {
        let cls_ref =
            mk.get_class_object(&pat_erasure.to_term_ty().meta_ty(), &LocationSpan::todo());
        // value.class.erasure_class == Foo
        Hir::method_call(
            ty::raw("Bool"),
            Hir::method_call(
                ty::raw("Class"),
                Hir::method_call(
                    ty::raw("Class"),
                    value.clone(),
                    method_fullname_raw("Object", "class"),
                    vec![],
                    Default::default(),
                ),
                method_fullname_raw("Class", "erasure_class"),
                vec![],
                Default::default(),
            ),
            method_fullname_raw("Class", "=="),
            vec![cls_ref],
            Default::default(),
        )
    }
}
