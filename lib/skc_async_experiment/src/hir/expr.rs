use crate::hir::{FunTy, FunctionName};
use crate::mir::expr::PseudoVar;
use anyhow::{anyhow, Result};
use shiika_core::names::{ConstFullname, MethodFirstname};
use shiika_core::ty::{self, TermTy};

pub type TypedExpr<T> = (Expr<T>, T);

#[derive(Debug)]
pub enum Expr<T> {
    Number(i64),
    PseudoVar(PseudoVar),
    LVarRef(String),
    ArgRef(usize, String), // (index, debug_name)
    ConstRef(ConstFullname),
    FuncRef(FunctionName),
    FunCall(Box<TypedExpr<T>>, Vec<TypedExpr<T>>),
    If(Box<TypedExpr<T>>, Box<TypedExpr<T>>, Box<TypedExpr<T>>),
    MethodCall(Box<TypedExpr<T>>, MethodFirstname, Vec<TypedExpr<T>>),
    While(Box<TypedExpr<T>>, Box<TypedExpr<T>>),
    Spawn(Box<TypedExpr<T>>),
    Alloc(String),
    Assign(String, Box<TypedExpr<T>>),
    ConstSet(ConstFullname, Box<TypedExpr<T>>),
    Return(Box<TypedExpr<T>>),
    Exprs(Vec<TypedExpr<T>>),
    Upcast(Box<TypedExpr<T>>, T),
}

impl Expr<TermTy> {
    pub fn number(n: i64) -> TypedExpr<TermTy> {
        (Expr::Number(n), ty::raw("Int"))
    }

    pub fn pseudo_var(var: PseudoVar) -> TypedExpr<TermTy> {
        let t = match var {
            PseudoVar::True | PseudoVar::False => ty::raw("Bool"),
            PseudoVar::Void => ty::raw("Void"),
            PseudoVar::SelfRef => panic!("Use self_ref(ty) instead"),
        };
        (Expr::PseudoVar(var), t)
    }

    pub fn self_ref(ty: TermTy) -> TypedExpr<TermTy> {
        (Expr::PseudoVar(PseudoVar::SelfRef), ty)
    }

    pub fn lvar_ref(name: impl Into<String>, ty: TermTy) -> TypedExpr<TermTy> {
        (Expr::LVarRef(name.into()), ty)
    }

    pub fn arg_ref(idx: usize, name: impl Into<String>, ty: TermTy) -> TypedExpr<TermTy> {
        (Expr::ArgRef(idx, name.into()), ty)
    }

    pub fn const_ref(name: ConstFullname, ty: TermTy) -> TypedExpr<TermTy> {
        (Expr::ConstRef(name), ty)
    }

    pub fn func_ref(name: FunctionName, fun_ty: FunTy) -> TypedExpr<TermTy> {
        (Expr::FuncRef(name), fun_ty.to_term_ty())
    }

    pub fn fun_call(func: TypedExpr<TermTy>, args: Vec<TypedExpr<TermTy>>) -> TypedExpr<TermTy> {
        let result_ty = func.1.fn_x_info().unwrap().last().unwrap().clone();
        (Expr::FunCall(Box::new(func), args), result_ty)
    }

    pub fn if_(
        cond: TypedExpr<TermTy>,
        then: TypedExpr<TermTy>,
        else_: TypedExpr<TermTy>,
    ) -> TypedExpr<TermTy> {
        let if_ty = Expr::if_ty(&then.1, &else_.1).unwrap();
        (
            Expr::If(Box::new(cond), Box::new(then), Box::new(else_)),
            if_ty,
        )
    }

    pub fn if_ty(then_ty: &TermTy, else_ty: &TermTy) -> Result<TermTy> {
        let t1 = then_ty;
        let t2 = else_ty;
        let if_ty = if *t1 == ty::raw("Never") {
            t2
        } else if *t2 == ty::raw("Never") {
            t1
        } else if *t1 == ty::raw("Void") {
            t2
        } else if *t2 == ty::raw("Void") {
            t1
        } else if t1 != t2 {
            return Err(anyhow!(
                "then and else should have the same type but got {:?} and {:?}",
                t1,
                t2
            ));
        } else {
            t1
        };
        Ok(if_ty.clone())
    }

    pub fn while_(cond: TypedExpr<TermTy>, body: TypedExpr<TermTy>) -> TypedExpr<TermTy> {
        if cond.1 != ty::raw("Bool") {
            panic!("[BUG] while cond not bool: {:?}", cond);
        }
        (Expr::While(Box::new(cond), Box::new(body)), ty::raw("Void"))
    }

    pub fn spawn(e: TypedExpr<TermTy>) -> TypedExpr<TermTy> {
        (Expr::Spawn(Box::new(e)), ty::raw("Void"))
    }

    pub fn alloc(name: impl Into<String>) -> TypedExpr<TermTy> {
        (Expr::Alloc(name.into()), ty::raw("Void"))
    }

    pub fn assign(name: impl Into<String>, e: TypedExpr<TermTy>) -> TypedExpr<TermTy> {
        (Expr::Assign(name.into(), Box::new(e)), ty::raw("Void"))
    }

    pub fn const_set(name: ConstFullname, e: TypedExpr<TermTy>) -> TypedExpr<TermTy> {
        (Expr::ConstSet(name, Box::new(e)), ty::raw("Void"))
    }

    pub fn return_(e: TypedExpr<TermTy>) -> TypedExpr<TermTy> {
        (Expr::Return(Box::new(e)), ty::raw("Never"))
    }

    pub fn exprs(mut exprs: Vec<TypedExpr<TermTy>>) -> TypedExpr<TermTy> {
        if exprs.is_empty() {
            exprs.push(Expr::pseudo_var(PseudoVar::Void));
        }
        let t = exprs.last().unwrap().1.clone();
        (Expr::Exprs(exprs), t)
    }

    pub fn upcast(e: TypedExpr<TermTy>, ty: TermTy) -> TypedExpr<TermTy> {
        (Expr::Upcast(Box::new(e), ty.clone()), ty)
    }
}

pub fn into_vec<T>(expr: TypedExpr<T>) -> Vec<TypedExpr<T>> {
    match expr.0 {
        Expr::Exprs(exprs) => exprs,
        _ => vec![expr],
    }
}

pub fn from_vec<T: Clone>(exprs: Vec<TypedExpr<T>>) -> TypedExpr<T> {
    if exprs.is_empty() {
        panic!("[BUG] from_vec: empty");
    }
    let t = exprs.last().unwrap().1.clone();
    (Expr::Exprs(exprs), t)
}
