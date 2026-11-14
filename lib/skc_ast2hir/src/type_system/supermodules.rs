//! Check that a class which implements a module also conforms to the
//! prerequisite modules of it.
//! eg. given `module Comparable<X> : Equatable<X>`,
//! `implement Comparable<Foo>` in `class Foo` is an error unless `Foo` (or its
//! ancestor) implements `Equatable<Foo>`.
use crate::class_dict::{when_body_tyargs, ClassDict};
use crate::error;
use anyhow::Result;
use shiika_core::ty;
use skc_hir::SkType;

/// Check all the classes. Called after indexing.
pub fn check_sk_types(dict: &ClassDict) -> Result<()> {
    for sk_type in dict.sk_types.types.values() {
        let SkType::Class(sk_class) = sk_type else {
            continue;
        };
        if sk_class.includes.is_empty() {
            continue;
        }
        // eg. `Foo<T>` of `class Foo<T>`
        let class_ty = ty::nonmeta(
            &sk_class.fullname().0,
            sk_class
                .base
                .typarams
                .iter()
                .enumerate()
                .map(|(i, tp)| ty::typaram_ref(tp, ty::TyParamKind::Class, i).into_term_ty())
                .collect(),
        );
        // Ancestors are checked on their own; only check the class's own includes
        for inc in &sk_class.includes {
            let conds = sk_class.include_conds_of(inc);
            // For `implement` in `when`, check under the conditions
            // (eg. `Array<T>` where `T: Comparable`)
            let self_ty = if conds.is_empty() {
                class_ty.clone()
            } else {
                class_ty.substitute_with_self(&when_body_tyargs(
                    &sk_class.base.typarams,
                    conds,
                    false,
                ))
            };
            let sk_module = dict.get_module(&inc.erasure().to_module_fullname());
            for req in sk_module.specialized_supers(inc.type_args()) {
                if !dict.conforms(&self_ty, &req.to_term_ty()) {
                    let in_when = if conds.is_empty() {
                        String::new()
                    } else {
                        let c = conds.iter().map(|c| c.to_string()).collect::<Vec<_>>();
                        format!(" (when {})", c.join(", "))
                    };
                    return Err(error::program_error(format!(
                        "class `{}' implements `{}'{} but it requires `{}'\n  \
                         hint: add `implement {}' to `{}'",
                        sk_class.fullname(),
                        inc.to_term_ty(),
                        in_when,
                        req.to_term_ty(),
                        req.to_term_ty(),
                        sk_class.fullname()
                    )));
                }
            }
        }
    }
    Ok(())
}
