pub mod expr;
mod ty;
pub mod typing;
pub mod untyped;
pub mod visitor;
use crate::hir;
use crate::names::FunctionName;
pub use expr::{Expr, TypedExpr};
use shiika_core::names::ConstFullname;
use shiika_core::ty::TermTy;
use skc_hir::{MethodSignature, SkTypes};
use skc_mir::LibraryExports;
pub use ty::FunTy;

#[derive(Debug)]
pub struct CompilationUnit {
    pub package_name: Option<String>,
    pub imports: LibraryExports,
    pub imported_asyncs: Vec<FunctionName>,
    pub program: Program<TermTy>,
    pub sk_types: SkTypes,
}

#[derive(Debug)]
pub struct Program<T> {
    pub methods: Vec<Method<T>>,
    pub top_exprs: Vec<TypedExpr<T>>,
    // Note: order is important because forward references are allowed
    pub constants: Vec<(ConstFullname, TypedExpr<T>)>,
}

#[derive(Debug)]
pub struct Method<T> {
    pub sig: MethodSignature,
    pub body_stmts: TypedExpr<T>,
}

impl<T: Clone> Method<T> {
    pub fn name(&self) -> FunctionName {
        FunctionName::from_sig(&self.sig)
    }

    pub fn self_ty(&self) -> TermTy {
        self.sig.fullname.type_name.to_ty()
    }

    pub fn fun_ty(&self) -> FunTy {
        FunTy {
            asyncness: hir::Asyncness::Unknown,
            param_tys: self
                .sig
                .params
                .iter()
                .map(|x| x.ty.clone())
                .collect::<Vec<_>>(),
            ret_ty: self.sig.ret_ty.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asyncness {
    Unknown,
    Sync,
    Async,
    Lowered,
}

impl From<bool> for Asyncness {
    fn from(x: bool) -> Self {
        if x {
            Asyncness::Async
        } else {
            Asyncness::Sync
        }
    }
}

impl Asyncness {
    /// Returns true for Asyncness::Async. Panics if not applicable
    pub fn is_async(&self) -> bool {
        match self {
            Asyncness::Unknown => panic!("[BUG] asyncness is unknown"),
            Asyncness::Async => true,
            Asyncness::Sync => false,
            Asyncness::Lowered => panic!("[BUG] asyncness is lost"),
        }
    }

    pub fn is_sync(&self) -> bool {
        !self.is_async()
    }
}
