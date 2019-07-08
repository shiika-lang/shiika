//#[derive(Debug, PartialEq, Clone)]
//pub struct ClassName(pub String);
//#[derive(Debug, PartialEq, Clone)]
//pub struct ClassFullname(pub String);
#[derive(Debug, PartialEq, Clone)]
pub struct MethodName(pub String);

#[derive(Debug, PartialEq, Clone)]
pub struct MethodFullname(pub String);
impl std::fmt::Display for MethodFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
