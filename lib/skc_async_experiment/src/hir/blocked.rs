use crate::hir;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Block {
    pub param_tys: Vec<hir::Ty>,
    pub stmts: Vec<hir::Typed<hir::Expr>>,
}

impl Block {
    pub fn new_empty(param_tys: Vec<hir::Ty>) -> Self {
        Block {
            param_tys,
            stmts: vec![],
        }
    }

    pub fn new(param_tys: Vec<hir::Ty>, stmts: Vec<hir::Typed<hir::Expr>>) -> Self {
        Block { param_tys, stmts }
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub externs: Vec<hir::Extern>,
    pub funcs: Vec<Function>,
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for e in &self.externs {
            write!(f, "{}", e)?;
        }
        for func in &self.funcs {
            write!(f, "{}", func)?;
        }
        write!(f, "")
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<hir::Param>,
    pub ret_ty: hir::Ty,
    pub body_blocks: Vec<Block>,
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let para = self
            .params
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "fun {}({}) -> {} {{\n", self.name, para, self.ret_ty)?;
        for (i, block) in self.body_blocks.iter().enumerate() {
            if i != 0 {
                let para = block
                    .param_tys
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "^bb{i}({para})\n")?;
            }
            for expr in &block.stmts {
                write!(f, "  {};  #-> {}\n", &expr.0, &expr.1)?;
            }
        }
        write!(f, "}}\n")
    }
}

impl Function {
    pub fn fun_ty(&self) -> hir::FunTy {
        hir::FunTy {
            asyncness: hir::Asyncness::Lowered,
            param_tys: self.params.iter().map(|x| x.ty.clone()).collect::<Vec<_>>(),
            ret_ty: Box::new(self.ret_ty.clone()),
        }
    }
}
