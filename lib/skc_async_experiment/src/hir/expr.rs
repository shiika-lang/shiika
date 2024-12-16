use crate::hir::FunctionName;
use crate::hir::{FunTy, Ty};
use anyhow::{anyhow, Result};

pub type Typed<T> = (T, Ty);
pub type TypedExpr = Typed<Expr>;

#[derive(Debug, Clone)]
pub enum Expr {
    Number(i64),
    PseudoVar(PseudoVar),
    LVarRef(String),
    ArgRef(usize, String),                   // (index, debug_name)
    EnvRef(usize, String),                   // (index, debug_name)
    EnvSet(usize, Box<Typed<Expr>>, String), // (index, value, debug_name)
    FuncRef(FunctionName),
    FunCall(Box<Typed<Expr>>, Vec<Typed<Expr>>),
    If(Box<Typed<Expr>>, Box<Typed<Expr>>, Box<Typed<Expr>>),
    While(Box<Typed<Expr>>, Vec<Typed<Expr>>),
    Spawn(Box<Typed<Expr>>),
    Alloc(String),
    Assign(String, Box<Typed<Expr>>),
    Return(Box<Typed<Expr>>),
    Exprs(Vec<Typed<Expr>>),
    Cast(CastType, Box<Typed<Expr>>),
    Unbox(Box<Typed<Expr>>),
    RawI64(i64),
    Nop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PseudoVar {
    True,
    False,
    Void,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CastType {
    AnyToFun(FunTy),
    AnyToInt,
    VoidToAny,
    IntToAny,
    FunToAny,
}

impl CastType {
    pub fn result_ty(&self) -> Ty {
        match self {
            CastType::AnyToFun(x) => x.clone().into(),
            CastType::AnyToInt => Ty::Int,
            CastType::VoidToAny | CastType::IntToAny | CastType::FunToAny => Ty::Any,
        }
    }
}

impl Expr {
    pub fn number(n: i64) -> TypedExpr {
        (Expr::Number(n), Ty::Int)
    }

    pub fn pseudo_var(var: PseudoVar) -> TypedExpr {
        let t = match var {
            PseudoVar::True | PseudoVar::False => Ty::Bool,
            PseudoVar::Void => Ty::Void,
        };
        (Expr::PseudoVar(var), t)
    }

    pub fn lvar_ref(name: impl Into<String>, ty: Ty) -> TypedExpr {
        (Expr::LVarRef(name.into()), ty)
    }

    pub fn arg_ref(idx: usize, name: impl Into<String>, ty: Ty) -> TypedExpr {
        (Expr::ArgRef(idx, name.into()), ty)
    }

    pub fn env_ref(idx: usize, name: impl Into<String>, ty: Ty) -> TypedExpr {
        (Expr::EnvRef(idx, name.into()), ty)
    }

    pub fn env_set(idx: usize, e: TypedExpr, name: impl Into<String>) -> TypedExpr {
        (Expr::EnvSet(idx, Box::new(e), name.into()), Ty::Void)
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
        let if_ty = if *t1 == Ty::Never {
            t2
        } else if *t2 == Ty::Never {
            t1
        } else if *t1 == Ty::Void {
            t2
        } else if *t2 == Ty::Void {
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

    pub fn while_(cond: TypedExpr, body: Vec<TypedExpr>) -> TypedExpr {
        if cond.1 != Ty::Bool {
            panic!("[BUG] while cond not bool: {:?}", cond);
        }
        (Expr::While(Box::new(cond), body), Ty::Void)
    }

    pub fn spawn(e: TypedExpr) -> TypedExpr {
        (Expr::Spawn(Box::new(e)), Ty::Void)
    }

    pub fn alloc(name: impl Into<String>) -> TypedExpr {
        (Expr::Alloc(name.into()), Ty::Void)
    }

    pub fn assign(name: impl Into<String>, e: TypedExpr) -> TypedExpr {
        (Expr::Assign(name.into(), Box::new(e)), Ty::Void)
    }

    pub fn return_(e: TypedExpr) -> TypedExpr {
        (Expr::Return(Box::new(e)), Ty::Never)
    }

    pub fn exprs(mut exprs: Vec<TypedExpr>) -> TypedExpr {
        if exprs.is_empty() {
            exprs.push(Expr::pseudo_var(PseudoVar::Void));
        }
        let t = exprs.last().unwrap().1.clone();
        (Expr::Exprs(exprs), t)
    }

    pub fn cast(cast_type: CastType, e: TypedExpr) -> TypedExpr {
        let ty = match &cast_type {
            CastType::AnyToFun(f) => f.clone().into(),
            CastType::AnyToInt => Ty::Int,
            CastType::VoidToAny => Ty::Any,
            CastType::IntToAny => Ty::Any,
            CastType::FunToAny => Ty::Any,
        };
        (Expr::Cast(cast_type, Box::new(e)), ty)
    }

    pub fn unbox(e: TypedExpr) -> TypedExpr {
        if e.1 != Ty::Int {
            panic!("[BUG] unbox non-Int: {:?}", e);
        }
        (Expr::Unbox(Box::new(e)), Ty::Int64)
    }

    pub fn raw_i64(n: i64) -> TypedExpr {
        (Expr::RawI64(n), Ty::Int64)
    }

    pub fn nop() -> TypedExpr {
        (Expr::Nop, Ty::Void)
    }

    pub fn is_async_fun_call(&self) -> bool {
        match self {
            Expr::FunCall(fexpr, _args) => fexpr.1.is_async_fun().unwrap(),
            _ => false,
        }
    }

    pub fn pretty_print(&self, lv: usize, as_stmt: bool) -> String {
        pretty_print(self, lv, as_stmt)
    }
}

pub fn into_exprs(expr: TypedExpr) -> Vec<TypedExpr> {
    match expr.0 {
        Expr::Exprs(exprs) => exprs,
        _ => vec![expr],
    }
}

fn pretty_print(node: &Expr, lv: usize, as_stmt: bool) -> String {
    let sp = "  ".repeat(lv);
    let mut indent = as_stmt;
    let s = match node {
        Expr::Number(n) => format!("{}", n),
        Expr::PseudoVar(PseudoVar::True) => "true".to_string(),
        Expr::PseudoVar(PseudoVar::False) => "false".to_string(),
        Expr::PseudoVar(PseudoVar::Void) => "Void".to_string(),
        Expr::LVarRef(name) => format!("{}", name),
        Expr::ArgRef(idx, name) => format!("{}@{}", name, idx),
        Expr::EnvRef(idx, name) => format!("{}%{}", name, idx),
        Expr::EnvSet(idx, e, name) => {
            format!(
                "env_set({}%{}, {})",
                name,
                idx,
                pretty_print(&e.0, lv, false)
            )
        }
        Expr::FuncRef(name) => format!("{}", name),
        Expr::FunCall(func, args) => {
            let Ty::Fun(fun_ty) = &func.1 else {
                panic!("[BUG] not a function: {:?}", func);
            };
            format!("{}{}(", func.0.pretty_print(0, false), fun_ty.asyncness)
                + args
                    .iter()
                    .map(|arg| arg.0.pretty_print(0, false))
                    .collect::<Vec<String>>()
                    .join(", ")
                    .as_str()
                + ")"
        }
        Expr::If(cond, then, else_) => {
            "if ".to_string()
                + cond.0.pretty_print(lv + 1, false).as_str()
                + "\n"
                + then.0.pretty_print(lv + 1, true).as_str()
                + &format!("\n{}else\n", sp)
                + else_.0.pretty_print(lv + 1, true).as_str()
                + &format!("\n{}end", sp)
        }
        Expr::While(cond, body) => {
            "while ".to_string()
                + cond.0.pretty_print(lv + 1, false).as_str()
                + "\n"
                + body
                    .iter()
                    .map(|stmt| stmt.0.pretty_print(lv + 1, true))
                    .collect::<Vec<String>>()
                    .join("\n")
                    .as_str()
        }
        Expr::Spawn(e) => format!("spawn {}", pretty_print(&e.0, lv, false)),
        Expr::Alloc(name) => format!("alloc {}", name),
        Expr::Assign(name, e) => format!("{} = {}", name, pretty_print(&e.0, lv, false)),
        Expr::Return(e) => format!("return {} # {}", pretty_print(&e.0, lv, false), e.1),
        Expr::Exprs(exprs) => {
            indent = false;
            exprs
                .iter()
                .map(|expr| format!("{}  #-> {}", pretty_print(&expr.0, lv, true), &expr.1))
                .collect::<Vec<String>>()
                .join("\n")
        }
        Expr::Cast(cast_type, e) => format!(
            "({} as {})",
            pretty_print(&e.0, lv, false),
            cast_type.result_ty()
        ),
        Expr::Unbox(e) => format!("unbox {}", pretty_print(&e.0, lv, false)),
        Expr::RawI64(n) => format!("{}", n),
        Expr::Nop => "%nop".to_string(),
        //_ => todo!("{:?}", self),
    };
    if indent {
        format!("{}{}", sp, s)
    } else {
        s
    }
}
