use crate::hir::FunctionName;
use crate::hir::{FunTy, Ty};

pub type Typed<T> = (T, Ty);
pub type TypedExpr = Typed<Expr>;

#[derive(Debug, Clone)]
pub enum Expr {
    Number(i64),
    PseudoVar(PseudoVar),
    LVarRef(String),
    ArgRef(usize),
    FuncRef(FunctionName),
    FunCall(Box<Typed<Expr>>, Vec<Typed<Expr>>),
    If(Box<Typed<Expr>>, Box<Typed<Expr>>, Option<Box<Typed<Expr>>>),
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

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Number(n) => write!(f, "{}", n),
            Expr::PseudoVar(PseudoVar::True) => write!(f, "true"),
            Expr::PseudoVar(PseudoVar::False) => write!(f, "false"),
            Expr::PseudoVar(PseudoVar::Void) => write!(f, "null"),
            Expr::LVarRef(name) => write!(f, "{}", name),
            Expr::ArgRef(idx) => write!(f, "%arg_{}", idx),
            Expr::FuncRef(name) => write!(f, "{}", name),
            Expr::FunCall(func, args) => {
                let Ty::Fun(fun_ty) = &func.1 else {
                    panic!("[BUG] not a function: {:?}", func);
                };
                write!(f, "{}{}(", func.0, fun_ty.asyncness)?;
                for (i, arg) in args.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg.0)?;
                }
                write!(f, ")")
            }
            Expr::If(cond, then, else_) => {
                write!(f, "if ({}) {{\n", cond.0)?;
                write!(f, "{}\n", then.0)?;
                write!(f, "  }}")?;
                if let Some(else_) = else_ {
                    write!(f, " else {{\n")?;
                    write!(f, "    {}\n", else_.0)?;
                    write!(f, "  }}")?;
                }
                Ok(())
            }
            Expr::While(cond, body) => {
                write!(f, "while {} {{\n", cond.0)?;
                for stmt in body {
                    write!(f, "  {}\n", stmt.0)?;
                }
                write!(f, "}}")
            }
            Expr::Spawn(e) => write!(f, "spawn {}", e.0),
            Expr::Alloc(name) => write!(f, "alloc {}", name),
            Expr::Assign(name, e) => write!(f, "{} = {}", name, e.0),
            Expr::Return(e) => write!(f, "return {}  # {}", e.0, e.1),
            Expr::Exprs(exprs) => {
                for expr in exprs {
                    write!(f, "    {}\n", expr.0)?;
                }
                Ok(())
            }
            Expr::Cast(cast_type, e) => write!(f, "({} as {})", e.0, cast_type.result_ty()),
            Expr::Unbox(e) => write!(f, "unbox {}", e.0),
            Expr::RawI64(n) => write!(f, "{}.raw", n),
            Expr::Nop => write!(f, "%nop"),
            //_ => todo!("{:?}", self),
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

    pub fn arg_ref(idx: usize, ty: Ty) -> TypedExpr {
        (Expr::ArgRef(idx), ty)
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

    pub fn if_(cond: TypedExpr, then: TypedExpr, else_: Option<TypedExpr>) -> TypedExpr {
        if cond.1 != Ty::Bool {
            panic!("[BUG] if cond not bool: {:?}", cond);
        }
        let t1 = &then.1;
        let t2 = match &else_ {
            Some(e) => e.1.clone(),
            None => Ty::Void,
        };
        let if_ty = if *t1 == Ty::Void {
            t2.clone()
        } else if t2 == Ty::Void {
            t1.clone()
        } else if *t1 == t2 {
            t1.clone()
        } else {
            panic!("[BUG] if types mismatch (t1: {:?}, t2: {:?})", t1, t2);
        };

        (
            Expr::If(Box::new(cond), Box::new(then), else_.map(Box::new)),
            if_ty,
        )
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

    pub fn exprs(exprs: Vec<TypedExpr>) -> TypedExpr {
        debug_assert!(!exprs.is_empty());
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
            Expr::FunCall(fexpr, _args) => fexpr.1.is_async_fun(),
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
