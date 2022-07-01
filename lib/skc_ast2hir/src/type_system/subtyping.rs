use crate::class_dict::ClassDict;
use shiika_core::{ty, ty::*};

/// Return true if `ty1` conforms to `ty2` i.e.
/// an object of the type `ty1` is included in the set of objects represented by the type `ty2`
#[allow(clippy::if_same_then_else)]
pub fn conforms(c: &ClassDict, ty1: &TermTy, ty2: &TermTy) -> bool {
    // `Never` is bottom type (i.e. subclass of any class)
    if ty1.is_never_type() {
        true
    } else if ty1.equals_to(ty2) {
        true
    } else if let TyBody::TyPara(ref1) = &ty1.body {
        if let TyBody::TyPara(ref2) = &ty2.body {
            ref1.upper_bound == ref2.upper_bound && ref1.lower_bound == ref2.lower_bound
        } else {
            let u1 = ref1.upper_bound.to_term_ty();
            conforms(c, &u1, ty2)
        }
    } else if let TyBody::TyPara(ref2) = &ty2.body {
        if let TyBody::TyPara(ref1) = &ty1.body {
            ref1.upper_bound == ref2.upper_bound && ref1.lower_bound == ref2.lower_bound
        } else {
            let u2 = ref2.upper_bound.to_term_ty();
            conforms(c, ty1, &u2)
        }
    } else {
        let mod1 = c.get_type(&ty1.erasure().to_type_fullname()).is_module();
        let mod2 = c.get_type(&ty2.erasure().to_type_fullname()).is_module();
        match (mod1, mod2) {
            (true, true) => false,
            (true, false) => module_conforms_to_class(c, ty1, ty2),
            (false, true) => class_conforms_to_module(c, ty1, ty2),
            (false, false) => class_conforms_to_class(c, ty1, ty2),
        }
    }
}

// Return true only if `ty2` is the top type
fn module_conforms_to_class(_c: &ClassDict, _ty1: &TermTy, ty2: &TermTy) -> bool {
    ty2.fullname.0 == "Object"
}

// Return true if `ty2` (or one of its ancestor) includes the module
fn class_conforms_to_module(c: &ClassDict, ty1: &TermTy, ty2: &TermTy) -> bool {
    ancestor_types(c, ty1).iter().any(|t| includes(c, t, ty2))
}

// Return true if `class` (eg. `Array<Int>`) includes `module` (eg. `Enumerable<Int>`)
fn includes(c: &ClassDict, class: &TermTy, module: &TermTy) -> bool {
    let sk_class = c.get_class(&class.erasure().to_class_fullname());
    sk_class.includes.iter().any(|m| {
        // eg. Make `Enumerable<Int>` from `Enumerable<T>` and `Array<Int>`
        let ms = m.ty().substitute(&class.tyargs(), Default::default());
        ms == *module
    })
}

// TODO: implement variance
fn class_conforms_to_class(c: &ClassDict, ty1: &TermTy, ty2: &TermTy) -> bool {
    let ancestors = ancestor_types(c, ty1);
    if let Some(t1) = ancestors.iter().find(|t| t.same_base(ty2)) {
        if t1.equals_to(ty2) {
            return true;
        } else if t1.tyargs().iter().all(|t| t.is_never_type()) {
            return true;
        } else {
            // Special care for void funcs
            return is_void_fn(ty2);
        }
    } else {
        false
    }
}

/// Returns if `ty` is a void-returning function (eg. `Fn1<Int, Void>`)
fn is_void_fn(ty: &TermTy) -> bool {
    if let Some(ret_ty) = ty.fn_x_info() {
        ret_ty.is_void_type()
    } else {
        false
    }
}

/// Returns the nearest common ancestor of the classes
/// Returns `None` if there is no common ancestor except `Object`, the
/// top type. However, returns `Some(Object)` when either of the arguments
/// is `Object`.
pub fn nearest_common_ancestor(c: &ClassDict, ty1: &TermTy, ty2: &TermTy) -> Option<TermTy> {
    if ty1 == ty2 {
        return Some(ty1.clone());
    }
    let t = _nearest_common_ancestor(c, ty1, ty2);
    let obj = ty::raw("Object");
    if t == obj {
        if *ty1 == obj || *ty2 == obj {
            Some(obj)
        } else {
            // No common ancestor found (except `Object`)
            None
        }
    } else {
        Some(t)
    }
}

/// Find common ancestor of two types
fn _nearest_common_ancestor(c: &ClassDict, ty1_: &TermTy, ty2_: &TermTy) -> TermTy {
    let ty1 = ty1_.upper_bound().into_term_ty();
    let ty2 = ty2_.upper_bound().into_term_ty();
    let ancestors1 = ancestor_types(c, &ty1);
    let ancestors2 = ancestor_types(c, &ty2);
    for t2 in &ancestors2 {
        let mut t = None;
        for t1 in &ancestors1 {
            if t1.equals_to(t2) {
                t = Some(t1);
                break;
            } else if t1.same_base(t2) {
                if conforms(c, t1, t2) {
                    t = Some(t2);
                    break;
                } else if conforms(c, t2, t1) {
                    t = Some(t1);
                    break;
                }
            }
        }
        if let Some(t3) = t {
            return t3.clone();
        }
    }
    panic!("[BUG] _nearest_common_ancestor not found");
}

/// Return ancestor types of `ty`, including itself.
fn ancestor_types(class_dict: &ClassDict, ty: &TermTy) -> Vec<TermTy> {
    let mut v = vec![];
    let mut t = Some(ty.clone());
    while let Some(tt) = t {
        t = class_dict.supertype(&tt);
        v.push(tt);
    }
    v
}
