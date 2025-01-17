use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FunctionName {
    Unmangled(String),
    Mangled(String),
}

impl fmt::Display for FunctionName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FunctionName::Unmangled(name) => write!(f, "{}", name),
            FunctionName::Mangled(name) => write!(f, "{}", name),
        }
    }
}

impl FunctionName {
    pub fn unmangled(name: impl Into<String>) -> FunctionName {
        FunctionName::Unmangled(name.into())
    }

    pub fn method(class_name: impl AsRef<String>, name: impl AsRef<String>) -> FunctionName {
        FunctionName::Unmangled(format!("{}#{}", class_name.as_ref(), name.as_ref()))
    }

    pub fn mangled(name: impl Into<String>) -> FunctionName {
        FunctionName::Mangled(name.into())
    }

    pub fn mangle(&self) -> String {
        match self {
            FunctionName::Unmangled(name) => shiika_ffi::mangle_method(name),
            FunctionName::Mangled(name) => name.clone(),
        }
    }
}
