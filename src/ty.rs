// Types for a term (types of Shiika values)
#[derive(Debug, PartialEq, Clone)]
pub enum TermTy { // TODO: Change this to a struct which have `fullname'
    // Types corresponds to non-generic class 
    // eg. "Int", "String", "Object"
    TyRaw { fullname: String },
    // Types corresponds to (non-generic) metaclass
    // eg. "Meta:Int", "Meta:String", "Meta:Object"
    TyMeta { fullname: String, base_fullname: String },
}
impl TermTy {
    pub fn class_fullname(&self) -> &str {
        match self {
            TermTy::TyRaw { fullname } => &fullname,
            TermTy::TyMeta { fullname, .. } => &fullname,
        }
    }

    // Returns true when this is the Void type
    pub fn is_void_type(&self) -> bool {
        match self {
            TermTy::TyRaw { fullname } => (fullname == "Void"),
            _ => false
        }
    }

    pub fn conforms_to(&self, other: &TermTy) -> bool {
        match self {
            TermTy::TyRaw { fullname: name1 } => {
                match other {
                    TermTy::TyRaw { fullname: name2 } => (*name1 == *name2),
                    TermTy::TyMeta { .. } => false,
                }
            },

            TermTy::TyMeta { fullname: name1, .. } => {
                match other {
                    TermTy::TyRaw { .. } => false,
                    TermTy::TyMeta { fullname: name2, .. } => (*name1 == *name2),
                }
            },
        }
    }
}

pub fn raw(fullname: &str) -> TermTy {
    TermTy::TyRaw { fullname: fullname.to_string() }
}

//impl TermTy for TyRaw {
//    fn fullname(&self) -> &str {
//        &self.fullname
//    }
//}
//
//pub struct TyMeta {
//    pub base_fullname: String,  // eg. "Int", "String", "Object"
//    pub fullname: String,
//}
//impl TyMeta {
//    pub fn new(base_fullname: &str) -> TyMeta {
//        TyMeta {
//            base_fullname: base_fullname.to_owned(),
//            fullname: "Meta:".to_owned() + base_fullname,
//        }
//    }
//}
//impl TermTy for TyMeta {
//    fn fullname(&self) -> &str {
//        &self.fullname
//    }
//}

// Types corresponds to (non-specialized) generic class
//pub struct TyGen {}
// Note: TyGen does not implement TermTy (Generic class itself cannot have an instance)

// Types corresponds to specialized generic class
//pub struct TySpe {}
//impl TermTy for TySpe {}
// Types corresponds to specialized generic metaclass
//pub struct TySpeMeta {}
//impl TermTy for TySpeMeta {}

#[derive(Debug, PartialEq, Clone)]
pub struct MethodSignature {
    pub name: String,
    pub fullname: String,
    pub ret_ty: TermTy,
    pub arg_tys: Vec<TermTy>, // TODO: Rename to 'param_tys'
}
