use super::class_name::{metaclass_fullname, ClassFullname};
use super::const_name::{const_fullname, ConstFullname};
use super::type_name::{type_fullname, TypeFullname};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq)]
pub struct ModuleFirstname(pub String);

impl std::fmt::Display for ModuleFirstname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ModuleFirstname {
    pub fn add_namespace(&self, namespace: &str) -> ModuleFullname {
        if namespace.is_empty() {
            module_fullname(self.0.clone())
        } else {
            module_fullname(namespace.to_string() + "::" + &self.0)
        }
    }
}

pub fn module_firstname(s: impl Into<String>) -> ModuleFirstname {
    ModuleFirstname(s.into())
}

#[derive(Debug, PartialEq, Clone, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleFullname(pub String);

impl std::fmt::Display for ModuleFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ModuleFullname> for TypeFullname {
    fn from(x: ModuleFullname) -> Self {
        type_fullname(x.0)
    }
}

impl ModuleFullname {
    pub fn to_type_fullname(&self) -> TypeFullname {
        type_fullname(&self.0)
    }

    pub fn to_const_fullname(&self) -> ConstFullname {
        const_fullname(&self.0)
    }

    pub fn meta_name(&self) -> ClassFullname {
        metaclass_fullname(&self.0)
    }
}

pub fn module_fullname(s: impl Into<String>) -> ModuleFullname {
    let name = s.into();
    debug_assert!(name != "Meta:");
    debug_assert!(!name.starts_with("::"));
    debug_assert!(!name.starts_with("Meta:Meta:"));
    ModuleFullname(name)
}
