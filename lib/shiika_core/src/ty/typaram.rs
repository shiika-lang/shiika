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
    Covariant,     // eg. `out T`
    Contravariant, // eg. `in T`
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
        let bound =
            if self.upper_bound.base_name == "Object" && self.lower_bound.base_name == "Never" {
                "".to_string()
            } else {
                format!(
                    ":{}~{}",
                    self.upper_bound.serialize(),
                    self.lower_bound.serialize()
                )
            };
        format!("{}{}{}", flag, &self.name, bound)
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
        let mut typaram = TyParam::new(name.to_string(), variance);

        let (s, opt_bounds) = nom::combinator::opt(super::typaram_ref::parse_bounds)(s)?;
        if let Some((upper_bound, lower_bound)) = opt_bounds {
            typaram.upper_bound = upper_bound;
            typaram.lower_bound = lower_bound;
        }
        Ok((s, typaram))
    }
}

#[test]
fn serialize_typaram_test() {
    let mut tp = TyParam::new("T", Variance::Covariant);
    tp.upper_bound = LitTy::raw("Ord");
    let s = tp.serialize();
    assert_eq!(s, "+T:Ord~Never");
    assert_eq!(TyParam::deserialize(&s), Ok(("", tp)));

    // Default bounds are omitted (and can be parsed back)
    let tp2 = TyParam::new("U", Variance::Invariant);
    assert_eq!(tp2.serialize(), "U");
    assert_eq!(TyParam::deserialize("U"), Ok(("", tp2)));
}
