use serde::{Deserialize, Serialize};
use crate::names::*;
use crate::ty;

// Types for a term (types of Shiika values)
#[derive(PartialEq, Clone, Serialize, Deserialize)]
pub struct TermTy {
    pub fullname: ClassFullname,
    pub body: TyBody,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum TyBody {
    /// Types of classes
    /// eg. "Int", "Meta:String", "Array<Int>", "Meta:Pair<Bool, Object>", etc.
    TyRaw(RawTy),
    /// Type parameter reference eg. `T`
    TyParamRef {
        kind: TyParamKind,
        name: String,
        idx: usize,
        upper_bound: Box<TermTy>,
        lower_bound: Box<TermTy>,
    },
}
use TyBody::*;

// REFACTOR: better name?
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct RawTy {
    // REFACTOR: ideally these should be private
    pub base_name: String,
    pub type_args: Vec<TermTy>,
    pub is_meta: bool,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum TyParamKind {
    /// eg. `class A<B>`
    Class,
    /// eg. `def foo<X>(...)`
    Method,
}

/// A type parameter
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct TyParam {
    pub name: String,
    pub variance: Variance,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Variance {
    Invariant,
    Covariant,     // eg. `in T`
    Contravariant, // eg. `out T`
}

impl TyParam {
    pub fn new(name: impl Into<String>) -> TyParam {
        TyParam {
            name: name.into(),
            variance: Variance::Invariant,
        }
    }
}

impl std::fmt::Display for TermTy {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.fullname)
    }
}

impl std::fmt::Debug for TermTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TermTy({})", &self.dbg_str())
    }
}

impl TermTy {
    /// Return string to inspect `self`
    fn dbg_str(&self) -> String {
        match &self.body {
            TyRaw(RawTy {
                base_name,
                type_args,
                is_meta,
            }) => {
                let meta = if *is_meta { "Meta:" } else { "" };
                format!("{}{}{}", meta, base_name, _dbg_type_args(type_args))
                // TODO: Use colors?
                // "\x1b[32m{}<\x1b[0m{}\x1b[32m>\x1b[0m"
            },
            TyParamRef {
                kind, name, idx, ..
            } => {
                let k = match kind {
                    TyParamKind::Class => "C",
                    TyParamKind::Method => "M",
                };
                format!("TyParamRef({} {}{})", name, idx, k)
            }
        }
    }

    /// Returns if value of this type is class
    pub fn is_metaclass(&self) -> bool {
        match &self.body {
            TyRaw(RawTy { base_name, is_meta, .. }) => *is_meta || base_name == "Metaclass",
            _ => false,
        }
    }

    /// Returns if this is TyParamRef
    pub fn is_typaram_ref(&self) -> bool {
        matches!(&self.body, TyParamRef { .. })
    }

    pub fn to_const_fullname(&self) -> ConstFullname {
        match &self.body {
            TyRaw(RawTy {
                base_name,
                type_args,
                is_meta,
            }) => {
                debug_assert!(is_meta);
                toplevel_const(&format!("{}{}", base_name, &tyargs_str(type_args)))
            }
            _ => panic!("[BUG] to_const_fullname called on {:?}", &self),
        }
    }

    // Returns true when this is the Void type
    pub fn is_void_type(&self) -> bool {
        match self.body {
            TyRaw(_) => (self.fullname.0 == "Void"),
            _ => false,
        }
    }

    // Returns true when this is the Never type
    pub fn is_never_type(&self) -> bool {
        match self.body {
            TyRaw(_) => (self.fullname.0 == "Never"),
            _ => false,
        }
    }

    // Returns ret_ty if this is any of Fn0, .., Fn9
    pub fn fn_x_info(&self) -> Option<TermTy> {
        match &self.body {
            TyRaw(RawTy {
                base_name,
                type_args,
                is_meta,
            }) => {
                if *is_meta {
                    return None;
                }
                for i in 0..=9 {
                    if *base_name == format!("Fn{}", i) {
                        let ret_ty = type_args.last().unwrap().clone();
                        return Some(ret_ty);
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub fn meta_ty(&self) -> TermTy {
        match &self.body {
            TyRaw(RawTy {
                base_name,
                type_args,
                ..
            }) => {
                if self.is_metaclass() {
                    ty::raw("Metaclass")
                } else {
                    ty::spe_meta(base_name, type_args.clone())
                }
            },
            _ => panic!("unexpected"),
        }
    }

    pub fn instance_ty(&self) -> TermTy {
        match &self.body {
            TyRaw(RawTy {
                base_name,
                type_args,
                is_meta,
            }) => {
                debug_assert!(is_meta);
                ty::spe(base_name, type_args.to_vec())
            }
            _ => panic!("instance_ty is undefined for {:?}", self),
        }
    }

    pub fn specialized_ty(&self, tyargs: Vec<TermTy>) -> TermTy {
        match &self.body {
            TyRaw(RawTy{ base_name, type_args, is_meta }) => {
                debug_assert!(type_args.len() == tyargs.len());
                ty::new(base_name, tyargs, *is_meta)
            },
            _ => panic!("unexpected"),
        }
    }

    /// Return "A" for "A<B>", "Meta:A" for "Meta:A<B>"
    pub fn base_class_name(&self) -> ClassFullname {
        match &self.body {
            TyRaw(RawTy { base_name, is_meta, .. } )=> {
                ClassFullname::new(base_name, *is_meta)
            }
            _ => panic!("unexpected"),
        }
    }

    /// Return true if two types are identical
    pub fn equals_to(&self, other: &TermTy) -> bool {
        self == other
    }

    /// Return true when two types are the same if type args are removed
    pub fn same_base(&self, other: &TermTy) -> bool {
        // PERF: building strings is not necesarry
        self.erasure() == other.erasure()
    }

    /// Return class name without type arguments
    /// eg.
    ///   Array<Int>      =>  Array
    ///   Pair<Int,Bool>  =>  Pair
    pub fn erasure(&self) -> ClassFullname {
        match &self.body {
            TyRaw(RawTy { base_name, is_meta, .. }) => {
                ClassFullname::new(base_name, *is_meta)
            }
            _ => todo!(),
        }
    }

    pub fn erasure_ty(&self) -> TermTy {
        ty::raw(self.erasure().0)
    }

    /// Returns type arguments, if any
    pub fn tyargs(&self) -> &[TermTy] {
        match &self.body {
            TyRaw(RawTy { type_args, .. }) => type_args,
            _ => &[],
        }
    }

    /// Apply type argments into type parameters
    /// - class_tyargs: None if the class is not generic (non-generic class
    ///   can have a generic method)
    /// - method_tyargs: None if not in a method context (eg. when creating
    ///   `Array<Int>` from `Array<T>`)
    pub fn substitute(&self, class_tyargs: &[TermTy], method_tyargs: &[TermTy]) -> TermTy {
        match &self.body {
            TyParamRef { kind, idx, .. } => match kind {
                TyParamKind::Class => {
                    if class_tyargs.is_empty() {
                        self.clone()
                    } else {
                        class_tyargs[*idx].clone()
                    }
                }
                TyParamKind::Method => {
                    if method_tyargs.is_empty() {
                        self.clone()
                    } else {
                        method_tyargs[*idx].clone()
                    }
                }
            },
            TyRaw(RawTy {
                base_name,
                type_args,
                is_meta,
            }) => {
                let args = type_args
                    .iter()
                    .map(|t| t.substitute(class_tyargs, method_tyargs))
                    .collect();
                ty::new(base_name, args, *is_meta)
            },
        }
    }

    /// Name for vtable when invoking a method on an object of this type
    pub fn vtable_name(&self) -> ClassFullname {
        match &self.body {
            TyRaw(RawTy { base_name, is_meta, .. }) => ClassFullname::new(base_name, *is_meta),
            _ => self.fullname.clone(),
        }
    }

    pub fn is_specialized(&self) -> bool {
        match &self.body {
            TyRaw(RawTy { type_args, .. }) => !type_args.is_empty(),
            _ => false,
        }
    }

    pub fn contains_typaram_ref(&self) -> bool {
        match &self.body {
            TyParamRef { .. } => true,
            TyRaw(RawTy { type_args, .. }) => type_args.iter().any(|t| t.contains_typaram_ref()),
        }
    }
}

/// Returns "" if the argument is empty.
/// Returns a string like "<A,B,C>" otherwise.
fn tyargs_str(type_args: &[TermTy]) -> String {
    if type_args.is_empty() {
        "".to_string()
    } else {
        let s = type_args
            .iter()
            .map(|x| x.fullname.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        format!("<{}>", &s)
    }
}

/// Format `type_args` with .dbg_str
fn _dbg_type_args(type_args: &[TermTy]) -> String {
    if type_args.is_empty() {
        "".to_string()
    } else {
        let s = type_args
            .iter()
            .map(|x| x.dbg_str())
            .collect::<Vec<_>>()
            .join(", ");
        format!("<{}>", &s)
    }
}

pub fn new(
    base_name_: impl Into<String>,
    type_args: Vec<TermTy>,
    is_meta: bool
) -> TermTy {
    let base_name = base_name_.into();
    debug_assert!(!base_name.is_empty());
    debug_assert!(!base_name.starts_with("Meta:"));
    debug_assert!(!base_name.contains('<'));
    let fullname = ClassFullname::new(
        format!("{}{}", &base_name, &tyargs_str(&type_args)),
        is_meta
    );
    TermTy {
        fullname,
        body: TyRaw(RawTy {
            base_name: base_name,
            type_args,
            is_meta
        })
    }
}

pub fn nonmeta(names: &[String], args: Vec<TermTy>) -> TermTy {
    ty::new(&names.join("::"), args, false)
}

pub fn raw(fullname_: impl Into<String>) -> TermTy {
    new(fullname_, Default::default(), false)
}

pub fn meta(base_fullname_: impl Into<String>) -> TermTy {
    new(base_fullname_, Default::default(), true)
}

pub fn spe(base_name_: impl Into<String>, type_args: Vec<TermTy>) -> TermTy {
    new(base_name_, type_args, false)
}

pub fn spe_meta(base_name_: impl Into<String>, type_args: Vec<TermTy>) -> TermTy {
    new(base_name_, type_args, true)
}

/// Create the type of return value of `.new` method of the class
pub fn return_type_of_new(classname: &ClassFullname, typarams: &[TyParam]) -> TermTy {
    if typarams.is_empty() {
        ty::raw(&classname.0)
    } else {
        let args = typarams
            .iter()
            .enumerate()
            .map(|(i, t)| typaram(&t.name, TyParamKind::Class, i))
            .collect::<Vec<_>>();
        ty::spe(&classname.0, args)
    }
}

/// Shortcut for Array<T>
pub fn ary(type_arg: TermTy) -> TermTy {
    spe("Array", vec![type_arg])
}

pub fn typaram(name: impl Into<String>, kind: TyParamKind, idx: usize) -> TermTy {
    let s = name.into();
    TermTy {
        // TODO: s is not a class name. `fullname` should be just a String
        fullname: class_fullname(s.clone()),
        body: TyParamRef {
            kind,
            name: s,
            idx,
            upper_bound: Box::new(ty::raw("Object")),
            lower_bound: Box::new(ty::raw("Never")),
        },
    }
}
