mod sk_class;
mod sk_module;
mod sk_type_base;
mod wtable;
use serde::{Deserialize, Serialize};
use shiika_core::names::*;
use shiika_core::ty::*;
pub use sk_class::SkClass;
pub use sk_module::SkModule;
pub use sk_type_base::SkTypeBase;
pub use wtable::WTable;


#[allow(clippy::large_enum_variant)]
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum SkType {
    Class(SkClass),
    Module(SkModule),
}

impl From<SkClass> for SkType {
    fn from(x: SkClass) -> Self {
        SkType::Class(x)
    }
}

impl From<SkModule> for SkType {
    fn from(x: SkModule) -> Self {
        SkType::Module(x)
    }
}

impl SkType {
    pub fn base(&self) -> &SkTypeBase {
        match self {
            SkType::Class(x) => &x.base,
            SkType::Module(x) => &x.base,
        }
    }

    pub fn base_mut(&mut self) -> &mut SkTypeBase {
        match self {
            SkType::Class(x) => &mut x.base,
            SkType::Module(x) => &mut x.base,
        }
    }

    pub fn is_class(&self) -> bool {
        matches!(&self, SkType::Class(_))
    }

    pub fn is_module(&self) -> bool {
        matches!(&self, SkType::Module(_))
    }

    pub fn class(&self) -> Option<&SkClass> {
        match self {
            SkType::Class(x) => Some(x),
            SkType::Module(_) => None,
        }
    }

    pub fn erasure(&self) -> &Erasure {
        &self.base().erasure
    }

    pub fn fullname(&self) -> TypeFullname {
        self.base().fullname()
    }

    // eg. TermTy(Array<T>), TermTy(Dict<K, V>)
    pub fn term_ty(&self) -> TermTy {
        self.base().term_ty()
    }

    pub fn const_is_obj(&self) -> bool {
        match self {
            SkType::Class(x) => x.const_is_obj,
            SkType::Module(_) => false,
        }
    }
}
