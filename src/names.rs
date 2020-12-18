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
    ClassFullname("Meta:".to_string() + base)
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
        ClassFullname("Meta:".to_string() + &self.0)
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

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct ConstFirstname(pub String);

impl std::fmt::Display for ConstFirstname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
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
    ConstFullname(s.to_string())
}

#[derive(Debug, PartialEq, Clone)]
pub struct ConstName {
    pub names: Vec<String>,
    pub args: Vec<ConstName>,
}

impl ConstName {
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

    /// Make TermTy form self
    pub fn to_ty(&self) -> TermTy {
        if self.args.is_empty() {
            ty::raw(&self.names.join("::"))
        } else {
            let type_args = self.args.iter().map(|n| n.to_ty()).collect();
            ty::spe(&self.names.join("::"), type_args)
        }
    }

    /// Make TermTy of the class
    /// eg. for `::Array<Int>`, returns `TermTy(Meta:Array<Int>)`, which is
    /// the type of the constant
    pub fn class_ty(&self) -> TermTy {
        self.to_ty().meta_ty()
    }
}

pub fn const_name(names: Vec<String>) -> ConstName {
    ConstName {
        names,
        args: vec![],
    }
}
