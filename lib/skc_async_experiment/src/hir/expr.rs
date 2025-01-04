use crate::hir::FunctionName;
use crate::hir::{FunTy, Ty};
use crate::mir::expr::PseudoVar;
use anyhow::{anyhow, Result};

pub type TypedExpr<T> = (Expr<T>, T);

#[derive(Debug, Clone)]
pub enum Expr<T> {
    Number(i64),
    PseudoVar(PseudoVar),
    LVarRef(String),
    ArgRef(usize, String), // (index, debug_name)
    FuncRef(FunctionName),
    FunCall(Box<TypedExpr<T>>, Vec<TypedExpr<T>>),
    If(Box<TypedExpr<T>>, Box<TypedExpr<T>>, Box<TypedExpr<T>>),
    While(Box<TypedExpr<T>>, Box<TypedExpr<T>>),
    Spawn(Box<TypedExpr<T>>),
    Alloc(String),
    Assign(String, Box<TypedExpr<T>>),
    Return(Box<TypedExpr<T>>),
    Exprs(Vec<TypedExpr<T>>),
}

impl Expr<Ty> {
    pub fn number(n: i64) -> TypedExpr<Ty> {
        (Expr::Number(n), Ty::raw("Int"))
    }

    pub fn pseudo_var(var: PseudoVar) -> TypedExpr<Ty> {
        let t = match var {
            PseudoVar::True | PseudoVar::False => Ty::raw("Bool"),
            PseudoVar::Void => Ty::raw("Void"),
        };
        (Expr::PseudoVar(var), t)
    }

    pub fn lvar_ref(name: impl Into<String>, ty: Ty) -> TypedExpr<Ty> {
        (Expr::LVarRef(name.into()), ty)
    }

    pub fn arg_ref(idx: usize, name: impl Into<String>, ty: Ty) -> TypedExpr<Ty> {
        (Expr::ArgRef(idx, name.into()), ty)
    }

    pub fn func_ref(name: FunctionName, fun_ty: FunTy) -> TypedExpr<Ty> {
        (Expr::FuncRef(name), fun_ty.into())
    }

    pub fn fun_call(func: TypedExpr<Ty>, args: Vec<TypedExpr<Ty>>) -> TypedExpr<Ty> {
        let result_ty = match &func.1 {
            Ty::Fun(f) => *f.ret_ty.clone(),
            _ => panic!("[BUG] not a function: {:?}", func),
        };
        (Expr::FunCall(Box::new(func), args), result_ty)
    }

    pub fn if_(cond: TypedExpr<Ty>, then: TypedExpr<Ty>, else_: TypedExpr<Ty>) -> TypedExpr<Ty> {
        let if_ty = Expr::if_ty(&then.1, &else_.1).unwrap();
        (
            Expr::If(Box::new(cond), Box::new(then), Box::new(else_)),
            if_ty,
        )
    }

    pub fn if_ty(then_ty: &Ty, else_ty: &Ty) -> Result<Ty> {
        let t1 = then_ty;
        let t2 = else_ty;
        let if_ty = if *t1 == Ty::raw("Never") {
            t2
        } else if *t2 == Ty::raw("Never") {
            t1
        } else if *t1 == Ty::raw("Void") {
            t2
        } else if *t2 == Ty::raw("Void") {
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

    pub fn while_(cond: TypedExpr<Ty>, body: TypedExpr<Ty>) -> TypedExpr<Ty> {
        if cond.1 != Ty::raw("Bool") {
            panic!("[BUG] while cond not bool: {:?}", cond);
        }
        (Expr::While(Box::new(cond), Box::new(body)), Ty::raw("Void"))
    }

    pub fn spawn(e: TypedExpr<Ty>) -> TypedExpr<Ty> {
        (Expr::Spawn(Box::new(e)), Ty::raw("Void"))
    }

    pub fn alloc(name: impl Into<String>) -> TypedExpr<Ty> {
        (Expr::Alloc(name.into()), Ty::raw("Void"))
    }

    pub fn assign(name: impl Into<String>, e: TypedExpr<Ty>) -> TypedExpr<Ty> {
        (Expr::Assign(name.into(), Box::new(e)), Ty::raw("Void"))
    }

    pub fn return_(e: TypedExpr<Ty>) -> TypedExpr<Ty> {
        (Expr::Return(Box::new(e)), Ty::raw("Never"))
    }

    pub fn exprs(mut exprs: Vec<TypedExpr<Ty>>) -> TypedExpr<Ty> {
        if exprs.is_empty() {
            exprs.push(Expr::pseudo_var(PseudoVar::Void));
        }
        let t = exprs.last().unwrap().1.clone();
        (Expr::Exprs(exprs), t)
    }

    pub fn is_async_fun_call(&self) -> bool {
        match self {
            Expr::FunCall(fexpr, _args) => fexpr.1.is_async_fun().unwrap(),
            _ => false,
        }
    }
}

pub fn into_exprs<Ty>(expr: TypedExpr<Ty>) -> Vec<TypedExpr<Ty>> {
    match expr.0 {
        Expr::Exprs(exprs) => exprs,
        _ => vec![expr],
    }
}
