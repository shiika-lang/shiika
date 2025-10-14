use crate::mir::FunctionName;
use crate::mir::{Asyncness, FunTy, Ty};
use anyhow::{anyhow, Result};
use shiika_core::names::ModuleFullname;
use shiika_core::ty::TermTy;

pub type Typed<T> = (T, Ty);
pub type TypedExpr = Typed<Expr>;

#[derive(Debug, Clone)]
pub enum Expr {
    Number(i64),
    PseudoVar(PseudoVar),
    StringRef(String),
    LVarRef(String),
    IVarRef(Box<Typed<Expr>>, usize, String), // (obj, index, debug_name)
    ArgRef(usize, String),                    // (index, debug_name)
    EnvRef(usize, String),                    // (index, debug_name)
    EnvSet(usize, Box<Typed<Expr>>, String),  // (index, value, debug_name)
    ConstRef(String),
    FuncRef(FunctionName),
    FunCall(Box<Typed<Expr>>, Vec<Typed<Expr>>),
    VTableRef(Box<Typed<Expr>>, usize, String), // (receiver, index, debug_name)
    WTableRef(Box<Typed<Expr>>, ModuleFullname, usize, String), // (receiver, module, index, debug_name)
    If(Box<Typed<Expr>>, Box<Typed<Expr>>, Box<Typed<Expr>>),
    While(Box<Typed<Expr>>, Box<Typed<Expr>>),
    Spawn(Box<Typed<Expr>>),
    Alloc(String, Ty),
    LVarSet(String, Box<Typed<Expr>>),
    IVarSet(Box<Typed<Expr>>, usize, Box<Typed<Expr>>, String), // (obj, index, value, debug_name)
    ConstSet(String, Box<Typed<Expr>>),
    Return(Box<Typed<Expr>>),
    Exprs(Vec<Typed<Expr>>),
    Cast(CastType, Box<Typed<Expr>>),
    CreateObject(String),
    CreateTypeObject(TermTy, Box<Typed<Expr>>), // (ty, name_expr)
    // Unbox Shiika's Int to Rust's i64. Only used in `main()`
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
    Force(Ty), // Converted from old HirBitCast. Some of them may be Upcast
    Upcast(Ty),
    ToAny,       // Cast the value to llvm `i64`
    Recover(Ty), // Cast a `Any` value (llvm `i64`) to a specific type
}

impl CastType {
    pub fn result_ty(&self) -> Ty {
        match self {
            CastType::Force(ty) => ty.clone(),
            CastType::Upcast(ty) => ty.clone(),
            CastType::ToAny => Ty::Any,
            CastType::Recover(ty) => ty.clone(),
        }
    }
}

impl Expr {
    pub fn void_const_ref() -> TypedExpr {
        (Expr::ConstRef("Void".to_string()), Ty::raw("Void"))
    }

    // A Shiika number (boxed int)
    pub fn number(n: i64) -> TypedExpr {
        (Expr::Number(n), Ty::raw("Int"))
    }

    pub fn pseudo_var(var: PseudoVar, ty: Ty) -> TypedExpr {
        (Expr::PseudoVar(var), ty)
    }

    pub fn string_ref(s: impl Into<String>) -> TypedExpr {
        (Expr::StringRef(s.into()), Ty::Ptr)
    }

    pub fn lvar_ref(name: impl Into<String>, ty: Ty) -> TypedExpr {
        (Expr::LVarRef(name.into()), ty)
    }

    pub fn ivar_ref(obj_expr: TypedExpr, idx: usize, name: impl Into<String>, ty: Ty) -> TypedExpr {
        (Expr::IVarRef(Box::new(obj_expr), idx, name.into()), ty)
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

    pub fn const_ref(name: impl Into<String>, ty: Ty) -> TypedExpr {
        (Expr::ConstRef(name.into()), ty)
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

    pub fn vtable_ref(
        receiver: TypedExpr,
        idx: usize,
        name: impl Into<String>,
        fun_ty: FunTy,
    ) -> TypedExpr {
        (
            Expr::VTableRef(Box::new(receiver), idx, name.into()),
            fun_ty.into(),
        )
    }

    pub fn wtable_ref(
        receiver: TypedExpr,
        module: ModuleFullname,
        idx: usize,
        name: impl Into<String>,
        fun_ty: FunTy,
    ) -> TypedExpr {
        (
            Expr::WTableRef(Box::new(receiver), module, idx, name.into()),
            fun_ty.into(),
        )
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

    pub fn alloc(name: impl Into<String>, ty: Ty) -> TypedExpr {
        (Expr::Alloc(name.into(), ty), Ty::raw("Void"))
    }

    pub fn lvar_set(name: impl Into<String>, e: TypedExpr) -> TypedExpr {
        (Expr::LVarSet(name.into(), Box::new(e)), Ty::raw("Void"))
    }

    pub fn ivar_set(
        obj_expr: TypedExpr,
        idx: usize,
        e: TypedExpr,
        name: impl Into<String>,
    ) -> TypedExpr {
        (
            Expr::IVarSet(Box::new(obj_expr), idx, Box::new(e), name.into()),
            Ty::raw("Void"),
        )
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
        let ty = cast_type.result_ty();
        (Expr::Cast(cast_type, Box::new(e)), ty)
    }

    pub fn create_object(ty: TermTy) -> TypedExpr {
        (
            Expr::CreateObject(ty.fullname.to_class_fullname().0),
            ty.into(),
        )
    }

    pub fn create_type_object(ty: TermTy) -> TypedExpr {
        let type_name = &ty.fullname.0;
        let fun_call_expr = {
            let string_new = Expr::func_ref(
                FunctionName::method("Meta:String", "new"),
                FunTy {
                    asyncness: Asyncness::Unknown,
                    param_tys: vec![Ty::raw("Meta:String"), Ty::Ptr, Ty::Int64],
                    ret_ty: Box::new(Ty::raw("String")),
                },
            );
            let bytesize = type_name.len() as i64;
            Expr::fun_call(
                string_new,
                vec![
                    Expr::const_ref("::String", Ty::raw("Meta:String")),
                    Expr::string_ref(type_name),
                    Expr::raw_i64(bytesize),
                ],
            )
        };
        (
            Expr::CreateTypeObject(ty.clone(), Box::new(fun_call_expr)),
            ty.meta_ty().into(),
        )
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
        Expr::IVarRef(obj_expr, _, name) => {
            format!("{}.{}", obj_expr.0.pretty_print(0, false), name)
        }
        Expr::ArgRef(idx, name) => format!("{}^{}", name, idx),
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
        Expr::VTableRef(receiver, idx, name) => {
            format!(
                "%VTableRef({}, {}, {})",
                receiver.0.pretty_print(0, false),
                idx,
                name
            )
        }
        Expr::WTableRef(receiver, module, idx, name) => {
            format!(
                "%WTableRef({}, {}, {}, {})",
                receiver.0.pretty_print(0, false),
                module,
                idx,
                name
            )
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
        Expr::Alloc(name, ty) => format!("alloc {}: {}", name, ty),
        Expr::LVarSet(name, e) => format!("{} = {}", name, pretty_print(&e.0, lv, false)),
        Expr::IVarSet(obj_expr, _idx, e, name) => {
            format!(
                "{}.{} = {}",
                obj_expr.0.pretty_print(0, false),
                name,
                pretty_print(&e.0, lv, false)
            )
        }
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
        Expr::Cast(cast_type, e) => {
            let expr = pretty_print(&e.0, lv, false);
            match cast_type {
                CastType::Force(_) => format!("%Force({}, {})", expr, cast_type.result_ty()),
                CastType::Upcast(_) => format!("%Upcast({}, {})", expr, cast_type.result_ty()),
                CastType::ToAny => format!("%ToAny({}, {})", &e.1, expr),
                CastType::Recover(_) => format!("%Recover({}, {})", expr, cast_type.result_ty()),
            }
        }
        Expr::CreateObject(name) => format!("%CreateObject('{}')", name),
        Expr::CreateTypeObject(_, name) => {
            format!("%CreateTypeObject({})", pretty_print(&name.0, lv, false))
        }
        Expr::Unbox(e) => format!("%Unbox({})", pretty_print(&e.0, lv, false)),
        Expr::RawI64(n) => format!("{}", n),
        Expr::Nop => "%Nop".to_string(),
        Expr::StringRef(s) => format!("\"{}\"", s),
        //_ => todo!("{:?}", self),
    };
    if indent {
        format!("{}{}", sp, s)
    } else {
        s
    }
}
