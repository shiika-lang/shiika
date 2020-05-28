use crate::code_gen::CodeGen;
use crate::hir::*;
use crate::hir::hir_maker::HirMaker;

impl HirMaker {
    /// Define getters and setters (unless there is a method of the same name)
    pub (in super) fn define_accessors(&mut self,
                        clsname: &ClassFullname,
                        ivars: SkIVars,
                        defs: &[ast::Definition]) {
        let method_names = defs.iter().filter_map(|def| {
            if let ast::Definition::InstanceMethodDefinition { sig, .. } = def {
                Some(&sig.name.0)
            }
            else {
                None
            }
        }).collect::<Vec<_>>();
        for (name, ivar) in ivars {
            // Already has it
            if method_names.iter().find(|x| ***x == name).is_some() { continue; }
            // Getter
            let getter = create_getter(&clsname, &method_firstname(&name), &ivar);
            let sig = getter.signature.clone();
            self.method_dict.add_method(&clsname, getter);
            self.class_dict.add_method(&clsname, sig);
            // TODO: Setter
        }
    }
}

fn create_getter(clsname: &ClassFullname,
                 method_name: &MethodFirstname,
                 ivar: &SkIVar) -> SkMethod {
    let sig = MethodSignature {
        fullname: method_fullname(clsname, &method_name.0),
        ret_ty: ivar.ty.clone(),
        params: vec![],
    };
    let name = method_name.0.clone();
    let idx = ivar.idx;
    let getter_body = move |code_gen: &CodeGen, function: &inkwell::values::FunctionValue| {
        let this = function.get_params()[0];
        let ptr = code_gen.builder.build_struct_gep(this.into_pointer_value(),
                                                    idx as u32,
                                                    &format!("addr_{}", name)).unwrap();
        let val = code_gen.builder.build_load(ptr, &name);
        code_gen.builder.build_return(Some(&val));
        Ok(())
    };

    SkMethod {
        signature: sig,
        body: SkMethodBody::RustClosureMethodBody {
            boxed_gen: Box::new(getter_body),
        }
    }
}