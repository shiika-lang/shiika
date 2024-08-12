use crate::hir::{FunTy, Ty};

pub type Typed<T> = (T, Ty);
pub type TypedExpr = Typed<Expr>;

#[derive(Debug, Clone)]
pub enum Expr {
    Number(i64),
    PseudoVar(PseudoVar),
    LVarRef(String),
    ArgRef(usize),
    FuncRef(String),
    OpCall(String, Box<Typed<Expr>>, Box<Typed<Expr>>),
    FunCall(Box<Typed<Expr>>, Vec<Typed<Expr>>),
    If(Box<Typed<Expr>>, Vec<Typed<Expr>>, Vec<Typed<Expr>>),
    Yield(Box<Typed<Expr>>),
    While(Box<Typed<Expr>>, Vec<Typed<Expr>>),
    Spawn(Box<Typed<Expr>>),
    Alloc(String),
    Assign(String, Box<Typed<Expr>>),
    Return(Box<Typed<Expr>>),
    Cast(CastType, Box<Typed<Expr>>),
    Nop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PseudoVar {
    True,
    False,
    Null,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CastType {
    AnyToFun(FunTy),
    AnyToInt,
    NullToAny,
    IntToAny,
    FunToAny,
}

impl CastType {
    pub fn result_ty(&self) -> Ty {
        match self {
            CastType::AnyToFun(x) => x.clone().into(),
            CastType::AnyToInt => Ty::Int,
            CastType::NullToAny | CastType::IntToAny | CastType::FunToAny => Ty::Any,
        }
    }
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Number(n) => write!(f, "{}", n),
            Expr::PseudoVar(PseudoVar::True) => write!(f, "true"),
            Expr::PseudoVar(PseudoVar::False) => write!(f, "false"),
            Expr::PseudoVar(PseudoVar::Null) => write!(f, "null"),
            Expr::LVarRef(name) => write!(f, "{}", name),
            Expr::ArgRef(idx) => write!(f, "%arg_{}", idx),
            Expr::FuncRef(name) => write!(f, "{}", name),
            Expr::OpCall(op, lhs, rhs) => write!(f, "({} {} {})", lhs.0, op, rhs.0),
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
                for stmt in then {
                    write!(f, "    {}\n", stmt.0)?;
                }
                write!(f, "  }}")?;
                if !else_.is_empty() {
                    write!(f, " else {{\n")?;
                    for stmt in else_ {
                        write!(f, "    {}\n", stmt.0)?;
                    }
                    write!(f, "  }}")?;
                }
                Ok(())
            }
            Expr::Yield(e) => write!(f, "yield {}  # {}", e.0, e.1),
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
            Expr::Cast(cast_type, e) => write!(f, "({} as {})", e.0, cast_type.result_ty()),
            Expr::Nop => write!(f, "%nop"),
            //_ => todo!("{:?}", self),
        }
    }
}

impl Expr {
    pub fn number(n: i64) -> TypedExpr {
        (Expr::Number(n), Ty::Int)
    }

    pub fn lvar_ref(name: impl Into<String>, ty: Ty) -> TypedExpr {
        (Expr::LVarRef(name.into()), ty)
    }

    pub fn arg_ref(idx: usize, ty: Ty) -> TypedExpr {
        (Expr::ArgRef(idx), ty)
    }

    pub fn func_ref(name: impl Into<String>, fun_ty: FunTy) -> TypedExpr {
        (Expr::FuncRef(name.into()), fun_ty.into())
    }

    pub fn op_call(op_: impl Into<String>, lhs: TypedExpr, rhs: TypedExpr) -> TypedExpr {
        let op = op_.into();
        let ty = match &op[..] {
            "+" | "-" | "*" | "/" => Ty::Int,
            "<" | "<=" | ">" | ">=" | "==" | "!=" => Ty::Bool,
            _ => panic!("[BUG] unknown operator: {op}"),
        };
        (Expr::OpCall(op, Box::new(lhs), Box::new(rhs)), ty)
    }

    pub fn fun_call(func: TypedExpr, args: Vec<TypedExpr>) -> TypedExpr {
        let result_ty = match &func.1 {
            Ty::Fun(f) => *f.ret_ty.clone(),
            _ => panic!("[BUG] not a function: {:?}", func),
        };
        (Expr::FunCall(Box::new(func), args), result_ty)
    }

    pub fn if_(cond: TypedExpr, then: Vec<TypedExpr>, else_: Vec<TypedExpr>) -> TypedExpr {
        if cond.1 != Ty::Bool {
            panic!("[BUG] if cond not bool: {:?}", cond);
        }
        let t1 = yielded_ty(&then);
        let t2 = yielded_ty(&else_);
        let if_ty = if t1 == Ty::Void {
            t2
        } else if t2 == Ty::Void {
            t1
        } else if t1 == t2 {
            t1
        } else {
            panic!("[BUG] if types mismatch (t1: {:?}, t2: {:?})", t1, t2);
        };

        (Expr::If(Box::new(cond), then, else_), if_ty)
    }

    pub fn yield_(expr: TypedExpr) -> TypedExpr {
        let t = expr.1.clone();
        (Expr::Yield(Box::new(expr)), t)
    }

    pub fn yield_null() -> TypedExpr {
        let null = (Expr::PseudoVar(PseudoVar::Null), Ty::Null);
        (Expr::Yield(Box::new(null)), Ty::Null)
    }

    pub fn while_(cond: TypedExpr, body: Vec<TypedExpr>) -> TypedExpr {
        if cond.1 != Ty::Bool {
            panic!("[BUG] while cond not bool: {:?}", cond);
        }
        (Expr::While(Box::new(cond), body), Ty::Null)
    }

    pub fn spawn(e: TypedExpr) -> TypedExpr {
        (Expr::Spawn(Box::new(e)), Ty::Void)
    }

    pub fn alloc(name: impl Into<String>) -> TypedExpr {
        (Expr::Alloc(name.into()), Ty::Null)
    }

    pub fn assign(name: impl Into<String>, e: TypedExpr) -> TypedExpr {
        (Expr::Assign(name.into(), Box::new(e)), Ty::Void)
    }

    pub fn return_(e: TypedExpr) -> TypedExpr {
        (Expr::Return(Box::new(e)), Ty::Void)
    }

    pub fn cast(cast_type: CastType, e: TypedExpr) -> TypedExpr {
        let ty = match &cast_type {
            CastType::AnyToFun(f) => f.clone().into(),
            CastType::AnyToInt => Ty::Int,
            CastType::NullToAny => Ty::Any,
            CastType::IntToAny => Ty::Any,
            CastType::FunToAny => Ty::Any,
        };
        (Expr::Cast(cast_type, Box::new(e)), ty)
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

pub fn yielded_ty(stmts: &[TypedExpr]) -> Ty {
    let stmt = stmts.last().unwrap();
    match &stmt.0 {
        Expr::Yield(val) => val.1.clone(),
        Expr::Return(_) => Ty::Void,
        _ => panic!("[BUG] if branch not terminated with yield: {:?}", stmt),
    }
}
