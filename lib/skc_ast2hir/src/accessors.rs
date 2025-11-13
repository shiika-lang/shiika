use crate::hir_maker::HirMaker;
use shiika_core::names::*;
use skc_hir::*;

impl<'hir_maker> HirMaker<'hir_maker> {
    /// Define getters and setters (unless there is a method of the same name)
    pub(super) fn define_accessors(
        &mut self,
        clsname: &ClassFullname,
        ivars: SkIVars,
        defs: &[shiika_ast::Definition],
    ) {
        let method_names = defs
            .iter()
            .filter_map(|def| {
                if let shiika_ast::Definition::InstanceMethodDefinition { sig, .. } = def {
                    Some(&sig.name.0)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for ivar in ivars.values() {
            let accessor_name = ivar.accessor_name();
            if !method_names.iter().any(|x| ***x == accessor_name) {
                let getter = create_getter(clsname, ivar);
                let getter_sig = create_getter_signature(clsname, ivar);
                self.method_dict
                    .add_method(clsname.to_type_fullname(), getter);
                self.class_dict.add_method(getter_sig);
            }

            let setter_name = format!("{}=", accessor_name);
            if !method_names.iter().any(|x| ***x == setter_name) {
                let setter = create_setter(clsname, ivar);
                let setter_sig = create_setter_signature(clsname, ivar);
                self.method_dict
                    .add_method(clsname.to_type_fullname(), setter);
                self.class_dict.add_method(setter_sig);
            }
        }
    }
}

fn create_getter_signature(clsname: &ClassFullname, ivar: &SkIVar) -> MethodSignature {
    MethodSignature {
        fullname: method_fullname(clsname.to_type_fullname(), &ivar.accessor_name()),
        ret_ty: ivar.ty.clone(),
        params: vec![],
        typarams: vec![],
        asyncness: Asyncness::Sync,
        is_virtual: false,
        is_rust: false,
    }
}

fn create_getter(clsname: &ClassFullname, ivar: &SkIVar) -> SkMethod {
    let fullname = method_fullname(clsname.to_type_fullname(), &ivar.accessor_name());
    SkMethod::simple(
        fullname,
        SkMethodBody::Getter {
            idx: ivar.idx,
            name: ivar.name.clone(),
            ty: ivar.ty.clone(),
            self_ty: clsname.to_ty(),
        },
    )
}

fn create_setter_signature(clsname: &ClassFullname, ivar: &SkIVar) -> MethodSignature {
    let accessor_name = ivar.accessor_name();
    let setter_name = format!("{}=", accessor_name);
    MethodSignature {
        fullname: method_fullname(clsname.to_type_fullname(), &setter_name),
        ret_ty: ivar.ty.clone(),
        params: vec![MethodParam {
            name: ivar.accessor_name(),
            ty: ivar.ty.clone(),
            has_default: false,
        }],
        typarams: vec![],
        asyncness: Asyncness::Sync,
        is_virtual: false,
        is_rust: false,
    }
}

fn create_setter(clsname: &ClassFullname, ivar: &SkIVar) -> SkMethod {
    let accessor_name = ivar.accessor_name();
    let setter_name = format!("{}=", accessor_name);
    let fullname = method_fullname(clsname.to_type_fullname(), &setter_name);
    SkMethod::simple(
        fullname,
        SkMethodBody::Setter {
            idx: ivar.idx,
            name: ivar.name.clone(),
            ty: ivar.ty.clone(),
            self_ty: clsname.to_ty(),
        },
    )
}
