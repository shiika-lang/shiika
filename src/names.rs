use crate::ty;
use crate::ty::*;

#[derive(Debug, PartialEq, Clone)]
pub struct ClassFirstname(pub String);

impl ClassFirstname {
    pub fn add_namespace(&self, namespace: &str) -> ClassFullname {
        if namespace == "" {
            ClassFullname(self.0.clone())
        } else {
            ClassFullname(namespace.to_string() + "::" + &self.0)
        }
    }
}

pub fn class_firstname(s: &str) -> ClassFirstname {
    ClassFirstname(s.to_string())
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct ClassFullname(pub String);

impl std::fmt::Display for ClassFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn class_fullname(s: impl Into<String>) -> ClassFullname {
    ClassFullname(s.into())
}

pub fn metaclass_fullname(base: &str) -> ClassFullname {
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
        if self.0 == "Class" {
            self.clone()
        } else {
            ClassFullname("Meta:".to_string() + &self.0)
        }
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct MethodFirstname(pub String);

impl std::fmt::Display for MethodFirstname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn method_firstname(s: &str) -> MethodFirstname {
    MethodFirstname(s.to_string())
}

impl MethodFirstname {
    pub fn append(&self, suffix: &str) -> MethodFirstname {
        MethodFirstname(self.0.clone() + suffix)
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct MethodFullname {
    pub full_name: String,
    pub first_name: MethodFirstname,
}

pub fn method_fullname(class_name: &ClassFullname, first_name: &str) -> MethodFullname {
    MethodFullname {
        full_name: class_name.0.clone() + "#" + first_name,
        first_name: MethodFirstname(first_name.to_string()),
    }
}

impl std::fmt::Display for MethodFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.full_name)
    }
}

impl MethodFullname {
    /// Returns true if this is any of `Fn0#call`, ..., `Fn9#call`
    pub fn is_fn_x_call(&self) -> bool {
        for i in 0..=9 {
            if self.full_name == format!("Fn{}#call", i) {
                return true;
            }
        }
        false
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct ConstFirstname(pub String);

impl std::fmt::Display for ConstFirstname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ConstFirstname {
    pub fn add_namespace(&self, namespace: &str) -> ConstFullname {
        const_fullname(&("::".to_string() + namespace + "::" + &self.0))
    }
}

pub fn const_firstname(s: &str) -> ConstFirstname {
    ConstFirstname(s.to_string())
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct ConstFullname(pub String);

impl std::fmt::Display for ConstFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn const_fullname(s: &str) -> ConstFullname {
    debug_assert!(s.starts_with("::"));
    debug_assert!(!s.starts_with("::::"));
    ConstFullname(s.to_string())
}

#[derive(Debug, PartialEq, Clone)]
pub struct ConstName {
    pub names: Vec<String>,
    pub args: Vec<ConstName>,
}

impl ConstName {
    /// Make ConstFullname prefixed by `namespace`
    pub fn under_namespace(&self, namespace: &str) -> ConstFullname {
        let s = if namespace.is_empty() {
            "::".to_string() + &self.string()
        } else {
            "::".to_string() + namespace + "::" + &self.string()
        };
        const_fullname(&s)
    }

    /// Make ConstFullname from self
    pub fn to_const_fullname(&self) -> ConstFullname {
        const_fullname(&self.fullname())
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
    pub fn string(&self) -> String {
        let mut s = self.names.join("::");
        if !self.args.is_empty() {
            s += "<";
            let v = self.args.iter().map(|x| x.string()).collect::<Vec<_>>();
            s += &v.join(",");
            s += ">";
        }
        s
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

pub fn const_name(names: Vec<String>) -> ConstName {
    ConstName {
        names,
        args: vec![],
    }
}
