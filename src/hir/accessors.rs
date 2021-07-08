use crate::code_gen::CodeGen;
use crate::hir::hir_maker::HirMaker;
use crate::hir::*;

impl<'hir_maker> HirMaker<'hir_maker> {
    /// Define getters and setters (unless there is a method of the same name)
    pub(super) fn define_accessors(
        &mut self,
        clsname: &ClassFullname,
        ivars: SkIVars,
        defs: &[ast::Definition],
    ) {
        let method_names = defs
            .iter()
            .filter_map(|def| {
                if let ast::Definition::InstanceMethodDefinition { sig, .. } = def {
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

fn create_getter(clsname: &ClassFullname, ivar: &SkIVar) -> SkMethod {
    let sig = MethodSignature {
        fullname: method_fullname(clsname, &ivar.accessor_name()),
        ret_ty: ivar.ty.clone(),
        params: vec![],
        typarams: vec![],
    };
    let name = ivar.name.clone(); // Clone to embed into the closure
    let idx = ivar.idx;
    let getter_body = move |code_gen: &CodeGen, function: &inkwell::values::FunctionValue| {
        let this = function.get_params()[0];
        let val = code_gen.build_ivar_load(this, idx, &name);
        code_gen.builder.build_return(Some(&val));
        Ok(())
    };

    SkMethod {
        signature: sig,
        body: SkMethodBody::RustClosureMethodBody {
            boxed_gen: Box::new(getter_body),
        },
        lvars: vec![],
    }
}

fn create_setter(clsname: &ClassFullname, ivar: &SkIVar) -> SkMethod {
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
    let ivar_name = ivar.name.clone(); // Clone to embed into the closure
    let idx = ivar.idx;
    let setter_body = move |code_gen: &CodeGen, function: &inkwell::values::FunctionValue| {
        let this = function.get_params()[0];
        let val = function.get_params()[1];
        code_gen.build_ivar_store(&this, idx, val, &ivar_name);
        code_gen.builder.build_return(Some(&val));
        Ok(())
    };

    SkMethod {
        signature: sig,
        body: SkMethodBody::RustClosureMethodBody {
            boxed_gen: Box::new(setter_body),
        },
        lvars: vec![],
    }
}
