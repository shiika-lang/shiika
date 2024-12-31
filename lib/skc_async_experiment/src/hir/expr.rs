use crate::hir::FunctionName;
use crate::hir::{FunTy, Ty};
use crate::mir::expr::PseudoVar;
use anyhow::{anyhow, Result};

pub type Typed<T> = (T, Ty);
pub type TypedExpr = Typed<Expr>;

#[derive(Debug, Clone)]
pub enum Expr {
    Number(i64),
    PseudoVar(PseudoVar),
    LVarRef(String),
    ArgRef(usize, String), // (index, debug_name)
    FuncRef(FunctionName),
    FunCall(Box<Typed<Expr>>, Vec<Typed<Expr>>),
    If(Box<Typed<Expr>>, Box<Typed<Expr>>, Box<Typed<Expr>>),
    While(Box<Typed<Expr>>, Box<Typed<Expr>>),
    Spawn(Box<Typed<Expr>>),
    Alloc(String),
    Assign(String, Box<Typed<Expr>>),
    Return(Box<Typed<Expr>>),
    Exprs(Vec<Typed<Expr>>),
}

impl Expr {
    pub fn number(n: i64) -> TypedExpr {
        (Expr::Number(n), Ty::raw("Int"))
    }

    pub fn pseudo_var(var: PseudoVar) -> TypedExpr {
        let t = match var {
            PseudoVar::True | PseudoVar::False => Ty::raw("Bool"),
            PseudoVar::Void => Ty::raw("Void"),
        };
        (Expr::PseudoVar(var), t)
    }

    pub fn lvar_ref(name: impl Into<String>, ty: Ty) -> TypedExpr {
        (Expr::LVarRef(name.into()), ty)
    }

    pub fn arg_ref(idx: usize, name: impl Into<String>, ty: Ty) -> TypedExpr {
        (Expr::ArgRef(idx, name.into()), ty)
    }

    pub fn func_ref(name: FunctionName, fun_ty: FunTy) -> TypedExpr {
        (Expr::FuncRef(name), fun_ty.into())
    }

    pub fn fun_call(func: TypedExpr, args: Vec<TypedExpr>) -> TypedExpr {
        let result_ty = match &func.1 {
            Ty::Fun(f) => *f.ret_ty.clone(),
            _ => panic!("[BUG] not a function: {:?}", func),
        };
        (Expr::FunCall(Box::new(func), args), result_ty)
    }

    pub fn if_(cond: TypedExpr, then: TypedExpr, else_: TypedExpr) -> TypedExpr {
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

    pub fn while_(cond: TypedExpr, body: TypedExpr) -> TypedExpr {
        if cond.1 != Ty::raw("Bool") {
            panic!("[BUG] while cond not bool: {:?}", cond);
        }
        (Expr::While(Box::new(cond), Box::new(body)), Ty::raw("Void"))
    }

    pub fn spawn(e: TypedExpr) -> TypedExpr {
        (Expr::Spawn(Box::new(e)), Ty::raw("Void"))
    }

    pub fn alloc(name: impl Into<String>) -> TypedExpr {
        (Expr::Alloc(name.into()), Ty::raw("Void"))
    }

    pub fn assign(name: impl Into<String>, e: TypedExpr) -> TypedExpr {
        (Expr::Assign(name.into(), Box::new(e)), Ty::raw("Void"))
    }

    pub fn return_(e: TypedExpr) -> TypedExpr {
        (Expr::Return(Box::new(e)), Ty::raw("Never"))
    }

    pub fn exprs(mut exprs: Vec<TypedExpr>) -> TypedExpr {
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

pub fn into_exprs(expr: TypedExpr) -> Vec<TypedExpr> {
    match expr.0 {
        Expr::Exprs(exprs) => exprs,
        _ => vec![expr],
    }
}
