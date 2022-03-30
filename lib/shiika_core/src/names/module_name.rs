use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
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

pub fn module_fullname(s: impl Into<String>) -> ModuleFullname {
    let name = s.into();
    debug_assert!(name != "Meta:");
    debug_assert!(!name.starts_with("::"));
    debug_assert!(!name.starts_with("Meta:Meta:"));
    ModuleFullname(name)
}
