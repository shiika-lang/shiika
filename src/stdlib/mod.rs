mod bool;
mod float;
mod int;
mod math;
mod object;
mod void;
mod never;
mod string;
mod shiika_internal_memory;
use std::collections::HashMap;
use std::rc::Rc;
use crate::names::*;
use crate::ty;
use crate::hir::*;

pub struct Stdlib {
    pub sk_classes: HashMap<ClassFullname, SkClass>,
    pub sk_methods: HashMap<ClassFullname, Vec<SkMethod>>,
}

impl Stdlib {
    /// Create empty Stdlib (for tests)
    pub fn empty() -> Stdlib {
        Stdlib { 
            sk_classes: HashMap::new(),
            sk_methods: HashMap::new(),
        }
    }

    pub fn create() -> Stdlib {
        let items = rust_body_items();
        let (sk_classes, sk_methods) = make_classes(items);
        Stdlib { sk_classes, sk_methods }
    }
}

fn rust_body_items() -> Vec<(&'static str, Vec<SkMethod>, Vec<SkMethod>, HashMap<String, SkIVar>)> {
    vec![
        // Classes
        ("Bool"  , bool::create_methods()  , vec![]                      , HashMap::new()),
        ("Float" , float::create_methods() , vec![]                      , HashMap::new()),
        ("Int"   , int::create_methods()   , vec![]                      , HashMap::new()),
        ("Object", object::create_methods(), vec![]                      , HashMap::new()),
        ("Void"  , void::create_methods()  , vec![]                      , HashMap::new()),
        ("Never" , never::create_methods() , vec![]                      , HashMap::new()),
        ("String", string::create_methods(), vec![]                      , string::ivars()),
        ("Class" , vec![],                   vec![]                      , HashMap::new()),
        //("Shiika::Internal::Ptr", vec![], vec![], HashMap::new()),
        // Modules
        ("Math"  , vec![]                  , math::create_class_methods(), HashMap::new()),
        ("Shiika", vec![], vec![], HashMap::new()),
        ("Shiika::Internal", vec![], vec![], HashMap::new()),
        ("Shiika::Internal::Memory", vec![], shiika_internal_memory::create_class_methods(), HashMap::new()),
    ]
}

fn make_classes(items: Vec<(&'static str, Vec<SkMethod>, Vec<SkMethod>, HashMap<String, SkIVar>)>)
               -> (HashMap<ClassFullname, SkClass>, HashMap<ClassFullname, Vec<SkMethod>>) {
    let mut sk_classes = HashMap::new();
    let mut sk_methods = HashMap::new();
    for t in items.into_iter() {
        let (name, imethods, cmethods, ivars) = t;
        let super_name = if name == "Object" { None }
                         else { Some(ClassFullname("Object".to_string())) };
        sk_classes.insert(
            ClassFullname(name.to_string()),
            SkClass {
                fullname: ClassFullname(name.to_string()),
                superclass_fullname: super_name,
                instance_ty: ty::raw(name),
                ivars: Rc::new(ivars),
                method_sigs: imethods.iter().map(|x|
                    (x.signature.first_name().clone(), x.signature.clone())
                ).collect(),
            }
        );

        sk_classes.insert(
            ClassFullname("Meta:".to_string() + name),
            SkClass {
                fullname: ClassFullname("Meta:".to_string() + name),
                superclass_fullname: Some(ClassFullname("Class".to_string())),
                instance_ty: ty::meta(name),
                ivars: Rc::new(HashMap::new()),
                method_sigs: cmethods.iter().map(|x|
                    (x.signature.first_name().clone(), x.signature.clone())
                ).collect(),
            }
        );
        sk_methods.insert(
            ClassFullname(name.to_string()),
            imethods.into_iter().chain(cmethods).collect()
        );
    };
    (sk_classes, sk_methods)
}

//fn shiika_body_items() -> Vec<> {
//}

fn create_method(class_name: &str,
                      sig_str: &str,
                      gen: GenMethodBody) -> SkMethod {
    let mut parser = crate::parser::Parser::new(sig_str);
    let (ast_sig, _) = parser.parse_method_signature().unwrap();
    let sig = crate::hir::create_signature(class_name.to_string(), &ast_sig);

    SkMethod {
        signature: sig,
        body: SkMethodBody::RustMethodBody{ gen: gen }
    }
}
