use crate::hir_maker::HirMaker;
use shiika_core::names::*;
use skc_hir::*;

impl<'hir_maker> HirMaker<'hir_maker> {
    /// Define getters and setters (unless there is a method of the same name)
    pub(super) fn define_accessors(
        &mut self,
        clsname: &ModuleFullname,
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
                let sig = getter.signature.clone();
                self.method_dict.add_method(clsname, getter);
                self.class_dict.add_method(clsname, sig);
            }

            let setter_name = format!("{}=", accessor_name);
            if !method_names.iter().any(|x| ***x == setter_name) {
                let setter = create_setter(clsname, ivar);
                let sig = setter.signature.clone();
                self.method_dict.add_method(clsname, setter);
                self.class_dict.add_method(clsname, sig);
            }
        }
    }
}

fn create_getter(clsname: &ModuleFullname, ivar: &SkIVar) -> SkMethod {
    let sig = MethodSignature {
        fullname: method_fullname(clsname, &ivar.accessor_name()),
        ret_ty: ivar.ty.clone(),
        params: vec![],
        typarams: vec![],
    };
    SkMethod {
        signature: sig,
        body: SkMethodBody::Getter {
            idx: ivar.idx,
            name: ivar.name.clone(),
        },
        lvars: vec![],
    }
}

fn create_setter(clsname: &ModuleFullname, ivar: &SkIVar) -> SkMethod {
    let accessor_name = ivar.accessor_name();
    let setter_name = format!("{}=", accessor_name);
    let sig = MethodSignature {
        fullname: method_fullname(clsname, &setter_name),
        ret_ty: ivar.ty.clone(),
        params: vec![MethodParam {
            name: ivar.accessor_name(),
            ty: ivar.ty.clone(),
        }],
        typarams: vec![],
    };
    SkMethod {
        signature: sig,
        body: SkMethodBody::Setter {
            idx: ivar.idx,
            name: ivar.name.clone(),
        },
        lvars: vec![],
    }
}
