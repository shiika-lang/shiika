use crate::ast;
use crate::ty;
use crate::ty::*;
use crate::names::*;

/// Create `hir::MethodSignature` from `ast::MethodSignature`
pub fn create_signature(
    class_fullname: &ClassFullname,
    sig: &ast::AstMethodSignature,
    typarams: &[String],
) -> MethodSignature {
    let fullname = MethodFullname {
        full_name: (class_fullname.0.to_string() + "#" + &sig.name.0),
        first_name: sig.name.clone(),
    };
    let ret_ty = convert_typ(&sig.ret_typ, typarams);
    let params = convert_params(&sig.params, typarams);
    MethodSignature { fullname, ret_ty, params }
}

// TODO: pass the list of visible classes
fn convert_typ(typ: &ast::Typ, typarams: &[String]) -> TermTy {
    if typarams.contains(&typ.name) {
        ty::typaram(&typ.name)
    }
    else {
        ty::raw(&typ.name)
    }
}

pub fn convert_params(params: &[ast::Param], typarams: &[String]) -> Vec<MethodParam> {
    params.iter().map(|param|
        MethodParam {
            name: param.name.to_string(),
            ty: convert_typ(&param.typ, typarams),
        }
    ).collect()
}

pub fn signature_of_new(metaclass_fullname: &ClassFullname,
                    initialize_params: Vec<MethodParam>,
                    instance_ty: &TermTy) -> MethodSignature {
    MethodSignature {
        fullname: method_fullname(metaclass_fullname, "new"),
        ret_ty: instance_ty.clone(),
        params: initialize_params,
    }
}
