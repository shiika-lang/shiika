use crate::names::*;
use crate::ty;
use crate::hir::*;
//use crate::stdlib::create_method;

pub fn create_class() -> Vec<SkClass> {
    vec![
        SkClass {
            fullname: ClassFullname("Void".to_string()),
            instance_ty: ty::raw("Void"),
            methods: create_methods(),
        },
        SkClass {
            fullname: ClassFullname("Meta:Void".to_string()),
            instance_ty: ty::meta("Void"),
            methods: vec![],
        },
    ]
}

fn create_methods() -> Vec<SkMethod> {
    vec![]
}

