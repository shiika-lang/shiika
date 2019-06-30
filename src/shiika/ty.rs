// Types for a term (types of Shiika values)
#[derive(Debug, PartialEq, Clone)]
pub enum TermTy {
    // Types corresponds to non-generic class 
    // eg. "Int", "String", "Object"
    TyRaw { fullname: String },
    // Types corresponds to (non-generic) metaclass
    // eg. "Meta:Int", "Meta:String", "Meta:Object"
    TyMeta { base_fullname: String },
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
