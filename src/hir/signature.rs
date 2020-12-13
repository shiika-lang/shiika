use crate::ast;
use crate::names::*;
use crate::ty;
use crate::ty::*;

/// Create `hir::MethodSignature` from `ast::MethodSignature`
pub fn create_signature(
    class_fullname: &ClassFullname,
    sig: &ast::AstMethodSignature,
    typarams: &[String],
) -> MethodSignature {
    let fullname = method_fullname(class_fullname, &sig.name.0);
    let ret_ty = convert_typ(&sig.ret_typ, typarams);
    let params = convert_params(&sig.params, typarams);
    MethodSignature {
        fullname,
        ret_ty,
        params,
    }
}

// TODO: pass the list of visible classes
fn convert_typ(typ: &ast::Typ, typarams: &[String]) -> TermTy {
    let found = typarams.iter().enumerate().find(|(_, s)| **s == typ.name);
    if let Some((idx, _)) = found {
        ty::typaram(&typ.name, idx)
    } else if typ.typ_args.is_empty() {
        ty::raw(&typ.name)
    } else {
        let tyargs = typ
            .typ_args
            .iter()
            .map(|t| convert_typ(t, typarams))
            .collect();
        ty::spe(&typ.name, tyargs)
    }
}

pub fn convert_params(params: &[ast::Param], typarams: &[String]) -> Vec<MethodParam> {
    params
        .iter()
        .map(|param| MethodParam {
            name: param.name.to_string(),
            ty: convert_typ(&param.typ, typarams),
        })
        .collect()
}

/// Create a signature of a `new` method
pub fn signature_of_new(
    metaclass_fullname: &ClassFullname,
    initialize_params: Vec<MethodParam>,
    instance_ty: &TermTy,
) -> MethodSignature {
    MethodSignature {
        fullname: method_fullname(metaclass_fullname, "new"),
        ret_ty: instance_ty.clone(),
        params: initialize_params,
    }
}
