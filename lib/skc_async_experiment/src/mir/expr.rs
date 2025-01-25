use crate::mir::FunctionName;
use crate::mir::{FunTy, Ty};
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
    ConstRef(String),
    FuncRef(FunctionName),
    FunCall(Box<Typed<Expr>>, Vec<Typed<Expr>>),
    If(Box<Typed<Expr>>, Box<Typed<Expr>>, Box<Typed<Expr>>),
    While(Box<Typed<Expr>>, Box<Typed<Expr>>),
    Spawn(Box<Typed<Expr>>),
    Alloc(String),
    Assign(String, Box<Typed<Expr>>),
    ConstSet(String, Box<Typed<Expr>>),
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
    SelfRef,
    Void,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CastType {
    Upcast(Ty),
    AnyToFun(FunTy),
    AnyToInt,
    RawToAny,
    FunToAny,
}

impl CastType {
    pub fn result_ty(&self) -> Ty {
        match self {
            CastType::Upcast(ty) => ty.clone(),
            CastType::AnyToFun(x) => x.clone().into(),
            CastType::AnyToInt => Ty::raw("Int"),
            CastType::RawToAny | CastType::FunToAny => Ty::Any,
        }
    }
}

impl Expr {
    pub fn number(n: i64) -> TypedExpr {
        (Expr::Number(n), Ty::raw("Int"))
    }

    pub fn pseudo_var(var: PseudoVar, ty: Ty) -> TypedExpr {
        (Expr::PseudoVar(var), ty)
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
        (Expr::EnvSet(idx, Box::new(e), name.into()), Ty::raw("Void"))
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

    pub fn const_set(name: impl Into<String>, e: TypedExpr) -> TypedExpr {
        (Expr::ConstSet(name.into(), Box::new(e)), Ty::raw("Void"))
    }

    pub fn return_(e: TypedExpr) -> TypedExpr {
        (Expr::Return(Box::new(e)), Ty::raw("Never"))
    }

    pub fn exprs(mut exprs: Vec<TypedExpr>) -> TypedExpr {
        if exprs.is_empty() {
            exprs.push(Expr::pseudo_var(PseudoVar::Void, Ty::raw("Void")));
        }
        let t = exprs.last().unwrap().1.clone();
        (Expr::Exprs(exprs), t)
    }

    pub fn cast(cast_type: CastType, e: TypedExpr) -> TypedExpr {
        let ty = match &cast_type {
            CastType::Upcast(ty) => ty.clone(),
            CastType::AnyToFun(f) => f.clone().into(),
            CastType::AnyToInt => Ty::raw("Int"),
            CastType::RawToAny => Ty::Any,
            CastType::FunToAny => Ty::Any,
        };
        (Expr::Cast(cast_type, Box::new(e)), ty)
    }

    pub fn unbox(e: TypedExpr) -> TypedExpr {
        if e.1 != Ty::raw("Int") {
            panic!("[BUG] unbox non-Int: {:?}", e);
        }
        (Expr::Unbox(Box::new(e)), Ty::Int64)
    }

    pub fn raw_i64(n: i64) -> TypedExpr {
        (Expr::RawI64(n), Ty::Int64)
    }

    pub fn nop() -> TypedExpr {
        (Expr::Nop, Ty::raw("Void"))
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
        Expr::PseudoVar(PseudoVar::SelfRef) => "self".to_string(),
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
        Expr::ConstRef(name) => format!("{}", name),
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
            format!("if {}\n", cond.0.pretty_print(0, false))
                + then.0.pretty_print(lv + 1, true).as_str()
                + &format!("\n{}else\n", sp)
                + else_.0.pretty_print(lv + 1, true).as_str()
                + &format!("\n{}end", sp)
        }
        Expr::While(cond, body) => {
            format!("while {}\n", cond.0.pretty_print(0, false))
                + body.0.pretty_print(lv + 1, true).as_str()
                + &format!("\n{}end", sp)
        }
        Expr::Spawn(e) => format!("spawn {}", pretty_print(&e.0, lv, false)),
        Expr::Alloc(name) => format!("alloc {}", name),
        Expr::Assign(name, e) => format!("{} = {}", name, pretty_print(&e.0, lv, false)),
        Expr::ConstSet(name, e) => format!("{} = {}", name, pretty_print(&e.0, lv, false)),
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
