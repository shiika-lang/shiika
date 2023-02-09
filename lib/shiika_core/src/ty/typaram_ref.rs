use crate::names::class_fullname;
use crate::ty::lit_ty::{parse_lit_ty, LitTy};
use crate::ty::term_ty::{TermTy, TyBody};
use nom::{bytes::complete::tag, IResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct TyParamRef {
    pub kind: TyParamKind,
    pub name: String,
    pub idx: usize,
    pub upper_bound: LitTy,
    pub lower_bound: LitTy,
    /// Whether referring this typaram as a class object (eg. `p T`)
    pub as_class: bool,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum TyParamKind {
    /// eg. `class A<B>`
    Class,
    /// eg. `def foo<X>(...)`
    Method,
}

impl From<TyParamRef> for TermTy {
    fn from(x: TyParamRef) -> Self {
        x.into_term_ty()
    }
}

impl TyParamRef {
    pub fn dbg_str(&self) -> String {
        let k = match &self.kind {
            TyParamKind::Class => "C",
            TyParamKind::Method => "M",
        };
        let c = if self.as_class { "!" } else { " " };
        format!("TyParamRef({}{}{}{})", &self.name, c, &self.idx, k)
    }

    pub fn to_term_ty(&self) -> TermTy {
        self.clone().into_term_ty()
    }

    pub fn into_term_ty(self) -> TermTy {
        TermTy {
            // TODO: self.name (eg. "T") is not a class name. Should remove fullname from TermTy?
            fullname: class_fullname(&self.name),
            body: TyBody::TyPara(self),
        }
    }

    /// Create new `TyParamRef` from self with as_class: true
    pub fn as_class(&self) -> TyParamRef {
        debug_assert!(!self.as_class);
        let mut ref2 = self.clone();
        ref2.as_class = true;
        ref2
    }

    /// Create new `TyParamRef` from self with as_class: false
    pub fn as_type(&self) -> TyParamRef {
        debug_assert!(self.as_class);
        let mut ref2 = self.clone();
        ref2.as_class = false;
        ref2
    }

    /// Returns a serialized string which can be parsed by `parse_typaram_ref`
    pub fn serialize(&self) -> String {
        let c = if self.as_class { "!" } else { ":" };
        let k = match &self.kind {
            TyParamKind::Class => "C",
            TyParamKind::Method => "M",
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
        format!("^{}{}{}{}{}", &self.name, c, &self.idx, k, bound)
    }
}

/// nom parser for TyParamRef
pub fn parse_typaram_ref(s: &str) -> IResult<&str, TyParamRef> {
    let (s, _) = tag("^")(s)?;

    let (s, name) = nom::character::complete::alphanumeric1(s)?;

    let (s, c) = nom::character::complete::one_of("!:")(s)?;
    let as_class = c == '!';

    let get_nums = nom::character::complete::digit1;
    let (s, idx) = nom::combinator::map_res(nom::combinator::recognize(get_nums), str::parse)(s)?;

    let (s, c) = nom::branch::alt((tag("C"), tag("M")))(s)?;
    let kind = if c == "C" {
        TyParamKind::Class
    } else {
        TyParamKind::Method
    };

    let (s, opt_bounds) = nom::combinator::opt(parse_bounds)(s)?;
    let (upper_bound, lower_bound) =
        opt_bounds.unwrap_or_else(|| (LitTy::raw("Object"), LitTy::raw("Never")));

    let tpref = TyParamRef {
        kind,
        name: name.to_string(),
        idx,
        upper_bound,
        lower_bound,
        as_class,
    };
    Ok((s, tpref))
}

pub fn parse_bounds(s: &str) -> IResult<&str, (LitTy, LitTy)> {
    let (s, _) = tag(":")(s)?;
    let (s, upper_bound) = parse_lit_ty(s)?;
    let (s, _) = tag("~")(s)?;
    let (s, lower_bound) = parse_lit_ty(s)?;
    Ok((s, (upper_bound, lower_bound)))
}

#[test]
fn parse_typaram_ref_test() {
    assert_eq!(
        parse_typaram_ref("^V:0C"),
        Ok((
            "",
            TyParamRef {
                kind: TyParamKind::Class,
                name: "V".to_string(),
                idx: 0,
                upper_bound: LitTy::raw("Object"),
                lower_bound: LitTy::raw("Never"),
                as_class: false,
            }
        ))
    );
}
