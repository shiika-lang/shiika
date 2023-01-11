use crate::type_inference::Id;
use shiika_core::ty;
use shiika_core::ty::{TermTy, TyBody, TyParamRef};
use std::fmt;

/// Type information that appears temporarily during type inference.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TmpTy {
    /// This type is unknown and should be inferred from other parts of the program.
    Unknown(Id),
    /// Mostly the same as `ty::LitTy` but may contain `Unknown` as a type argument.
    Literal {
        base_name: String,
        type_args: Vec<TmpTy>,
        /// `true` if values of this type are classes
        is_meta: bool,
    },
    /// Just wraps `ty::TyParamRef`
    TyParamRef(ty::TyParamRef),
}

impl fmt::Display for TmpTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl Default for TmpTy {
    /// Zero-value for TmpTy
    /// (We could use `Option` but I didn't want to `unwrap` everywhere)
    fn default() -> Self {
        TmpTy::Unknown(0)
    }
}

impl TmpTy {
    /// Make a TmpTy from a TermTy by replacing TyParamRef's with `Unknown`s.
    pub fn make(t: &TermTy, vars: &[(Id, TyParamRef)]) -> TmpTy {
        match &t.body {
            TyBody::TyRaw(lit_ty) => TmpTy::Literal {
                base_name: lit_ty.base_name.clone(),
                type_args: lit_ty
                    .type_args
                    .iter()
                    .map(|arg| Self::make(arg, vars))
                    .collect(),
                is_meta: lit_ty.is_meta,
            },
            TyBody::TyPara(tp_ref1) => {
                let found = vars.iter().find(|(_, tp_ref2)| *tp_ref1 == *tp_ref2);
                if let Some((id, _)) = found {
                    TmpTy::Unknown(*id)
                } else {
                    TmpTy::TyParamRef(tp_ref1.clone())
                }
            }
        }
    }

    /// Just convert a TermTy to TmpTy
    pub fn from(t: &TermTy) -> TmpTy {
        Self::make(t, Default::default())
    }

    /// Returns true if `Unknown(id)` appears in self
    pub fn contains(&self, id: Id) -> bool {
        match self {
            TmpTy::Unknown(id2) => id == *id2,
            TmpTy::Literal { type_args, .. } => type_args.iter().any(|t| t.contains(id)),
            TmpTy::TyParamRef(_) => false,
        }
    }

    /// Resolve `Unknown(id)` with `t`
    pub fn substitute(&self, id: &Id, t: &TmpTy) -> TmpTy {
        match self {
            TmpTy::Unknown(id2) => {
                if id == id2 {
                    t.clone()
                } else {
                    self.clone()
                }
            }
            TmpTy::Literal {
                base_name,
                type_args,
                is_meta,
            } => {
                let new_args = type_args.iter().map(|x| x.substitute(id, t)).collect();
                TmpTy::Literal {
                    base_name: base_name.clone(),
                    type_args: new_args,
                    is_meta: *is_meta,
                }
            }
            TmpTy::TyParamRef(_) => self.clone(),
        }
    }

    /// Returns human-readable representation
    pub fn to_string(&self) -> String {
        match self {
            TmpTy::Unknown(id) => format!("'{}", id),
            TmpTy::Literal {
                base_name,
                type_args,
                is_meta,
            } => {
                let m = if *is_meta && base_name != "Metaclass" {
                    "Meta:"
                } else {
                    ""
                };
                let args = if type_args.is_empty() {
                    "".to_string()
                } else {
                    "<".to_string()
                        + &type_args
                            .iter()
                            .map(|t| t.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                        + ">"
                };
                format!("{}{}{}", m, base_name, args)
            }
            TmpTy::TyParamRef(tp_ref) => tp_ref.name.clone(),
        }
    }

    pub fn type_args(&self) -> Option<&[TmpTy]> {
        match self {
            TmpTy::Literal { type_args, .. } => Some(type_args),
            _ => None,
        }
    }
}
