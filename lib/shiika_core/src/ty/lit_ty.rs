use super::erasure::Erasure;
use super::term_ty::TermTy;
use crate::ty;
use nom::{bytes::complete::tag, IResult};
use serde::{Deserialize, Serialize};

/// "Literal" type i.e. types that are not type parameter reference.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct LitTy {
    // REFACTOR: ideally these should be private
    pub base_name: String,
    pub type_args: Vec<TermTy>,
    /// `true` if values of this type are classes
    pub is_meta: bool,
}

impl From<LitTy> for TermTy {
    fn from(x: LitTy) -> Self {
        x.into_term_ty()
    }
}

impl LitTy {
    pub fn new(base_name: String, type_args: Vec<TermTy>, is_meta_: bool) -> LitTy {
        let is_meta = if base_name == "Metaclass" {
            // There is no `Meta:Metaclass`
            true
        } else {
            is_meta_
        };
        LitTy {
            base_name,
            type_args,
            is_meta,
        }
    }

    pub fn raw(base_name: &str) -> LitTy {
        LitTy::new(base_name.to_string(), vec![], false)
    }

    pub fn to_term_ty(&self) -> TermTy {
        ty::new(self.base_name.clone(), self.type_args.clone(), self.is_meta)
    }

    pub fn into_term_ty(self) -> TermTy {
        ty::new(self.base_name, self.type_args, self.is_meta)
    }

    pub fn meta_ty(&self) -> LitTy {
        debug_assert!(!self.is_meta);
        LitTy::new(self.base_name.clone(), self.type_args.clone(), true)
    }

    pub fn erasure(&self) -> Erasure {
        Erasure::new(self.base_name.clone(), self.is_meta)
    }

    pub fn substitute(&self, class_tyargs: &[TermTy], method_tyargs: &[TermTy]) -> LitTy {
        let args = self
            .type_args
            .iter()
            .map(|t| t.substitute(class_tyargs, method_tyargs))
            .collect();
        LitTy::new(self.base_name.clone(), args, self.is_meta)
    }

    /// Returns a serialized string which can be parsed by `parse_lit_ty`
    pub fn serialize(&self) -> String {
        let meta = if self.is_meta && self.base_name != "Metaclass" {
            "Meta:"
        } else {
            ""
        };
        let args = if self.type_args.is_empty() {
            "".to_string()
        } else {
            let s = self
                .type_args
                .iter()
                .map(|x| x.serialize())
                .collect::<Vec<_>>()
                .join(",");
            format!("<{}>", &s)
        };
        format!("{}{}{}", meta, self.base_name, args)
    }

    /// nom parser for LiTTy
    pub fn deserialize(s: &str) -> IResult<&str, LitTy> {
        // `Meta:` (optional)
        let (s, meta) = nom::multi::many_m_n(0, 1, tag("Meta:"))(s)?;
        let is_meta = !meta.is_empty();

        // `Foo::Bar`
        let (s, names) =
            nom::multi::separated_list1(tag("::"), nom::character::complete::alphanumeric1)(s)?;
        let base_name = names.join("::");

        // `<Int,String>`
        let parse_tys = nom::multi::separated_list1(tag(","), TermTy::deserialize);
        let (s, tyargs) =
            nom::combinator::opt(nom::sequence::delimited(tag("<"), parse_tys, tag(">")))(s)?;
        let type_args = tyargs.unwrap_or_default();

        Ok((s, LitTy::new(base_name.to_string(), type_args, is_meta)))
    }
}
