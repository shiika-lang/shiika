use crate::hir::class_dict::ClassDict;
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
    pub body: TyBody,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TyBody {
    // Types corresponds to non-generic class
    // eg. "Int", "String", "Object"
    TyRaw,
    // Types corresponds to (non-generic) metaclass
    // eg. "Meta:Int", "Meta:String", "Meta:Object"
    TyMeta {
        base_fullname: String,
    },
    // This object belongs to the class `Class` (i.e. this is a class object)
    TyClass,
    // Types for generic metaclass eg. `Meta:Pair<S, T>`
    TyGenMeta {
        base_name: String,          // eg. "Pair"
        typaram_names: Vec<String>, // eg. ["S", "T"] (For debug print)
    },
    // Types for specialized class eg. `Pair<Int, Bool>`
    TySpe {
        base_name: String, // eg. "Pair"
        type_args: Vec<TermTy>,
    },
    // Types for specialized metaclass eg. `Meta:Pair<Int, Bool>`
    TySpeMeta {
        base_name: String, // eg. "Pair"
        type_args: Vec<TermTy>,
    },
    // Type parameter reference eg. `T`
    TyParamRef {
        name: String,
        idx: usize,
    },
}

use TyBody::*;

impl TermTy {
    // Returns true when this is the Void type
    pub fn is_void_type(&self) -> bool {
        match self.body {
            TyRaw => (self.fullname.0 == "Void"),
            _ => false,
        }
    }

    pub fn meta_ty(&self) -> TermTy {
        match self.body {
            TyRaw => ty::meta(&self.fullname.0),
            TyMeta { .. } => ty::class(),
            TyClass => ty::class(),
            _ => panic!("TODO"),
        }
    }

    pub fn conforms_to(&self, other: &TermTy) -> bool {
        if let TyParamRef { .. } = other.body {
            return self == &ty::raw("Object"); // The upper bound
        }
        // TODO: Should respect class hierarchy
        self.equals_to(other)
    }

    /// Return true if two types are identical
    pub fn equals_to(&self, other: &TermTy) -> bool {
        self == other
    }

    /// Return the supertype of self
    pub fn supertype(&self, class_dict: &ClassDict) -> Option<TermTy> {
        match &self.body {
            TyRaw => class_dict
                .get_superclass(&self.fullname)
                .map(|scls| ty::raw(&scls.fullname.0)),
            TyMeta { base_fullname } => {
                match class_dict.get_superclass(&class_fullname(base_fullname)) {
                    Some(scls) => Some(ty::meta(&scls.fullname.0)),
                    None => Some(ty::class()), // Meta:Object < Class
                }
            }
            TyClass => Some(ty::raw("Object")),
            _ => panic!("TODO"),
        }
    }

    /// Apply type argments into type parameters
    pub fn substitute(&self, type_args: &[TermTy]) -> TermTy {
        match &self.body {
            TyParamRef { idx, .. } => type_args[*idx].clone(),
            _ => self.clone(),
        }
    }

    pub fn is_specialized(&self) -> bool {
        match self.body {
            TySpe { .. } | TySpeMeta { .. } => true,
            _ => false,
        }
    }
}

pub fn raw(fullname: &str) -> TermTy {
    TermTy {
        fullname: class_fullname(fullname),
        body: TyRaw,
    }
}

pub fn meta(base_fullname: &str) -> TermTy {
    TermTy {
        fullname: metaclass_fullname(base_fullname),
        body: TyMeta {
            base_fullname: base_fullname.to_string(),
        },
    }
}

pub fn class() -> TermTy {
    TermTy {
        fullname: class_fullname("Class"),
        body: TyClass,
    }
}

pub fn spe(base_name: &str, type_args: Vec<TermTy>) -> TermTy {
    let tyarg_names = type_args
        .iter()
        .map(|x| x.fullname.0.to_string())
        .collect::<Vec<_>>();
    TermTy {
        fullname: class_fullname(&format!("{}<{}>", &base_name, &tyarg_names.join(","))),
        body: TySpe {
            base_name: base_name.to_string(),
            type_args,
        },
    }
}

pub fn typaram(name: impl Into<String>, idx: usize) -> TermTy {
    let s = name.into();
    TermTy {
        fullname: class_fullname(&s),
        body: TyParamRef { name: s, idx },
    }
}

/// A type parameter
/// In the future, may have something like +T/-T or in/out
#[derive(Debug, PartialEq, Clone)]
pub struct TyParam {
    pub name: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct MethodSignature {
    pub fullname: MethodFullname,
    pub ret_ty: TermTy,
    pub params: Vec<MethodParam>,
}

impl MethodSignature {
    /// Return a param of the given name and its index
    pub fn find_param(&self, name: &str) -> Option<(usize, &MethodParam)> {
        self.params
            .iter()
            .enumerate()
            .find(|(_, param)| param.name == name)
    }

    pub fn first_name(&self) -> &MethodFirstname {
        &self.fullname.first_name
    }

    /// Substitute type parameters with type arguments
    pub fn specialize(&self, type_args: &[TermTy]) -> MethodSignature {
        MethodSignature {
            fullname: self.fullname.clone(),
            ret_ty: self.ret_ty.substitute(&type_args),
            params: self
                .params
                .iter()
                .map(|param| param.substitute(&type_args))
                .collect(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct MethodParam {
    pub name: String,
    pub ty: TermTy,
}

impl MethodParam {
    pub fn substitute(&self, type_args: &[TermTy]) -> MethodParam {
        MethodParam {
            name: self.name.clone(),
            ty: self.ty.substitute(&type_args),
        }
    }
}
