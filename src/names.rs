//#[derive(Debug, PartialEq, Clone)]
//pub struct ClassName(pub String);

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct ClassFullname(pub String);

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
