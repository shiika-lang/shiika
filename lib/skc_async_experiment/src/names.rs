#[derive(Debug, Clone, PartialEq)]
pub enum FunctionName {
    Unmangled(String),
    Mangled(String),
}

impl FunctionName {
    pub fn mangled(&self) -> String {
        match self {
            FunctionName::Unmangled(name) => shiika_ffi::mangle_method(name),
            FunctionName::Mangled(name) => name.clone(),
        }
    }
}
