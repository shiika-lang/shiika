//! Converts if-else expressions to a sequence of blocks.
//! Intended to use `cf.cond_br` rather than `scf.if` which (IIRC) cannot contain
//! `func.return`.
//!
//! The branches must not contain async function calls (use lower_async_if to
//! remove them first.)
//!
//! Example:
//! ```
//! // Before
//! fun foo() {
//!   ...
//!   x = if (a) {
//!     b ...
//!     yield c
//!   } else {
//!     d ...
//!     yield e
//!   }
//!   ...
//!   x + ...
//!
//! // After
//! fun foo() -> Foo {
//!   ...
//!     cond_br a, ^bb1(), ^bb2()
//!
//!   ^bb1():
//!     b ...
//!     br ^bb3(c)
//!
//!   ^bb2():
//!     d ...
//!     br ^bb3(e)
//!
//!   ^bb3(x):
//!     ...
//!     x + ...
//! }
//! ```
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

struct Compiler {
    blocks: Vec<blocked::Block>,
}

impl Compiler {
    fn new(f: &hir::Function) -> Self {
        let first_block =
            blocked::Block::new_empty(f.params.iter().map(|p| p.ty.clone()).collect());
        Compiler {
            blocks: vec![first_block],
        }
    }

    fn compile_func(&mut self, body_stmts: Vec<hir::TypedExpr>) {
        for s in body_stmts {
            let new_s = self.walk_expr(s).unwrap();
            self.push(new_s);
        }
    }

    fn push(&mut self, e: hir::TypedExpr) {
        self.blocks.last_mut().unwrap().stmts.push(e);
    }
}

impl HirRewriter for Compiler {
    fn rewrite_expr(&mut self, e: hir::TypedExpr) -> Result<hir::TypedExpr> {
        match e.0 {
            hir::Expr::If(cond, then_exprs, else_exprs) => {
                let if_ty = e.1;
                let id = self.blocks.len() - 1;
                self.push(hir::Expr::cond_br(*cond, id + 1, id + 2));

                let then_block = blocked::Block::new(vec![], modify_branch(then_exprs, id + 3));
                self.blocks.push(then_block);
                let else_block = blocked::Block::new(vec![], modify_branch(else_exprs, id + 3));
                self.blocks.push(else_block);

                if if_ty == hir::Ty::Void {
                    Ok(hir::Expr::nop())
                } else {
                    let endif_block = blocked::Block::new_empty(vec![if_ty.clone()]);
                    self.blocks.push(endif_block);
                    Ok(hir::Expr::block_arg_ref(if_ty))
                }
            }
            _ => Ok(e),
        }
    }
}

/// Replace `yield` with `br` and `return` with `ret`.
fn modify_branch(mut exprs: Vec<hir::TypedExpr>, to: usize) -> Vec<hir::TypedExpr> {
    match exprs.pop().unwrap().0 {
        hir::Expr::Return(v) => exprs.push(hir::Expr::return_(*v)),
        hir::Expr::Yield(v) => {
            exprs.push(hir::Expr::br(*v, to));
        }
        _ => panic!(
            "[BUG] unexpected expr in modify_branch: {:?}",
            exprs.last().unwrap().0
        ),
    }
    exprs
}
