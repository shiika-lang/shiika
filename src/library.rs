use crate::hir::*;
use crate::names::*;
use crate::ty::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct ImportedItems {
    pub sk_classes: HashMap<ClassFullname, SkClass>,
    pub sk_methods: HashMap<ClassFullname, Vec<SkMethod>>,
    pub constants: HashMap<ConstFullname, TermTy>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LibraryExports {
    classes: Vec<ClassInfo>,
    constants: HashMap<ConstFullname, TermTy>,
}

impl LibraryExports {
    pub fn new(hir: &Hir) -> LibraryExports {
        let classes = hir.sk_classes.values().map(ClassInfo::new).collect();
        LibraryExports {
            classes,
            constants: hir.constants.clone(),
        }
    }

    pub fn into_imported_items(self) -> ImportedItems {
        let mut sk_classes = HashMap::new();
        for cls_info in self.classes {
            let class = cls_info.extract();
            sk_classes.insert(class.fullname.clone(), class);
        }
        ImportedItems {
            sk_classes,
            sk_methods: Default::default(),
            constants: self.constants,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ClassInfo {
    name: ClassFullname,
    typarams: Vec<TyParam>,
    superclass_fullname: Option<ClassFullname>,
    instance_ty: TermTy,
    method_sigs: HashMap<MethodFirstname, MethodSignature>,
    const_is_obj: bool,
}

impl ClassInfo {
    fn new(sk_class: &SkClass) -> ClassInfo {
        ClassInfo {
            name: sk_class.fullname.clone(),
            typarams: sk_class.typarams.clone(),
            superclass_fullname: sk_class.superclass_fullname.clone(),
            instance_ty: sk_class.instance_ty.clone(),
            method_sigs: sk_class.method_sigs.clone(),
            const_is_obj: sk_class.const_is_obj,
        }
    }

    fn extract(self) -> SkClass {
        SkClass {
            fullname: self.name,
            typarams: self.typarams,
            superclass_fullname: self.superclass_fullname,
            instance_ty: self.instance_ty,
            ivars: Default::default(),
            method_sigs: self.method_sigs,
            const_is_obj: self.const_is_obj,
            foreign: true,
        }
    }
}
