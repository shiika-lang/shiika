use crate::hir;
use crate::hir::blocked;
use crate::hir::rewriter::HirRewriter;
use anyhow::Result;

pub fn run(program: hir::Program) -> blocked::Program {
    let funcs = program
        .funcs
        .into_iter()
        .map(|f| {
            let mut c = Compiler::new(&f);
            c.compile_func(f.body_stmts);
            blocked::Function {
                name: f.name,
                params: f.params,
                ret_ty: f.ret_ty,
                body_blocks: c.blocks,
            }
        })
        .collect();
    blocked::Program {
        externs: program.externs,
        funcs,
    }
}
