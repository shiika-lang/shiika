/// Shiika types
///
/// ```text
///   ^ : superclass-subclass relationship
///   ~ : class-instance relationship
///
///                        Object
///                           ^
///                        Class ~ Class
///                           ^
///               Object ~ Meta:Object
///                 ^         ^
///     [1,2,3] ~ Array  ~ Meta:Array ~ Class
/// ```
///
use crate::names::*;
use crate::ty;

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
    // This object belongs to the class `Class` (i.e. this is a class object)
    TyClass,
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

    pub fn is_nonmeta(&self) -> bool {
        match self.body {
            TyRaw => true,
            _ => false,
        }
    }

    pub fn meta_ty(&self) -> TermTy {
        match self.body {
            TyRaw => ty::meta(&self.fullname.0),
            TyMeta { .. } => ty::class(),
            TyClass => ty::class(),
        }
    }

    pub fn conforms_to(&self, other: &TermTy) -> bool {
        match self.body {
            TyRaw => {
                match other.body {
                    TyRaw => (self.fullname == other.fullname),
                    _ => false,
                }
            },
            TyMeta { .. } => {
                match other.body  {
                    TyMeta { .. } => (self.fullname == other.fullname),
                    _ => false,
                }
            },
            TyClass => {
                match other.body {
                    TyClass => true,
                    _ => false,
                }
            }
        }
    }

    pub fn equals_to(&self, other: &TermTy) -> bool {
        match self.body {
            TyRaw => {
                match other.body {
                    TyRaw => (self.fullname == other.fullname),
                    _ => false,
                }
            },
            TyMeta { .. } => {
                match other.body  {
                    TyMeta { .. } => (self.fullname == other.fullname),
                    _ => false,
                }
            },
            TyClass => {
                match other.body {
                    TyClass => true,
                    _ => false,
                }
            }
        }
    }
}

pub fn raw(fullname: &str) -> TermTy {
    TermTy { fullname: ClassFullname(fullname.to_string()), body: TyRaw }
}

pub fn meta(base_fullname: &str) -> TermTy {
    TermTy {
        fullname: ClassFullname("Meta:".to_string() + base_fullname),
        body: TyMeta { base_fullname: base_fullname.to_string() },
    }
}

pub fn class() -> TermTy {
    TermTy {
        fullname: ClassFullname("Class".to_string()),
        body: TyClass,
    }
}

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
    pub fullname: MethodFullname,
    pub ret_ty: TermTy,
    pub params: Vec<MethodParam>,
}

impl MethodSignature {
    /// Return a param of the given name and its index
    pub fn find_param(&self, name: &str) -> Option<(usize, &MethodParam)> {
        self.params.iter().enumerate().find(|(_, param)| param.name == name)
    }

    pub fn first_name(&self) -> &MethodFirstname {
        &self.fullname.first_name
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct MethodParam {
    pub name: String,
    pub ty: TermTy,
}
