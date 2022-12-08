use shiika_ast::*;
use shiika_core::ty::*;
use skc_hir::*;

/// Result of looking up a lvar
#[derive(Debug)]
pub(super) struct LVarInfo {
    pub ty: TermTy,
    pub detail: LVarDetail,
    /// The position of this lvar in the source text
    pub locs: LocationSpan,
}
#[derive(Debug)]
pub(super) enum LVarDetail {
    /// Found in the current scope
    CurrentScope { name: String },
    /// Found in the current method/lambda argument
    Argument { idx: usize },
    /// Found in outer scope
    OuterScope {
        /// Index of the lvar in `captures`
        cidx: usize,
        readonly: bool,
    },
    /// Same as `OuterScope` but `cidx` is not yet resolved.
    OuterScope_ { readonly: bool },
}

impl LVarInfo {
    /// Returns HirExpression to refer this lvar
    pub fn ref_expr(self) -> HirExpression {
        match self.detail {
            LVarDetail::CurrentScope { name } => Hir::lvar_ref(self.ty, name, self.locs),
            LVarDetail::Argument { idx } => Hir::arg_ref(self.ty, idx, self.locs),
            LVarDetail::OuterScope { cidx, readonly } => {
                Hir::lambda_capture_ref(self.ty, cidx, readonly, self.locs)
            }
            LVarDetail::OuterScope_ { .. } => panic!("[BUG] OuterScope_ leak"),
        }
    }

    /// Returns HirExpression to update this lvar
    pub fn assign_expr(self, expr: HirExpression) -> HirExpression {
        match self.detail {
            LVarDetail::CurrentScope { name, .. } => Hir::lvar_assign(name, expr, self.locs),
            LVarDetail::Argument { .. } => panic!("[BUG] Cannot reassign argument"),
            LVarDetail::OuterScope { cidx, .. } => Hir::lambda_capture_write(cidx, expr, self.locs),
            LVarDetail::OuterScope_ { .. } => panic!("[BUG] OuterScope_ leak"),
        }
    }

    /// Set `cidx` with the given value
    pub fn set_cidx(&mut self, cidx: usize) {
        match self.detail {
            LVarDetail::OuterScope_ { readonly } => {
                self.detail = LVarDetail::OuterScope { cidx, readonly }
            }
            _ => panic!("[BUG] Not LVarDetail::OuterScope_"),
        }
    }
}
