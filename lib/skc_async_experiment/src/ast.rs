pub type Program<'a> = Vec<Declaration>;

#[derive(PartialEq, Debug, Clone)]
pub enum Declaration {
    Extern(Extern),
    Function(Function),
}

#[derive(PartialEq, Debug, Clone)]
pub struct Extern {
    // Denotes the rust-implemented function returns Future
    pub is_async: bool,
    // Used in prelude.rs
    pub is_internal: bool,
    pub name: String,
    pub params: Vec<Param>,
    pub ret_ty: Ty,
}

#[derive(PartialEq, Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub ret_ty: Ty,
    pub body_stmts: Vec<Expr>,
}

#[derive(PartialEq, Debug, Clone)]
pub struct Param {
    pub ty: Ty,
    pub name: String,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Ty {
    Raw(String),
    Fun(FunTy),
}

#[derive(PartialEq, Debug, Clone)]
pub struct FunTy {
    pub param_tys: Vec<Ty>,
    pub ret_ty: Box<Ty>,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Expr {
    Number(i64),
    VarRef(String),
    OpCall(String, Box<Expr>, Box<Expr>),
    FunCall(Box<Expr>, Vec<Expr>),
    If(Box<Expr>, Vec<Expr>, Option<Vec<Expr>>),
    Yield(Box<Expr>),
    While(Box<Expr>, Vec<Expr>),
    Spawn(Box<Expr>),
    Alloc(String),
    Assign(String, Box<Expr>),
    Return(Box<Expr>),
}
