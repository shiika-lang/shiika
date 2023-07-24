use crate::ty::LitTy;
use nom::IResult;
use serde::{Deserialize, Serialize};

/// A type parameter
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TyParam {
    pub name: String,
    pub variance: Variance,
    pub upper_bound: LitTy,
    pub lower_bound: LitTy,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum Variance {
    Invariant,
    Covariant,     // eg. `in T`
    Contravariant, // eg. `out T`
}

impl TyParam {
    pub fn new(name: impl Into<String>, variance: Variance) -> TyParam {
        TyParam {
            name: name.into(),
            variance,
            upper_bound: LitTy::raw("Object"),
            lower_bound: LitTy::raw("Never"),
        }
    }

    /// Returns a serialized string which can be parsed by `deserialize`
    pub fn serialize(&self) -> String {
        let flag = match &self.variance {
            Variance::Invariant => "",
            Variance::Covariant => "+",
            Variance::Contravariant => "-",
        };
        format!("{}{}", flag, &self.name)
    }

    /// nom parser for TyParam
    pub fn deserialize(s: &str) -> IResult<&str, TyParam> {
        let (s, c) = nom::combinator::opt(nom::character::complete::one_of("+-"))(s)?;
        let variance = match c {
            Some('+') => Variance::Covariant,
            Some('-') => Variance::Contravariant,
            _ => Variance::Invariant,
        };

        let (s, name) = nom::character::complete::alphanumeric1(s)?;
        Ok((s, TyParam::new(name.to_string(), variance)))
    }
}
