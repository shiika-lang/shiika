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
    /// Types of non-meta classes
    /// eg. "Int", "String", "Array<Int>", "Pair<Bool, Object>", etc.
    TyRaw {
        base_name: String,
        type_args: Vec<TermTy>,
    },
    /// Types corresponds to metaclass
    /// eg. "Meta:Int", "Meta:String", "Meta:Array<Int>"
    TyMeta {
        base_name: String, // eg. "Pair"
        type_args: Vec<TermTy>,
    },
    // This object belongs to the class `Metaclass` (i.e. this is a class object)
    TyMetaclass,
    // Type parameter reference eg. `T`
    TyParamRef {
        kind: TyParamKind,
        name: String,
        idx: usize,
        upper_bound: Box<TermTy>,
        lower_bound: Box<TermTy>,
    },
}
use TyBody::*;

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
            TyRaw {
                base_name,
                type_args,
            } => {
                if type_args.is_empty() {
                    self.fullname.0.clone()
                } else {
                    format!(
                    "\x1b[32m{}<\x1b[0m{}\x1b[32m>\x1b[0m",
                    base_name,
                    _dbg_type_args(type_args)
                    )
                }
            },
            TyMeta {
                base_name,
                type_args,
            } => {
                if type_args.is_empty() {
                    self.fullname.0.clone()
                } else {
                    format!("Meta:{}<{}>", base_name, _dbg_type_args(type_args))
                }
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
            _ => self.fullname.0.clone(),
        }
    }

    /// Returns if value of this type is class
    pub fn is_metaclass(&self) -> bool {
        matches!(
            &self.body,
            TyMeta { .. } | TyMetaclass
        )
    }

    /// Returns if this is TyParamRef
    pub fn is_typaram_ref(&self) -> bool {
        matches!(&self.body, TyParamRef { .. })
    }

    pub fn to_const_fullname(&self) -> ConstFullname {
        match &self.body {
            TyMeta {
                base_name,
                type_args,
            } => {
                if type_args.is_empty() {
                    toplevel_const(&base_name)
                } else {
                    let args = type_args
                        .iter()
                        .map(|t| t.fullname.0.clone())
                        .collect::<Vec<_>>()
                        .join(",");
                    toplevel_const(&format!("{}<{}>", base_name, args))
                }
            }
            _ => panic!("[BUG] to_const_fullname called on {:?}", &self),
        }
    }

    // Returns true when this is the Void type
    pub fn is_void_type(&self) -> bool {
        match self.body {
            TyRaw { .. } => (self.fullname.0 == "Void"),
            _ => false,
        }
    }

    // Returns true when this is the Never type
    pub fn is_never_type(&self) -> bool {
        match self.body {
            TyRaw { .. } => (self.fullname.0 == "Never"),
            _ => false,
        }
    }

    // Returns ret_ty if this is any of Fn0, .., Fn9
    pub fn fn_x_info(&self) -> Option<TermTy> {
        match &self.body {
            TyRaw {
                base_name,
                type_args,
            } => {
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
            TyRaw {
                base_name,
                type_args,
            } => {
                if type_args.is_empty() {
                    ty::meta(&self.fullname.0)
                } else {
                    ty::spe_meta(base_name, type_args.clone())
                }
            },
            TyMeta { .. } => ty::metaclass(),
            TyMetaclass => ty::metaclass(),
            _ => panic!("TODO"),
        }
    }

    pub fn instance_ty(&self) -> TermTy {
        match &self.body {
            TyMeta {
                base_name,
                type_args,
            } => ty::spe(base_name, type_args.to_vec()),
            _ => panic!("instance_ty is undefined for {:?}", self),
        }
    }

    pub fn specialized_ty(&self, tyargs: Vec<TermTy>) -> TermTy {
        match &self.body {
            TyRaw { base_name, type_args } => {
                debug_assert!(type_args.len() == tyargs.len());
                ty::spe(base_name, tyargs)
            },
            TyMeta { base_name, type_args } => {
                debug_assert!(type_args.len() == tyargs.len());
                ty::spe_meta(base_name, tyargs)
            }
            _ => panic!("unexpected"),
        }
    }

    /// Return "A" for "A<B>", "Meta:A" for "Meta:A<B>"
    pub fn base_class_name(&self) -> ClassFullname {
        match &self.body {
            TyRaw { base_name, .. } => class_fullname(base_name),
            TyMeta { base_name, .. } => class_fullname("Meta:".to_string() + base_name),
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
            TyRaw { base_name, .. } => class_fullname(base_name),
            TyMeta { base_name, .. } => metaclass_fullname(base_name),
            TyMetaclass => class_fullname("Metaclass"),
            _ => todo!(),
        }
        // REFACTOR: technically, this can return &ClassFullname instead of ClassFullname.
        // To do this, TyRaw.base_name etc. should be a ClassFullname rather than a String.
    }

    pub fn erasure_ty(&self) -> TermTy {
        ty::raw(self.erasure().0)
    }

    /// Returns type arguments, if any
    pub fn tyargs(&self) -> &[TermTy] {
        match &self.body {
            TyRaw { type_args, .. } | TyMeta { type_args, .. } => type_args,
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
            TyRaw {
                base_name,
                type_args,
            } => {
                let args = type_args
                    .iter()
                    .map(|t| t.substitute(class_tyargs, method_tyargs))
                    .collect();
                ty::spe(base_name, args)
            },
            TyMeta {
                base_name,
                type_args,
            } => {
                let args = type_args
                    .iter()
                    .map(|t| t.substitute(class_tyargs, method_tyargs))
                    .collect();
                ty::spe_meta(base_name, args)
            },
            _ => self.clone(),
        }
    }

    /// Name for vtable when invoking a method on an object of this type
    pub fn vtable_name(&self) -> ClassFullname {
        match &self.body {
            TyRaw { base_name, .. } => class_fullname(base_name),
            TyMeta { base_name, .. } => metaclass_fullname(base_name),
            _ => self.fullname.clone(),
        }
    }

    pub fn is_specialized(&self) -> bool {
        match &self.body {
            TyRaw { type_args, .. } => !type_args.is_empty(),
            TyMeta { type_args, .. } => !type_args.is_empty(),
            _ => false,
        }
    }

    pub fn contains_typaram_ref(&self) -> bool {
        match &self.body {
            TyParamRef { .. } => true,
            TyRaw { type_args, .. } => type_args.iter().any(|t| t.contains_typaram_ref()),
            TyMeta { type_args, .. } => type_args.iter().any(|t| t.contains_typaram_ref()),
            _ => false,
        }
    }
}

/// Format `type_args` with .dbg_str
fn _dbg_type_args(type_args: &[TermTy]) -> String {
    type_args
        .iter()
        .map(|x| x.dbg_str())
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn nonmeta(names: &[String], args: Vec<TermTy>) -> TermTy {
    if args.is_empty() {
        ty::raw(&names.join("::"))
    } else {
        ty::spe(&names.join("::"), args)
    }
}

pub fn raw(fullname_: impl Into<String>) -> TermTy {
    let fullname = fullname_.into();
    debug_assert!(!fullname.contains('<'));
    TermTy {
        fullname: class_fullname(&fullname),
        body: TyRaw {
            base_name: fullname,
            type_args: Default::default(),
        }
    }
}

pub fn meta(base_fullname_: impl Into<String>) -> TermTy {
    let base_fullname = base_fullname_.into();
    debug_assert!(!base_fullname.is_empty());
    debug_assert!(!base_fullname.contains('<'));
    TermTy {
        fullname: metaclass_fullname(&base_fullname),
        body: TyMeta {
            base_name: base_fullname,
            type_args: Default::default(),
        },
    }
}

pub fn metaclass() -> TermTy {
    TermTy {
        fullname: class_fullname("Metaclass"),
        body: TyMetaclass,
    }
}

pub fn spe(base_name_: impl Into<String>, type_args: Vec<TermTy>) -> TermTy {
    let base_name = base_name_.into();
    debug_assert!(!base_name.starts_with("Meta:"));
    debug_assert!(!type_args.is_empty());
    let tyarg_names = type_args
        .iter()
        .map(|x| x.fullname.0.to_string())
        .collect::<Vec<_>>();
    TermTy {
        fullname: class_fullname(&format!("{}<{}>", &base_name, &tyarg_names.join(","))),
        body: TyRaw {
            base_name,
            type_args,
        },
    }
}

pub fn spe_meta(base_name_: impl Into<String>, type_args: Vec<TermTy>) -> TermTy {
    let base_name = base_name_.into();
    let tyarg_names = type_args
        .iter()
        .map(|x| x.fullname.0.to_string())
        .collect::<Vec<_>>();
    TermTy {
        fullname: class_fullname(&format!("Meta:{}<{}>", &base_name, &tyarg_names.join(","))),
        body: TyMeta {
            base_name,
            type_args,
        },
    }
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
