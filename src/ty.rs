use crate::names::*;

// Types for a term (types of Shiika values)
#[derive(Debug, PartialEq, Clone)]
pub struct TermTy {
    pub fullname: ClassFullname,
    pub body: TyBody
}

#[derive(Debug, PartialEq, Clone)]
pub enum TyBody {
    // Types corresponds to non-generic class 
    // eg. "Int", "String", "Object"
    TyRaw,
    // Types corresponds to (non-generic) metaclass
    // eg. "Meta:Int", "Meta:String", "Meta:Object"
    TyMeta { base_fullname: String },
}

use TyBody::*;

impl TermTy {
    // Returns true when this is the Void type
    pub fn is_void_type(&self) -> bool {
        match self.body {
            TyRaw => (self.fullname.0 == "Void"),
            _ => false
        }
    }

    pub fn conforms_to(&self, other: &TermTy) -> bool {
        match self.body {
            TyRaw => {
                match other.body {
                    TyRaw => (self.fullname == other.fullname),
                    TyMeta { .. } => false,
                }
            },
            TyMeta { .. } => {
                match other.body  {
                    TyRaw => false,
                    TyMeta { .. } => (self.fullname == other.fullname),
                }
            },
        }
    }
}

pub fn raw(fullname: &str) -> TermTy {
    TermTy { fullname: ClassFullname(fullname.to_string()), body: TyRaw }
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
    pub name: MethodName,
    pub fullname: MethodFullname,
    pub ret_ty: TermTy,
    pub params: Vec<MethodParam>,
}

impl MethodSignature {
    /// Return a param of the given name and its index
    pub fn find_param(&self, name: &str) -> Option<(&MethodParam, u32)> {
        for i in 0..self.params.len() {
            let param = &self.params[i];
            if param.name == name {
                return Some((param, i as u32))
            }
        }
        None
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct MethodParam {
    pub name: String,
    pub ty: TermTy,
}
