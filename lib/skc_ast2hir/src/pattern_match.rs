use crate::class_expr;
use crate::error;
use crate::hir_maker::HirMaker;
use crate::hir_maker_context::HirMakerContext;
use anyhow::Result;
use shiika_ast::*;
use shiika_core::{names::*, ty, ty::*};
use skc_hir::pattern_match::{Component, MatchClause};
use skc_hir::*;

/// Convert a match expression into Hir::match_expression
pub fn convert_match_expr(
    mk: &mut HirMaker,
    cond: &AstExpression,
    ast_clauses: &[AstMatchClause],
) -> Result<(HirExpression, HirLVars)> {
    let cond_expr = mk.convert_expr(cond)?;
    let tmp_name = mk.generate_lvar_name("expr");
    let tmp_ref = Hir::lvar_ref(cond_expr.ty.clone(), tmp_name.clone());
    let mut clauses = ast_clauses
        .iter()
        .map(|clause| convert_match_clause(mk, &tmp_ref, clause))
        .collect::<Result<Vec<MatchClause>>>()?;
    let result_ty = calc_result_ty(mk, &mut clauses)?;
    let mut lvars = collect_lvars(&clauses);
    lvars.push((tmp_name.clone(), cond_expr.ty.clone()));

    let panic_msg = Hir::string_literal(mk.register_string_literal("no matching clause found"));
    clauses.push(MatchClause {
        components: vec![],
        body_hir: Hir::expressions(vec![Hir::method_call(
            ty::raw("Never"),
            Hir::decimal_literal(0), // whatever.
            method_fullname_raw("Object", "panic"),
            vec![panic_msg],
        )]),
    });

    let tmp_assign = Hir::lvar_assign(&tmp_name, cond_expr);
    Ok((Hir::match_expression(result_ty, tmp_assign, clauses), lvars))
}

/// Convert a match clause into a big `if` expression
fn convert_match_clause(
    mk: &mut HirMaker,
    value: &HirExpression,
    (pat, body): &(AstPattern, Vec<AstExpression>),
) -> Result<MatchClause> {
    let components = convert_match(mk, value, pat)?;
    let body_hir = compile_body(mk, &components, body)?;
    Ok(MatchClause {
        components,
        body_hir,
    })
}

/// Compile clause body into HIR
fn compile_body(
    mk: &mut HirMaker,
    components: &[Component],
    body: &[AstExpression],
) -> Result<HirExpressions> {
    mk.ctx_stack.push(HirMakerContext::match_clause());
    // Declare lvars introduced by matching
    for component in components {
        if let Component::Bind(name, expr) = component {
            let readonly = true;
            mk.ctx_stack.declare_lvar(name, expr.ty.clone(), readonly);
        }
    }
    let hir_exprs = mk.convert_exprs(body)?;
    mk.ctx_stack.pop_match_clause_ctx();
    Ok(hir_exprs)
}

/// Calculate the type of the match expression from clauses
fn calc_result_ty(mk: &HirMaker, clauses: &mut [MatchClause]) -> Result<TermTy> {
    debug_assert!(!clauses.is_empty());
    let mut clauses = clauses
        .iter_mut()
        .filter(|c| !c.body_hir.ty.is_never_type())
        .collect::<Vec<_>>();
    if clauses.iter().any(|c| c.body_hir.ty.is_void_type()) {
        for c in clauses.iter_mut() {
            if !c.body_hir.ty.is_void_type() {
                c.body_hir.voidify();
            }
        }
        Ok(ty::raw("Void"))
    } else {
        let mut ty = clauses[0].body_hir.ty.clone();
        for c in &clauses {
            if let Some(t) = mk.class_dict.nearest_common_ancestor(&ty, &c.body_hir.ty) {
                ty = t;
            } else {
                return Err(error::type_error("match clause type mismatch"));
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
    tmp = tmp.bitcast_to(ty);
    std::mem::swap(&mut tmp, &mut c.body_hir);
}

fn collect_lvars(clauses: &[MatchClause]) -> Vec<(String, TermTy)> {
    let mut lvars = vec![];
    for clause in clauses {
        for component in &clause.components {
            if let Component::Bind(name, expr) = component {
                lvars.push((name.to_string(), expr.ty.clone()));
            }
        }
    }
    lvars
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
        AstPattern::IntegerLiteralPattern(i) => {
            if value.ty != ty::raw("Int") {
                return Err(error::type_error(&format!(
                    "expr of `{}' never matches to `Int'",
                    value.ty,
                )));
            }
            let test = Hir::method_call(
                ty::raw("Bool"),
                value.clone(),
                method_fullname_raw("Int", "=="),
                vec![Hir::decimal_literal(*i)],
            );
            Ok(vec![Component::Test(test)])
        }
        _ => todo!(),
    }
}

/// Create components for match against extractor pattern
fn convert_extractor(
    mk: &mut HirMaker,
    value: &HirExpression,
    names: &[String],
    param_patterns: &[AstPattern],
) -> Result<Vec<Component>> {
    // eg. `ty::raw("Maybe::Some")`
    let base_ty = mk
        .resolve_class_expr(&UnresolvedConstName(names.to_vec()))?
        .ty
        .instance_ty();
    let pat_ty = match &value.ty.body {
        TyBody::TyRaw(LitTy { type_args, .. }) => ty::spe(&base_ty.fullname.0, type_args.clone()),
        _ => base_ty.clone(),
    };
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

fn class_props(mk: &HirMaker, cls: &TermTy) -> Result<Vec<(String, TermTy)>> {
    let found =
        mk.class_dict
            .lookup_method(cls, &method_firstname("initialize"), Default::default())?;
    Ok(found
        .sig
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
            "this match needs {} patterns but {} there",
            ivars.len(),
            patterns.len()
        )));
    }
    let mut components = vec![];
    for i in 0..ivars.len() {
        let (name, ty) = &ivars[i];
        // eg. `value.foo`
        let ivar_ref = Hir::method_call(
            ty.clone(),
            value.clone(),
            method_fullname(&pat_ty.base_class_name(), name),
            vec![],
        );
        components.append(&mut convert_match(mk, &ivar_ref, &patterns[i])?);
    }
    Ok(components)
}

/// Create `expr.class == cls`
fn test_class(mk: &mut HirMaker, value: &HirExpression, pat_ty: &TermTy) -> HirExpression {
    let cls_ref = class_expr(mk, pat_ty);
    Hir::method_call(
        ty::raw("Bool"),
        Hir::method_call(
            ty::raw("Class"),
            value.clone(),
            method_fullname_raw("Object", "class"),
            vec![],
        ),
        method_fullname_raw("Class", "=="),
        vec![cls_ref],
    )
}
