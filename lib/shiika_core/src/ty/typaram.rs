use serde::{Deserialize, Serialize};

/// A type parameter
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct TyParam {
    pub name: String,
    pub variance: Variance,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Variance {
    Invariant,
    Covariant,     // eg. `in T`
    Contravariant, // eg. `out T`
}

impl TyParam {
    pub fn new(name: impl Into<String>) -> TyParam {
        TyParam {
            name: name.into(),
            variance: Variance::Invariant,
        }
    }
}

