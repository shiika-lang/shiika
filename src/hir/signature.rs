use crate::ast;
use crate::ty;
use crate::ty::*;
use crate::names::*;

/// Create `hir::MethodSignature` from `ast::MethodSignature`
pub fn create_signature(class_fullname: String, sig: &ast::AstMethodSignature) -> MethodSignature {
    let fullname = MethodFullname {
        full_name: (class_fullname + "#" + &sig.name.0),
        first_name: sig.name.clone(),
    };
    let ret_ty = convert_typ(&sig.ret_typ);
    let params = convert_params(&sig.params);
    MethodSignature { fullname, ret_ty, params }
}

// TODO: pass the list of current typarams
// TODO: pass the list of visible classes
fn convert_typ(typ: &ast::Typ) -> TermTy {
    ty::raw(&typ.name)
}

pub fn convert_params(params: &[ast::Param]) -> Vec<MethodParam> {
    params.iter().map(|param|
        MethodParam {
            name: param.name.to_string(),
            ty: convert_typ(&param.typ),
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
