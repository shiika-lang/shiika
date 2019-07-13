#[derive(Debug, PartialEq, Clone)]
pub struct ClassName(pub String);

impl ClassName {
    // TODO: remove this after nested class is supported
    pub fn to_class_fullname(&self) -> ClassFullname {
        ClassFullname(self.0.clone())
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct ClassFullname(pub String);

impl ClassFullname {
    pub fn metaclass_fullname(&self) -> ClassFullname {
        ClassFullname("Meta:".to_string() + &self.0)
    }
}

impl std::fmt::Display for ClassFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct MethodName(pub String);

impl std::fmt::Display for MethodName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct MethodFullname(pub String);

impl std::fmt::Display for MethodFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
