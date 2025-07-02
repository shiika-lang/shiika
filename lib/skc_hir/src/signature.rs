use nom::{bytes::complete::tag, IResult};
use serde::{de, ser};
use shiika_core::{names::*, ty, ty::*};
use std::fmt;

/// Information of a method except its body exprs.
/// Note that `params` may contain some HIR when it has default expr.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MethodSignature {
    pub fullname: MethodFullname,
    pub ret_ty: TermTy,
    pub params: Vec<MethodParam>,
    pub typarams: Vec<TyParam>,
    pub asyncness: Asyncness,
    /// True if this method is inheritable (i.e. belongs to non-final class) or overrides
    /// ancestor's. `None` if not known yet.
    pub polymorphic: Option<bool>,
}

impl fmt::Display for MethodSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.full_string())
    }
}

impl MethodSignature {
    pub fn receiver_ty(&self) -> TermTy {
        self.fullname.type_name.to_ty()
    }

    pub fn param_tys(&self) -> Vec<TermTy> {
        self.params.iter().map(|p| p.ty.clone()).collect()
    }

    pub fn has_default_expr(&self) -> bool {
        self.params.iter().any(|p| p.has_default)
    }

    pub fn has_typarams(&self) -> bool {
        !self.typarams.is_empty()
    }

    pub fn is_class_method(&self) -> bool {
        self.fullname.type_name.is_meta()
    }

    /// Returns if this is `Class#new` or a method which overrides it.
    pub fn is_the_new(&self) -> bool {
        self.fullname.type_name.is_meta() && self.fullname.first_name.0 == "new"
    }

    pub fn first_name(&self) -> &MethodFirstname {
        &self.fullname.first_name
    }

    /// If this method takes a block, returns types of block params and block value.
    pub fn block_ty(&self) -> Option<&[TermTy]> {
        self.params.last().and_then(|param| param.ty.fn_x_info())
    }

    /// Substitute type parameters with type arguments
    pub fn specialize(&self, class_tyargs: &[TermTy], method_tyargs: &[TermTy]) -> MethodSignature {
        MethodSignature {
            fullname: self.fullname.clone(),
            ret_ty: self.ret_ty.substitute(class_tyargs, method_tyargs),
            params: self
                .params
                .iter()
                .map(|param| param.substitute(class_tyargs, method_tyargs))
                .collect(),
            typarams: self.typarams.clone(), // eg. Array<T>#map<U>(f: Fn1<T, U>) -> Array<Int>#map<U>(f: Fn1<Int, U>)
            asyncness: self.asyncness.clone(),
            polymorphic: self.polymorphic,
        }
    }

    /// Returns true if `self` is the same as `other` except the
    /// parameter names.
    pub fn equivalent_to(&self, other: &MethodSignature) -> bool {
        if self.fullname.first_name != other.fullname.first_name {
            return false;
        }
        if !self.ret_ty.equals_to(&other.ret_ty) {
            return false;
        }
        if self.params.len() != other.params.len() {
            return false;
        }
        for i in 0..self.params.len() {
            if self.params[i].ty != other.params[i].ty {
                return false;
            }
        }
        if self.typarams != other.typarams {
            return false;
        }
        true
    }

    pub fn full_string(&self) -> String {
        let typarams = if self.typarams.is_empty() {
            "".to_string()
        } else {
            "<".to_string()
                + &self
                    .typarams
                    .iter()
                    .map(|x| x.name.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
                + ">"
        };
        let params = self
            .params
            .iter()
            .map(|x| format!("{}: {}", &x.name, &x.ty))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{}{}({}) -> {}",
            &self.fullname, typarams, params, &self.ret_ty
        )
    }

    /// Returns a serialized string which can be parsed by `deserialize`
    pub fn serialize(&self) -> String {
        let polymorphic = match self.polymorphic {
            Some(true) => "polymorphic ",
            Some(false) => "",
            None => panic!("MethodSignature::serialize: polymorphic is None"),
        };
        let typarams = self
            .typarams
            .iter()
            .map(TyParam::serialize)
            .collect::<Vec<_>>()
            .join(",");
        let params = self
            .params
            .iter()
            .map(MethodParam::serialize)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{}{}<{}>{}({}){}",
            polymorphic,
            self.asyncness,
            typarams,
            &self.fullname,
            params,
            &self.ret_ty.serialize()
        )
    }

    /// nom parser for MethodSignature
    pub fn deserialize(s: &str) -> IResult<&str, MethodSignature> {
        let (s, polymorphic) = nom::combinator::opt(tag("polymorphic "))(s)?;
        let (s, asyncness) = Asyncness::deserialize(s)?;
        let parse_typarams = nom::multi::separated_list0(tag(","), TyParam::deserialize);
        let (s, typarams) = nom::sequence::delimited(tag("<"), parse_typarams, tag(">"))(s)?;

        let get_method = nom::bytes::complete::take_until("(");
        let (s, fullname) = nom::combinator::map_opt(get_method, MethodFullname::parse)(s)?;

        let parse_params = nom::multi::separated_list0(tag(","), MethodParam::deserialize);
        let (s, params) = nom::sequence::delimited(tag("("), parse_params, tag(")"))(s)?;
        let (s, ret_ty) = TermTy::deserialize(s)?;
        Ok((
            s,
            MethodSignature {
                fullname,
                ret_ty,
                params,
                typarams,
                asyncness,
                polymorphic: Some(polymorphic.is_some()),
            },
        ))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MethodParam {
    pub name: String,
    pub ty: TermTy,
    pub has_default: bool,
}

impl MethodParam {
    pub fn substitute(&self, class_tyargs: &[TermTy], method_tyargs: &[TermTy]) -> MethodParam {
        MethodParam {
            name: self.name.clone(),
            ty: self.ty.substitute(class_tyargs, method_tyargs),
            has_default: self.has_default,
        }
    }

    /// Returns a serialized string which can be parsed by `deserialize`
    pub fn serialize(&self) -> String {
        let d = if self.has_default { "=" } else { "" };
        format!("{}:{}{}", &self.name, &self.ty.serialize(), d)
    }

    /// nom parser for MethodParam
    pub fn deserialize(s: &str) -> IResult<&str, MethodParam> {
        let get_param_name_part = nom::multi::many1(nom::branch::alt((
            tag("_"),
            nom::character::complete::alphanumeric1,
        )));
        let (s, name) = nom::combinator::recognize(nom::sequence::preceded(
            nom::combinator::opt(tag("@")),
            get_param_name_part,
        ))(s)?;
        let (s, _) = tag(":")(s)?;
        let (s, ty) = TermTy::deserialize(s)?;
        let (s, e) = nom::combinator::opt(tag("="))(s)?;
        let has_default = e.is_some();
        Ok((
            s,
            MethodParam {
                name: name.to_string(),
                ty,
                has_default,
            },
        ))
    }
}

/// Return a param of the given name and its index
pub fn find_param<'a>(params: &'a [MethodParam], name: &str) -> Option<(usize, &'a MethodParam)> {
    params
        .iter()
        .enumerate()
        .find(|(_, param)| param.name == name)
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Asyncness {
    Sync,
    Async,
    Unknown,
}

impl fmt::Display for Asyncness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Asyncness::Sync => "Sync",
                Asyncness::Async => "Async",
                Asyncness::Unknown => "Unknown",
            }
        )
    }
}

impl Asyncness {
    pub fn serialize(&self) -> String {
        self.to_string()
    }

    pub fn deserialize(s: &str) -> IResult<&str, Asyncness> {
        let (s, asyncness) = nom::branch::alt((
            tag("Unknown"),
            nom::branch::alt((tag("Async"), tag("Sync"))),
        ))(s)?;
        let asyncness = match &asyncness[..] {
            "Async" => Asyncness::Async,
            "Sync" => Asyncness::Sync,
            "Unknown" => Asyncness::Unknown,
            _ => unreachable!(),
        };
        Ok((s, asyncness))
    }
}

/// Create a signature of a `new` method
/// eg. Given this Pair#initialize,
///     def initialize(@fst: A, @snd: B)
///   returns
///   Meta:Pair.new<A, B>(@fst: A, @snd: B) -> Pair<A, B>
pub fn signature_of_new(
    // eg. `Meta:Pair`
    metaclass_fullname: &ClassFullname,
    initialize_params: Vec<MethodParam>,
    typarams: Vec<ty::TyParam>,
) -> MethodSignature {
    let method_typaram_refs = ty::typarams_to_typaram_refs(&typarams, TyParamKind::Method, false)
        .into_iter()
        .map(|x| x.into_term_ty())
        .collect::<Vec<_>>();

    // Replace references of class typarams with method typarams
    let params = initialize_params
        .iter()
        .map(|param| param.substitute(&method_typaram_refs, Default::default()))
        .collect::<Vec<_>>();

    let instance_ty_base = metaclass_fullname.to_ty().instance_ty();
    let ret_ty = instance_ty_base.specialized_ty(method_typaram_refs);

    MethodSignature {
        fullname: method_fullname(metaclass_fullname.clone().into(), "new"),
        ret_ty,
        params,
        typarams,
        asyncness: Asyncness::Unknown,
        polymorphic: None,
    }
}

/// Create a signature of a `initialize` method
pub fn signature_of_initialize(
    class_fullname: &ClassFullname,
    params: Vec<MethodParam>,
) -> MethodSignature {
    MethodSignature {
        fullname: method_fullname(class_fullname.clone().into(), "initialize"),
        ret_ty: ty::raw("Void"),
        params,
        typarams: vec![],
        asyncness: Asyncness::Unknown,
        polymorphic: None,
    }
}

//
// serde - simplify JSON representation
//
impl ser::Serialize for MethodSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.serialize())
    }
}

struct MethodSignatureVisitor;
impl<'de> de::Visitor<'de> for MethodSignatureVisitor {
    type Value = MethodSignature;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        formatter.write_str("a MethodSignature")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match MethodSignature::deserialize(v) {
            Ok((s, n)) => {
                if s.is_empty() {
                    Ok(n)
                } else {
                    Err(serde::de::Error::custom(format!(
                        "tried to parse `{}' as MethodSignature but `{}' is left after parsing",
                        v, s
                    )))
                }
            }
            Err(e) => Err(serde::de::Error::custom(format!(
                "failed to parse MethodSignature ({}): {}",
                v, e
            ))),
        }
    }
}

impl<'de> de::Deserialize<'de> for MethodSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as de::Deserializer<'de>>::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_identifier(MethodSignatureVisitor)
    }
}
