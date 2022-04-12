use super::class_name::*;
use super::const_name::*;

#[derive(Debug, PartialEq, Clone)]
pub struct Namespace(pub Vec<String>);

impl std::fmt::Display for Namespace {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "::{}", &self.string())
    }
}

impl Namespace {
    /// Create a namespace object
    pub fn new(names: Vec<String>) -> Namespace {
        debug_assert!(names.iter().all(|x| !x.contains("::")));
        Namespace(names)
    }

    /// Returns a toplevel namespace
    pub fn root() -> Namespace {
        Namespace::new(vec![])
    }

    /// Add `name` to the end of `self`
    pub fn add(&self, name: &ClassFirstname) -> Namespace {
        let mut v = self.0.clone();
        v.push(name.0.clone());
        Namespace::new(v)
    }

    /// Join Namespace and ClassFirstname
    pub fn class_fullname(&self, name: &ClassFirstname) -> ClassFullname {
        let n = self.string();
        if n.is_empty() {
            class_fullname(&name.0)
        } else {
            class_fullname(format!("{}::{}", n, &name.0))
        }
    }

    /// Returns fullname of the constant in this namespace
    pub fn const_fullname(&self, name: &str) -> ConstFullname {
        let n = self.string();
        if n.is_empty() {
            const_fullname(name)
        } else {
            const_fullname(format!("{}::{}", &n, name))
        }
    }

    pub fn head(&self, n: usize) -> &[String] {
        &self.0[0..n]
    }

    /// Number of names
    pub fn size(&self) -> usize {
        self.0.len()
    }

    /// Returns string representation of self
    pub fn string(&self) -> String {
        self.0.join("::")
    }
}
