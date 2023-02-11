use super::type_name::*;
use serde::{de, ser, Deserialize, Serialize};
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, PartialEq, Clone, Eq, Hash, Serialize, Deserialize)]
pub struct MethodFirstname(pub String);

impl std::fmt::Display for MethodFirstname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn method_firstname(s: impl Into<String>) -> MethodFirstname {
    MethodFirstname(s.into())
}

impl MethodFirstname {
    pub fn append(&self, suffix: &str) -> MethodFirstname {
        MethodFirstname(self.0.clone() + suffix)
    }
}

#[derive(PartialEq, Clone, Eq)]
pub struct MethodFullname {
    // class/module part
    pub type_name: TypeFullname,
    // method part
    pub first_name: MethodFirstname,
    // cache
    pub full_name: String,
}

impl Hash for MethodFullname {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.full_name.hash(state);
    }
}

impl fmt::Debug for MethodFullname {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MethodFullname(`{}`)", &self.full_name)
    }
}

pub fn method_fullname(type_name: TypeFullname, first_name_: impl Into<String>) -> MethodFullname {
    let first_name = first_name_.into();
    debug_assert!(!first_name.is_empty());
    debug_assert!(!first_name.starts_with('@'));
    let full_name = type_name.0.clone() + "#" + &first_name;
    MethodFullname {
        type_name,
        full_name,
        first_name: MethodFirstname(first_name),
    }
}

pub fn method_fullname_raw(cls: impl Into<String>, method: impl Into<String>) -> MethodFullname {
    method_fullname(type_fullname(cls), method)
}

impl std::fmt::Display for MethodFullname {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.full_name)
    }
}

impl MethodFullname {
    /// Returns true if this method isn't an instance method
    pub fn is_class_method(&self) -> bool {
        self.full_name.starts_with("Meta:")
    }
}

//
// serde - simplify JSON representation
//
impl ser::Serialize for MethodFullname {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.full_name)
    }
}

struct MethodFullnameVisitor;
impl<'de> de::Visitor<'de> for MethodFullnameVisitor {
    type Value = MethodFullname;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        formatter.write_str("a MethodFullname")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let parts = v.split("#").collect::<Vec<_>>();
        if parts.len() == 2 {
            Ok(method_fullname_raw(parts[0], parts[1]))
        } else {
            Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &"something like `Foo#bar'",
            ))
        }
    }
}

impl<'de> de::Deserialize<'de> for MethodFullname {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as de::Deserializer<'de>>::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_identifier(MethodFullnameVisitor)
    }
}
