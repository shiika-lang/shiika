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
#[derive(PartialEq, Clone)]
pub struct TermTy {
    pub fullname: ClassFullname,
    pub body: TyBody,
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
            TyGenMeta {
                base_name,
                typaram_names,
            } => format!("Meta:{}<{}>", base_name, typaram_names.join(", ")),
            TySpe {
                base_name,
                type_args,
            } => format!("{}<{}>", base_name, _dbg_type_args(type_args)),
            TySpeMeta {
                base_name,
                type_args,
            } => format!("Meta:{}<{}>", base_name, _dbg_type_args(type_args)),
            TyParamRef { kind, name, idx } => {
                let k = match kind {
                    TyParamKind::Class => "C",
                    TyParamKind::Method => "M",
                };
                format!("TyParamRef({} {}{})", name, idx, k)
            }
            _ => self.fullname.0.clone(),
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
        kind: TyParamKind,
        name: String,
        idx: usize,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum TyParamKind {
    /// eg. `class A<B>`
    Class,
    /// eg. `def foo<X>(...)`
    Method,
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

    // Returns true when this is the Never type
    pub fn is_never_type(&self) -> bool {
        match self.body {
            TyRaw => (self.fullname.0 == "Never"),
            _ => false,
        }
    }

    // Returns ret_ty if this is any of Fn0, .., Fn9
    pub fn fn_x_info(&self) -> Option<TermTy> {
        match &self.body {
            TySpe {
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
            TyRaw => ty::meta(&self.fullname.0),
            TyMeta { .. } => ty::class(),
            TyClass => ty::class(),
            TyGenMeta { .. } => ty::class(),
            TySpe {
                base_name,
                type_args,
            } => ty::spe_meta(&base_name, type_args.clone()),
            TySpeMeta { .. } => ty::class(),
            _ => panic!("TODO"),
        }
    }

    /// Return "A" for "A<B>", "Meta:A" for "Meta:A<B>"
    pub fn base_class_name(&self) -> ClassFullname {
        match &self.body {
            TySpe { base_name, .. } => class_fullname(base_name),
            TySpeMeta { base_name, .. } => class_fullname("Meta:".to_string() + base_name),
            _ => panic!("unexpected"),
        }
    }

    /// Return true if `self` conforms to `other` i.e.
    /// an object of the type `self` is included in the set of objects represented by the type `other`
    pub fn conforms_to(&self, other: &TermTy, class_dict: &ClassDict) -> bool {
        // `Never` is bottom type (i.e. subclass of any class)
        if self.is_never_type() {
            return true;
        }
        if let TyParamRef { name, .. } = &self.body {
            if let TyParamRef { name: name2, .. } = &other.body {
                name == name2
            } else {
                other == &ty::raw("Object") // The upper bound
            }
        } else if let TyParamRef { name, .. } = &other.body {
            if let TyParamRef { name: name2, .. } = &self.body {
                name == name2
            } else {
                false
            }
        } else if let TySpe {
            base_name,
            type_args,
        } = &self.body
        {
            if let TySpe {
                base_name: b2,
                type_args: a2,
            } = &other.body
            {
                if base_name != b2 {
                    return false;
                } // TODO: Relax this condition
                for (i, a) in type_args.iter().enumerate() {
                    // Invariant
                    if a.equals_to(&a2[i]) || a2[i].is_void_type() {
                        // ok
                    } else {
                        return false;
                    }
                }
                true
            } else {
                // eg. Passing a `Array<String>` for `Object`
                let base = ty::raw(base_name);
                class_dict.is_descendant(&base, other)
            }
        } else {
            self.equals_to(other) || class_dict.is_descendant(self, other)
        }
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
            TySpe { base_name, .. } => {
                match class_dict.get_superclass(&class_fullname(base_name)) {
                    Some(scls) => Some(ty::raw(&scls.fullname.0)),
                    None => panic!("unexpected"),
                }
            }
            TySpeMeta { base_name, .. } => {
                match class_dict.get_superclass(&class_fullname(base_name)) {
                    Some(scls) => Some(ty::meta(&scls.fullname.0)),
                    None => panic!("unexpected"),
                }
            }
            _ => panic!("TODO: {}", self),
        }
    }

    /// Apply type argments into type parameters
    /// - class_tyargs: None if the class is not generic (non-generic class
    ///   can have a generic method)
    /// - method_tyargs: None if not in a method context (eg. when creating
    ///   `Array<Int>` from `Array<T>`)
    pub fn substitute(
        &self,
        class_tyargs: Option<&[TermTy]>,
        method_tyargs: Option<&[TermTy]>,
    ) -> TermTy {
        match &self.body {
            TyParamRef { kind, idx, .. } => match kind {
                TyParamKind::Class => {
                    if let Some(tyargs) = class_tyargs {
                        tyargs[*idx].clone()
                    } else {
                        self.clone()
                    }
                }
                TyParamKind::Method => {
                    if let Some(tyargs) = method_tyargs {
                        tyargs[*idx].clone()
                    } else {
                        self.clone()
                    }
                }
            },
            TySpe {
                base_name,
                type_args,
            } => {
                let args = type_args
                    .iter()
                    .map(|t| t.substitute(class_tyargs, method_tyargs))
                    .collect();
                ty::spe(base_name, args)
            }
            TySpeMeta { .. } => todo!(),
            _ => self.clone(),
        }
    }

    /// Name for vtable when invoking a method on an object of this type
    pub fn vtable_name(&self) -> ClassFullname {
        match &self.body {
            TySpe { base_name, .. } => class_fullname(base_name),
            TySpeMeta { base_name, .. } => class_fullname(base_name),
            _ => self.fullname.clone(),
        }
    }

    pub fn is_specialized(&self) -> bool {
        match self.body {
            TySpe { .. } | TySpeMeta { .. } => true,
            _ => false,
        }
    }

    pub fn upper_bound(&self) -> TermTy {
        match &self.body {
            TyParamRef { .. } => ty::raw("Object"),
            TySpe {
                base_name,
                type_args,
            } => ty::spe(
                base_name,
                type_args.iter().map(|t| t.upper_bound()).collect(),
            ),
            TySpeMeta {
                base_name,
                type_args,
            } => ty::spe_meta(
                base_name,
                type_args.iter().map(|t| t.upper_bound()).collect(),
            ),
            _ => self.clone(),
        }
    }
}

pub fn raw(fullname: &str) -> TermTy {
    debug_assert!(!fullname.contains('<'), fullname.to_string());
    TermTy {
        fullname: class_fullname(fullname),
        body: TyRaw,
    }
}

pub fn meta(base_fullname: &str) -> TermTy {
    debug_assert!(!base_fullname.contains('<'), base_fullname.to_string());
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

pub fn spe_meta(base_name: &str, type_args: Vec<TermTy>) -> TermTy {
    let tyarg_names = type_args
        .iter()
        .map(|x| x.fullname.0.to_string())
        .collect::<Vec<_>>();
    TermTy {
        fullname: class_fullname(&format!("Meta:{}<{}>", &base_name, &tyarg_names.join(","))),
        body: TySpeMeta {
            base_name: base_name.to_string(),
            type_args,
        },
    }
}

/// Create the type of return value of `.new` method of the class
pub fn return_type_of_new(classname: &ClassFullname, typarams: &[String]) -> TermTy {
    if typarams.is_empty() {
        ty::raw(&classname.0)
    } else {
        let args = typarams
            .iter()
            .enumerate()
            .map(|(i, s)| TermTy {
                fullname: class_fullname(s),
                body: TyParamRef {
                    kind: TyParamKind::Class,
                    name: s.to_string(),
                    idx: i,
                },
            })
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
        body: TyParamRef { kind, name: s, idx },
    }
}

/// A type parameter
/// In the future, may have something like +T/-T or in/out
#[derive(Debug, PartialEq, Clone)]
pub struct TyParam {
    pub name: String,
}

/// Return the nearest common ancestor of the classes
pub fn nearest_common_ancestor(ty1: &TermTy, ty2: &TermTy, class_dict: &ClassDict) {
    let ancestors1 = class_dict.ancestor_types(ty1);
    let ancestors2 = class_dict.ancestor_types(ty2);
    for t2 in ancestors2 {
        if let Some(eq) = ancestors1.iter().find(|t1| t1.equals_to(&t2)) {
            return eq.clone();
        }
    }
    panic!("[BUG] nearest_common_ancestor_type not found");
}
