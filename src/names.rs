#[derive(Debug, PartialEq, Clone)]
pub struct ClassFirstName(pub String);

impl ClassFirstName {
    // TODO: remove this after nested class is supported
    pub fn to_class_fullname(&self) -> ClassFullname {
        ClassFullname(self.0.clone())
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct ClassFullname(pub String);

impl std::fmt::Display for ClassFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct MethodFirstName(pub String);

impl std::fmt::Display for MethodFirstName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct MethodFullname {
    pub full_name: String,
    pub first_name: MethodFirstName,
}

impl std::fmt::Display for MethodFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.full_name)
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct ConstFullname(pub String);

impl std::fmt::Display for ConstFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
