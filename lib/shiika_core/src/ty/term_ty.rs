use crate::names::*;
use crate::ty;
use crate::ty::erasure::Erasure;
use crate::ty::lit_ty::LitTy;
use crate::ty::typaram_ref::{TyParamKind, TyParamRef};
use nom::IResult;
use serde::{de, ser};
use std::fmt;

/// Types for a term (a Shiika value).
#[derive(PartialEq, Eq, Clone)]
pub struct TermTy {
    pub fullname: TypeFullname,
    pub body: TyBody,
}

impl AsRef<TermTy> for TermTy {
    fn as_ref(&self) -> &TermTy {
        self
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TyBody {
    /// Types of classes
    /// eg. "Int", "Meta:String", "Array<Int>", "Meta:Pair<Bool, Object>", etc.
    TyRaw(LitTy),
    /// Type parameter reference eg. `T`
    TyPara(TyParamRef),
}
use TyBody::*;

impl std::fmt::Display for TermTy {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.fullname)
    }
}

impl std::fmt::Debug for TermTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TermTy({})", &self.dbg_str())
    }
}

/// Format `type_args` with .dbg_str
fn _dbg_type_args(type_args: &[TermTy]) -> String {
    if type_args.is_empty() {
        "".to_string()
    } else {
        let s = type_args
            .iter()
            .map(|x| x.dbg_str())
            .collect::<Vec<_>>()
            .join(", ");
        format!("<{}>", &s)
    }
}

impl TermTy {
    pub fn upper_bound(&self) -> LitTy {
        match &self.body {
            TyRaw(t) => t.clone(),
            TyPara(TyParamRef {
                upper_bound,
                as_class,
                ..
            }) => {
                if *as_class {
                    upper_bound.meta_ty()
                } else {
                    upper_bound.clone()
                }
            }
        }
    }

    /// Return string to inspect `self`
    fn dbg_str(&self) -> String {
        match &self.body {
            TyRaw(LitTy {
                base_name,
                type_args,
                is_meta,
            }) => {
                let meta = if *is_meta && base_name != "Metaclass" {
                    "Meta:"
                } else {
                    ""
                };
                format!("{}{}{}", meta, base_name, _dbg_type_args(type_args))
                // TODO: Use colors?
                // "\x1b[32m{}<\x1b[0m{}\x1b[32m>\x1b[0m"
            }
            TyPara(typaram_ref) => typaram_ref.dbg_str(),
        }
    }

    /// Returns if value of this type is class
    pub fn is_metaclass(&self) -> bool {
        match &self.body {
            TyRaw(LitTy {
                base_name, is_meta, ..
            }) => *is_meta || base_name == "Metaclass",
            _ => false,
        }
    }

    /// Returns if this is TyParamRef
    pub fn is_typaram_ref(&self) -> bool {
        matches!(&self.body, TyPara(_))
    }

    // Returns true when this is the Void type
    pub fn is_void_type(&self) -> bool {
        match self.body {
            TyRaw(_) => self.fullname.0 == "Void",
            _ => false,
        }
    }

    // Returns true when this is the Never type
    pub fn is_never_type(&self) -> bool {
        match self.body {
            TyRaw(_) => self.fullname.0 == "Never",
            _ => false,
        }
    }

    // If this is any of Fn0, .., Fn9, returns its type arguments.
    pub fn fn_x_info(&self) -> Option<&[TermTy]> {
        match &self.body {
            TyRaw(LitTy {
                base_name,
                type_args,
                is_meta,
            }) => {
                if *is_meta {
                    return None;
                }
                for i in 0..=9 {
                    if *base_name == format!("Fn{}", i) {
                        return Some(type_args);
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub fn meta_ty(&self) -> TermTy {
        match &self.body {
            TyRaw(LitTy {
                base_name,
                type_args,
                ..
            }) => {
                if self.is_metaclass() {
                    ty::raw("Metaclass")
                } else {
                    ty::spe_meta(base_name, type_args.clone())
                }
            }
            TyPara(typaram_ref) => {
                debug_assert!(!typaram_ref.as_class);
                typaram_ref.as_class().into()
            }
        }
    }

    pub fn instance_ty(&self) -> TermTy {
        match &self.body {
            TyRaw(LitTy {
                base_name,
                type_args,
                is_meta,
            }) => {
                debug_assert!(is_meta);
                ty::spe(base_name, type_args.to_vec())
            }
            TyPara(_) => self.clone(),
        }
    }

    pub fn has_type_args(&self) -> bool {
        match &self.body {
            TyRaw(LitTy { type_args, .. }) => !type_args.is_empty(),
            _ => false,
        }
    }

    pub fn type_args(&self) -> &[TermTy] {
        match &self.body {
            TyRaw(LitTy { type_args, .. }) => type_args,
            _ => &[],
        }
    }

    pub fn as_type_argument(&self) -> TermTy {
        match &self.body {
            TyRaw(LitTy {
                base_name,
                type_args,
                is_meta,
            }) => {
                debug_assert!(is_meta);
                ty::spe(base_name, type_args.to_vec())
            }
            TyPara(_) => self.clone(),
            //TyPara(typaram_ref) => typaram_ref.as_class().into_term_ty(),
        }
    }

    pub fn specialized_ty(&self, tyargs: Vec<TermTy>) -> TermTy {
        match &self.body {
            TyRaw(LitTy {
                base_name, is_meta, ..
            }) => ty::new(base_name, tyargs, *is_meta),
            _ => panic!("unexpected"),
        }
    }

    /// Return "A" for "A<B>", "Meta:A" for "Meta:A<B>"
    pub fn base_type_name(&self) -> TypeFullname {
        match &self.body {
            TyRaw(LitTy {
                base_name, is_meta, ..
            }) => TypeFullname::new(base_name, *is_meta),
            _ => panic!("unexpected"),
        }
    }

    // obsolete
    pub fn base_class_name(&self) -> ClassFullname {
        self.base_type_name().to_class_fullname()
    }

    /// Return true if two types are identical
    pub fn equals_to(&self, other: &TermTy) -> bool {
        self == other
    }

    /// Return true when two types are the same if type args are removed
    pub fn same_base(&self, other: &TermTy) -> bool {
        self.erasure() == other.erasure()
    }

    pub fn erasure(&self) -> Erasure {
        match &self.body {
            TyRaw(LitTy {
                base_name, is_meta, ..
            }) => Erasure::new(base_name.clone(), *is_meta),
            _ => todo!(),
        }
    }

    pub fn erasure_ty(&self) -> TermTy {
        match &self.body {
            TyRaw(LitTy {
                base_name, is_meta, ..
            }) => ty::new(base_name, Default::default(), *is_meta),
            _ => todo!(),
        }
    }

    /// Returns type arguments, if any
    pub fn tyargs(&self) -> &[TermTy] {
        match &self.body {
            TyRaw(LitTy { type_args, .. }) => type_args,
            _ => &[],
        }
    }

    /// Apply type argments into type parameters
    /// - class_tyargs: None if the class is not generic (non-generic class
    ///   can have a generic method)
    /// - method_tyargs: None if not in a method context (eg. when creating
    ///   `Array<Int>` from `Array<T>`)
    pub fn substitute(&self, class_tyargs: &[TermTy], method_tyargs: &[TermTy]) -> TermTy {
        match &self.body {
            TyPara(TyParamRef { kind, idx, .. }) => match kind {
                TyParamKind::Class => {
                    if class_tyargs.is_empty() {
                        self.clone()
                    } else {
                        class_tyargs[*idx].clone()
                    }
                }
                TyParamKind::Method => {
                    if method_tyargs.is_empty() {
                        self.clone()
                    } else {
                        method_tyargs[*idx].clone()
                    }
                }
            },
            TyRaw(lit_ty) => lit_ty.substitute(class_tyargs, method_tyargs).into(),
        }
    }

    /// Name for vtable when invoking a method on an object of this type
    pub fn vtable_name(&self) -> ClassFullname {
        match &self.body {
            TyRaw(LitTy {
                base_name, is_meta, ..
            }) => ClassFullname::new(base_name, *is_meta),
            _ => self.fullname.to_class_fullname(),
        }
    }

    pub fn is_specialized(&self) -> bool {
        match &self.body {
            TyRaw(LitTy { type_args, .. }) => !type_args.is_empty(),
            _ => false,
        }
    }

    pub fn contains_typaram_ref(&self) -> bool {
        match &self.body {
            TyPara(_) => true,
            TyRaw(LitTy { type_args, .. }) => type_args.iter().any(|t| t.contains_typaram_ref()),
        }
    }

    /// Returns a serialized string which can be parsed by `deserialize`
    pub fn serialize(&self) -> String {
        match &self.body {
            TyRaw(x) => x.serialize(),
            TyPara(x) => x.serialize(),
        }
    }

    /// nom parser for TermTy
    pub fn deserialize(s: &str) -> IResult<&str, TermTy> {
        if let Ok((s, t)) = TyParamRef::deserialize(s) {
            Ok((s, t.to_term_ty()))
        } else {
            let (s, t) = LitTy::deserialize(s)?;
            Ok((s, t.to_term_ty()))
        }
    }
}

//
// serde - simplify JSON representation
//
impl ser::Serialize for TermTy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.serialize())
    }
}

struct TermTyVisitor;
impl<'de> de::Visitor<'de> for TermTyVisitor {
    type Value = TermTy;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        formatter.write_str("a TermTy")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match TermTy::deserialize(v) {
            Ok((s, ty)) => {
                if s.is_empty() {
                    Ok(ty)
                } else {
                    Err(serde::de::Error::custom(format!(
                        "tried to parse `{}' as TermTy but `{}' is left after parsing",
                        v, s
                    )))
                }
            }
            Err(_) => Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &self,
            )),
        }
    }
}

impl<'de> de::Deserialize<'de> for TermTy {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as de::Deserializer<'de>>::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_identifier(TermTyVisitor)
    }
}
