use crate::ty;
use crate::ty::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone)]
pub struct ClassFirstname(pub String);

impl std::fmt::Display for ClassFirstname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ClassFirstname {
    pub fn add_namespace(&self, namespace: &str) -> ClassFullname {
        if namespace.is_empty() {
            ClassFullname(self.0.clone())
        } else {
            ClassFullname(namespace.to_string() + "::" + &self.0)
        }
    }
}

pub fn class_firstname(s: impl Into<String>) -> ClassFirstname {
    ClassFirstname(s.into())
}

#[derive(Debug, PartialEq, Clone, Eq, Hash, Serialize, Deserialize)]
pub struct ClassFullname(pub String);

impl std::fmt::Display for ClassFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn class_fullname(s: impl Into<String>) -> ClassFullname {
    let name = s.into();
    debug_assert!(!name.starts_with("::"));
    debug_assert!(!name.starts_with("Meta:Meta:"));
    ClassFullname(name)
}

pub fn metaclass_fullname(base: &str) -> ClassFullname {
    debug_assert!(!base.starts_with("Meta:"));
    if base == "Class" {
        class_fullname("Class")
    } else {
        class_fullname(&("Meta:".to_string() + base))
    }
}

impl ClassFullname {
    pub fn instance_ty(&self) -> TermTy {
        ty::raw(&self.0)
    }

    pub fn class_ty(&self) -> TermTy {
        ty::meta(&self.0)
    }

    pub fn is_meta(&self) -> bool {
        self.0.starts_with("Meta:")
    }

    pub fn to_ty(&self) -> TermTy {
        if self.is_meta() {
            let mut name = self.0.clone();
            name.replace_range(0..=4, "");
            ty::meta(&name)
        } else {
            self.instance_ty()
        }
    }

    pub fn meta_name(&self) -> ClassFullname {
        debug_assert!(!self.0.starts_with("Meta:"));
        if self.0 == "Class" {
            self.clone()
        } else {
            ClassFullname("Meta:".to_string() + &self.0)
        }
    }

    pub fn method_fullname(&self, method_firstname: &MethodFirstname) -> MethodFullname {
        method_fullname(self, &method_firstname.0)
    }

    pub fn to_const_fullname(&self) -> ConstFullname {
        toplevel_const(&self.0)
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Hash, Serialize, Deserialize)]
pub struct MethodFirstname(pub String);

impl std::fmt::Display for MethodFirstname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn method_firstname(s: impl Into<String>) -> MethodFirstname {
    MethodFirstname(s.into())
}

impl MethodFirstname {
    pub fn append(&self, suffix: &str) -> MethodFirstname {
        MethodFirstname(self.0.clone() + suffix)
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Hash, Serialize, Deserialize)]
pub struct MethodFullname {
    pub full_name: String,
    pub first_name: MethodFirstname,
}

pub fn method_fullname(
    class_name: &ClassFullname,
    first_name_: impl Into<String>,
) -> MethodFullname {
    let first_name = first_name_.into();
    MethodFullname {
        full_name: class_name.0.clone() + "#" + &first_name,
        first_name: MethodFirstname(first_name),
    }
}

impl std::fmt::Display for MethodFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.full_name)
    }
}

impl MethodFullname {
    /// Returns true if this method isn't an instance method
    pub fn is_class_method(&self) -> bool {
        self.full_name.starts_with("Meta:")
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Hash, Serialize, Deserialize)]
pub struct ConstFullname(pub String);

impl std::fmt::Display for ConstFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn const_fullname(s_: impl Into<String>) -> ConstFullname {
    let s = s_.into();
    debug_assert!(!s.starts_with("::"));
    ConstFullname(format!("::{}", &s))
}

pub fn toplevel_const(first_name: &str) -> ConstFullname {
    debug_assert!(!first_name.starts_with("::"));
    ConstFullname(format!("::{}", first_name))
}

impl ConstFullname {
    /// Returns true if this const is not visible in Shiika level
    pub fn is_internal(&self) -> bool {
        self.0.contains("<internal>")
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Namespace(pub Vec<String>);

impl std::fmt::Display for Namespace {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "::{}", &self.string())
    }
}

impl Namespace {
    /// Create a namespace object
    pub fn new(names: Vec<String>) -> Namespace {
        debug_assert!(names.iter().all(|x| !x.contains("::")));
        Namespace(names)
    }

    /// Returns a toplevel namespace
    pub fn root() -> Namespace {
        Namespace::new(vec![])
    }

    /// Returns the hidden namespace
    pub fn internal() -> Namespace {
        Namespace::new(vec!["<internal>".to_string()])
    }

    /// Add `name` to the end of `self`
    pub fn add(&self, name: &ClassFirstname) -> Namespace {
        let mut v = self.0.clone();
        v.push(name.0.clone());
        Namespace::new(v)
    }

    /// Join Namespace and ClassFirstname
    pub fn class_fullname(&self, name: &ClassFirstname) -> ClassFullname {
        let n = self.string();
        if n.is_empty() {
            class_fullname(&name.0)
        } else {
            class_fullname(format!("{}::{}", n, &name.0))
        }
    }

    /// Returns fullname of the constant in this namespace
    pub fn const_fullname(&self, name: &str) -> ConstFullname {
        let n = self.string();
        if n.is_empty() {
            const_fullname(name)
        } else {
            const_fullname(format!("{}::{}", &n, name))
        }
    }

    pub fn head(&self, n: usize) -> &[String] {
        &self.0[0..n]
    }

    /// Number of names
    pub fn size(&self) -> usize {
        self.0.len()
    }

    /// Returns string representation of self
    pub fn string(&self) -> String {
        self.0.join("::")
    }
}

// REFACTOR: Rename to `UnresolvedTypeName` or something.
#[derive(Debug, PartialEq, Clone)]
pub struct ConstName {
    pub names: Vec<String>,
    pub args: Vec<ConstName>,
}

impl ConstName {
    /// Convert self to ResolvedConstName. `args` must be empty
    pub fn resolved(&self) -> ResolvedConstName {
        debug_assert!(self.args.is_empty());
        ResolvedConstName {
            names: self.names.clone(),
            args: vec![],
        }
    }

    // TODO: resolve class name instead of this
    pub fn to_ty(&self, class_typarams: &[String]) -> TermTy {
        if self.args.is_empty() {
            let s = self.names.join("::");
            if let Some(i) = class_typarams.iter().position(|name| *name == s) {
                ty::typaram(s, ty::TyParamKind::Class, i)
            } else {
                ty::raw(&self.names.join("::"))
            }
        } else {
            let type_args = self.args.iter().map(|n| n.to_ty(class_typarams)).collect();
            ty::spe(&self.names.join("::"), type_args)
        }
    }

    /// Returns if generic
    pub fn has_type_args(&self) -> bool {
        !self.args.is_empty()
    }

    /// Make ClassFullname from self
    pub fn to_class_fullname(&self) -> ClassFullname {
        class_fullname(&self.string())
    }

    /// Return const name as String
    pub fn fullname(&self) -> String {
        "::".to_string() + &self.string()
    }

    /// Return class name as String
    fn string(&self) -> String {
        let mut s = self.names.join("::");
        if !self.args.is_empty() {
            s += "<";
            let v = self.args.iter().map(|x| x.string()).collect::<Vec<_>>();
            s += &v.join(",");
            s += ">";
        }
        s
    }
}

pub fn const_name(names: Vec<String>) -> ConstName {
    ConstName {
        names,
        args: vec![],
    }
}

/// A const name not resolved yet
#[derive(Debug, PartialEq, Clone)]
pub struct UnresolvedConstName(pub Vec<String>);

/// Fully qualified const name.
#[derive(Debug, PartialEq, Clone)]
pub struct ResolvedConstName {
    pub names: Vec<String>,
    pub args: Vec<ResolvedConstName>,
}

impl std::fmt::Display for ResolvedConstName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.string())
    }
}

impl ResolvedConstName {
    pub fn new(names: Vec<String>, args: Vec<ResolvedConstName>) -> ResolvedConstName {
        ResolvedConstName { names, args }
    }

    pub fn unsafe_create(s: String) -> ResolvedConstName {
        ResolvedConstName {
            names: vec![s],
            args: vec![],
        }
    }

    /// Returns if generic
    pub fn has_type_args(&self) -> bool {
        !self.args.is_empty()
    }

    /// Returns `self` without type arguments
    pub fn base(&self) -> ResolvedConstName {
        ResolvedConstName {
            names: self.names.clone(),
            args: Default::default(),
        }
    }

    /// Convert to ConstFullname
    pub fn to_const_fullname(&self) -> ConstFullname {
        toplevel_const(&self.string())
    }

    /// Convert to ClassFullname
    pub fn to_class_fullname(&self) -> ClassFullname {
        class_fullname(self.string())
    }

    /// Returns string representation
    pub fn string(&self) -> String {
        let mut s = self.names.join("::");
        // Type args (optional)
        if !self.args.is_empty() {
            s += "<";
            let v = self.args.iter().map(|arg| arg.string()).collect::<Vec<_>>();
            s += &v.join(",");
            s += ">";
        }
        s
    }

    /// Apply type args to `self`. `self.args` must be empty.
    pub fn with_type_args(&self, args: Vec<ResolvedConstName>) -> ResolvedConstName {
        debug_assert!(self.args.is_empty());
        ResolvedConstName {
            names: self.names.clone(),
            args,
        }
    }

    /// Returns the instance type when this const refers to a class
    /// eg. "Object" -> `TermTy(Object)`
    pub fn to_ty(&self, class_typarams: &[String], method_typarams: &[String]) -> TermTy {
        if self.args.is_empty() {
            let s = self.names.join("::");
            if let Some(i) = class_typarams.iter().position(|name| *name == s) {
                ty::typaram(s, ty::TyParamKind::Class, i)
            } else if let Some(i) = method_typarams.iter().position(|name| *name == s) {
                ty::typaram(s, ty::TyParamKind::Method, i)
            } else {
                ty::raw(&self.names.join("::"))
            }
        } else {
            let type_args = self
                .args
                .iter()
                .map(|n| n.to_ty(class_typarams, method_typarams))
                .collect();
            ty::spe(&self.names.join("::"), type_args)
        }
    }
}

/// Create a ResolvedConstName (which is not generic).
pub fn resolved_const_name(namespace: Namespace, names: Vec<String>) -> ResolvedConstName {
    let new_names = namespace
        .0
        .into_iter()
        .chain(names.into_iter())
        .collect::<Vec<String>>();
    ResolvedConstName {
        names: new_names,
        args: vec![],
    }
}

// ad hoc. Not sure I'm doing right
pub fn typaram_as_resolved_const_name(name: impl Into<String>) -> ResolvedConstName {
    resolved_const_name(Namespace::root(), vec![name.into()])
}

// The constant `::Void` is an *instance* of the class `Void`. However we need
// the class object for `::Void.class`; Returns name for this internal constant
pub fn const_is_obj_class_internal_const_name(name: &ResolvedConstName) -> ResolvedConstName {
    debug_assert!(!name.has_type_args());
    resolved_const_name(Namespace::internal(), name.names.clone())
}
