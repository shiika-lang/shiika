use crate::mir;
use crate::names::FunctionName;
use shiika_core::names::{ClassFullname, MethodFirstname, MethodFullname, ModuleFullname};
use skc_hir::{MethodSignature, SkClass, SkTypes};

/// Returns the signature of a module method, as declared in the module.
pub fn module_method_sig<'a>(
    sk_types: &'a SkTypes,
    imported_sk_types: &'a SkTypes,
    module: &ModuleFullname,
    method_name: &MethodFirstname,
) -> &'a MethodSignature {
    let fullname = module.to_type_fullname().method_fullname(method_name);
    find_sig(sk_types, imported_sk_types, &fullname)
}

fn find_sig<'a>(
    sk_types: &'a SkTypes,
    imported_sk_types: &'a SkTypes,
    fullname: &MethodFullname,
) -> &'a MethodSignature {
    sk_types
        .get_sig(fullname)
        .or_else(|| imported_sk_types.get_sig(fullname))
        .unwrap_or_else(|| panic!("[BUG] method not found: {}", fullname))
}

/// The ABI of calling a wtable slot. It is determined by the module's declared
/// signature, so the module's type parameters are erased (eg. `N<X>#get -> X`
/// returns a pointer even when called on `N<Int>`).
pub fn wtable_fun_ty(module_sig: &MethodSignature) -> mir::FunTy {
    super::build_fun_ty(module_sig)
}

/// Returns the function to put in the wtable slot `idx` of `module` of
/// `sk_class`. Usually it is the implementation itself, but when the ABI of
/// the implementation differs from that of the slot (eg. `class F : N<Int>`
/// with `def get -> Int`, which returns a raw i64), returns the adapter which
/// converts the representation.
pub fn slot_func_name(
    sk_types: &SkTypes,
    imported_sk_types: &SkTypes,
    sk_class: &SkClass,
    module: &ModuleFullname,
    idx: usize,
) -> FunctionName {
    let (module_sig, impl_sig) = slot_sigs(sk_types, imported_sk_types, sk_class, module, idx);
    if needs_adapter(module_sig, impl_sig) {
        adapter_name(&sk_class.fullname(), module, idx)
    } else {
        FunctionName::from_sig(impl_sig)
    }
}

/// Build the adapters needed for the wtables of the classes in `sk_types`.
pub fn adapter_funcs(sk_types: &SkTypes, imported_sk_types: &SkTypes) -> Vec<mir::Function> {
    let mut funcs = vec![];
    for sk_class in sk_types.sk_classes() {
        for (module, methods) in &sk_class.wtable.0 {
            for idx in 0..methods.len() {
                let (module_sig, impl_sig) =
                    slot_sigs(sk_types, imported_sk_types, sk_class, module, idx);
                if needs_adapter(module_sig, impl_sig) {
                    let name = adapter_name(&sk_class.fullname(), module, idx);
                    funcs.push(build_adapter(name, module_sig, impl_sig));
                }
            }
        }
    }
    funcs
}

/// Returns the signature of the module method and that of its implementation
/// for a wtable slot.
fn slot_sigs<'a>(
    sk_types: &'a SkTypes,
    imported_sk_types: &'a SkTypes,
    sk_class: &SkClass,
    module: &ModuleFullname,
    idx: usize,
) -> (&'a MethodSignature, &'a MethodSignature) {
    let module_type = sk_types
        .types
        .get(&module.to_type_fullname())
        .or_else(|| imported_sk_types.types.get(&module.to_type_fullname()))
        .unwrap_or_else(|| panic!("[BUG] module not found: {}", module));
    let (module_sig, _) = module_type.base().method_sigs.to_ordered()[idx];
    let impl_name = &sk_class.wtable.0[module][idx];
    (module_sig, find_sig(sk_types, imported_sk_types, impl_name))
}

/// True if the implementation uses a raw immediate where the slot uses a
/// boxed value, or vice versa.
fn needs_adapter(module_sig: &MethodSignature, impl_sig: &MethodSignature) -> bool {
    let slot = wtable_fun_ty(module_sig);
    let imp = super::build_fun_ty(impl_sig);
    slot.param_tys
        .iter()
        .zip(imp.param_tys.iter())
        .any(|(a, b)| a.is_immediate() != b.is_immediate())
        || slot.ret_ty.is_immediate() != imp.ret_ty.is_immediate()
}

fn adapter_name(cls: &ClassFullname, module: &ModuleFullname, idx: usize) -> FunctionName {
    FunctionName::mangled(format!("shiika_wtable_adapter_{}_{}_{}", cls, module, idx))
}

/// Build a function which takes the arguments in the slot's ABI, converts them
/// to the implementation's ABI, calls it and converts the result back.
fn build_adapter(
    name: FunctionName,
    module_sig: &MethodSignature,
    impl_sig: &MethodSignature,
) -> mir::Function {
    let slot = wtable_fun_ty(module_sig);
    let imp = super::build_fun_ty(impl_sig);
    let coerce = |e: mir::TypedExpr, target: &mir::Ty| -> mir::TypedExpr {
        if &e.1 != target {
            mir::Expr::cast(mir::CastType::Force(target.clone()), e)
        } else {
            e
        }
    };
    let params = slot
        .param_tys
        .iter()
        .enumerate()
        .map(|(i, ty)| mir::Param::new(ty.clone(), format!("a{}", i)))
        .collect::<Vec<_>>();
    let args = slot
        .param_tys
        .iter()
        .zip(imp.param_tys.iter())
        .enumerate()
        .map(|(i, (slot_ty, imp_ty))| {
            coerce(
                mir::Expr::arg_ref(i, format!("a{}", i), slot_ty.clone()),
                imp_ty,
            )
        })
        .collect();
    let func = mir::Expr::func_ref(FunctionName::from_sig(impl_sig), imp.clone());
    let call = coerce(mir::Expr::fun_call(func, args), &slot.ret_ty);
    mir::Function {
        asyncness: impl_sig.asyncness.clone().into(),
        name,
        params,
        ret_ty: (*slot.ret_ty).clone(),
        body_stmts: mir::Expr::return_(call),
        sig: None,
        lvar_count: None,
    }
}

pub fn inserter_funcs(sk_types: &SkTypes, imported_sk_types: &SkTypes) -> Vec<mir::Function> {
    let mut funcs = vec![];
    funcs.push(main_inserter(sk_types, imported_sk_types));
    for sk_class in sk_types.sk_classes().chain(imported_sk_types.sk_classes()) {
        if !sk_class.wtable.is_empty() {
            funcs.push(inserter_func(sk_class));
        }
    }
    funcs
}

fn inserter_func(sk_class: &skc_hir::SkClass) -> mir::Function {
    let mut body_stmts = vec![];
    for mod_name in sk_class.wtable.0.keys() {
        let len = sk_class.wtable.get_len(mod_name);
        body_stmts.push(mir::Expr::fun_call(
            mir::Expr::func_ref(
                FunctionName::mangled("shiika_insert_wtable"),
                mir::FunTy::sync(
                    vec![mir::Ty::Ptr, mir::Ty::Int64, mir::Ty::Ptr, mir::Ty::Int64],
                    mir::Ty::Ptr,
                ),
            ),
            vec![
                mir::Expr::const_ref(sk_class.fullname().to_const_fullname(), mir::Ty::Ptr),
                mir::Expr::wtable_key(mod_name.clone()),
                mir::Expr::wtable_row(sk_class.fullname(), mod_name.clone()),
                mir::Expr::raw_i64(len as i64),
            ],
        ));
    }
    body_stmts.push(mir::Expr::return_cvoid());
    let func_name = insert_wtable_func_name(&sk_class.fullname());
    mir::Function {
        asyncness: mir::Asyncness::Sync,
        name: FunctionName::mangled(func_name),
        params: vec![],
        ret_ty: mir::Ty::CVoid,
        body_stmts: mir::Expr::exprs(body_stmts),
        sig: None,
        lvar_count: None,
    }
}

/// Returns mir::Expr that calls shiika_insert_all_wtables()
pub fn call_main_inserter() -> mir::TypedExpr {
    mir::Expr::fun_call(
        mir::Expr::func_ref(
            main_inserter_name(),
            mir::FunTy::sync(vec![], mir::Ty::CVoid),
        ),
        vec![],
    )
}

fn main_inserter(sk_types: &SkTypes, imported_sk_types: &SkTypes) -> mir::Function {
    let mut body_stmts = vec![];
    for sk_class in sk_types.sk_classes().chain(imported_sk_types.sk_classes()) {
        if !sk_class.wtable.is_empty() {
            body_stmts.push(mir::Expr::fun_call(
                mir::Expr::func_ref(
                    FunctionName::mangled(insert_wtable_func_name(&sk_class.fullname())),
                    mir::FunTy::sync(vec![], mir::Ty::CVoid),
                ),
                vec![],
            ));
        }
    }
    body_stmts.push(mir::Expr::return_cvoid());
    mir::Function {
        asyncness: mir::Asyncness::Sync,
        name: main_inserter_name(),
        params: vec![],
        ret_ty: mir::Ty::CVoid,
        body_stmts: mir::Expr::exprs(body_stmts),
        sig: None,
        lvar_count: None,
    }
}

fn main_inserter_name() -> FunctionName {
    FunctionName::mangled("shiika_insert_all_wtables")
}

fn insert_wtable_func_name(cls: &ClassFullname) -> String {
    format!("shiika_insert_{}_wtables", cls)
}
