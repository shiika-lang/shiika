use shiika_core::names::MethodFullname;
use skc_hir::MethodSignature;
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

impl From<MethodSignature> for FunctionName {
    fn from(sig: MethodSignature) -> FunctionName {
        FunctionName::from_sig(&sig)
    }
}

impl From<&MethodSignature> for FunctionName {
    fn from(sig: &MethodSignature) -> FunctionName {
        FunctionName::from_sig(sig)
    }
}

impl From<MethodFullname> for FunctionName {
    fn from(name: MethodFullname) -> FunctionName {
        FunctionName::Unmangled(name.full_name)
    }
}

impl From<&MethodFullname> for FunctionName {
    fn from(name: &MethodFullname) -> FunctionName {
        FunctionName::Unmangled(name.full_name.clone())
    }
}

impl FunctionName {
    pub fn unmangled(name: impl Into<String>) -> FunctionName {
        FunctionName::Unmangled(name.into())
    }

    pub fn from_sig(sig: &MethodSignature) -> FunctionName {
        FunctionName::unmangled(&sig.fullname.full_name)
    }

    pub fn method(class_name: impl AsRef<str>, name: impl AsRef<str>) -> FunctionName {
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

    /// Break FunctionName into class name and method name.
    // REFACTOR: FunctionName should hold the original MethodFullname
    pub fn split(&self) -> Option<(&str, &str)> {
        match self {
            FunctionName::Unmangled(name) => {
                let parts: Vec<&str> = name.split('#').collect();
                if parts.len() == 2 {
                    Some((parts[0], parts[1]))
                } else {
                    // eg. shiika_init_const_core
                    None
                }
            }
            FunctionName::Mangled(name) => panic!("Cannot split mangled function name: {}", name),
        }
    }
}
