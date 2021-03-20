use crate::ast;
use crate::names::*;
use crate::ty;
use crate::ty::*;

#[derive(Debug, PartialEq, Clone)]
pub struct MethodSignature {
    pub fullname: MethodFullname,
    pub ret_ty: TermTy,
    pub params: Vec<MethodParam>,
    pub typarams: Vec<String>,
}

impl MethodSignature {
    pub fn first_name(&self) -> &MethodFirstname {
        &self.fullname.first_name
    }

    /// Substitute type parameters with type arguments
    pub fn specialize(
        &self,
        class_tyargs: Option<&[TermTy]>,
        method_tyargs: Option<&[TermTy]>,
    ) -> MethodSignature {
        MethodSignature {
            fullname: self.fullname.clone(),
            ret_ty: self.ret_ty.substitute(class_tyargs, method_tyargs),
            params: self
                .params
                .iter()
                .map(|param| param.substitute(class_tyargs, method_tyargs))
                .collect(),
            typarams: self.typarams.clone(), // eg. Array<T>#map<U>(f: Fn1<T, U>) -> Array<Int>#map<U>(f: Fn1<Int, U>)
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct MethodParam {
    pub name: String,
    pub ty: TermTy,
}

impl MethodParam {
    pub fn substitute(
        &self,
        class_tyargs: Option<&[TermTy]>,
        method_tyargs: Option<&[TermTy]>,
    ) -> MethodParam {
        MethodParam {
            name: self.name.clone(),
            ty: self.ty.substitute(class_tyargs, method_tyargs),
        }
    }
}

/// Return a param of the given name and its index
pub fn find_param<'a>(params: &'a [MethodParam], name: &str) -> Option<(usize, &'a MethodParam)> {
    params
        .iter()
        .enumerate()
        .find(|(_, param)| param.name == name)
}

/// Create `hir::MethodSignature` from `ast::MethodSignature`
pub fn create_signature(
    class_fullname: &ClassFullname,
    sig: &ast::AstMethodSignature,
    class_typarams: &[String],
) -> MethodSignature {
    let fullname = method_fullname(class_fullname, &sig.name.0);
    let ret_ty = convert_typ(&sig.ret_typ, class_typarams, &sig.typarams);
    let params = convert_params(&sig.params, class_typarams, &sig.typarams);
    MethodSignature {
        fullname,
        ret_ty,
        params,
        typarams: sig.typarams.clone(),
    }
}

// TODO: pass the list of visible classes
fn convert_typ(typ: &ast::Typ, class_typarams: &[String], method_typarams: &[String]) -> TermTy {
    if let Some(idx) = class_typarams.iter().position(|s| *s == typ.name) {
        ty::typaram(&typ.name, ty::TyParamKind::Class, idx)
    } else if let Some(idx) = method_typarams.iter().position(|s| *s == typ.name) {
        ty::typaram(&typ.name, ty::TyParamKind::Method, idx)
    } else if typ.typ_args.is_empty() {
        ty::raw(&typ.name)
    } else {
        let tyargs = typ
            .typ_args
            .iter()
            .map(|t| convert_typ(t, class_typarams, method_typarams))
            .collect();
        ty::spe(&typ.name, tyargs)
    }
}

pub fn convert_params(
    params: &[ast::Param],
    class_typarams: &[String],
    method_typarams: &[String],
) -> Vec<MethodParam> {
    params
        .iter()
        .map(|param| MethodParam {
            name: param.name.to_string(),
            ty: convert_typ(&param.typ, class_typarams, method_typarams),
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
        typarams: vec![],
    }
}
