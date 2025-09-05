pub mod method_signature;
mod method_signatures;
pub mod pattern_match;
mod sk_method;
mod sk_type;
mod sk_types;
mod supertype;
pub mod visitor;
pub use crate::method_signature::*;
pub use crate::method_signatures::MethodSignatures;
pub use crate::sk_method::{SkMethod, SkMethodBody, SkMethods};
pub use crate::sk_type::{SkClass, SkModule, SkType, SkTypeBase, WTable};
pub use crate::sk_types::SkTypes;
pub use crate::supertype::Supertype;
use serde::{Deserialize, Serialize};
use shiika_ast::LocationSpan;
use shiika_core::{names::*, ty, ty::*};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Hir {
    pub sk_types: SkTypes,
    pub sk_methods: SkMethods,
    pub constants: HashMap<ConstFullname, TermTy>,
    pub str_literals: Vec<String>,
    pub const_inits: Vec<HirExpression>,
    pub main_exprs: Vec<HirExpression>,
    /// Local variables in `main_exprs`
    pub main_lvars: HirLVars,
}

impl Hir {
    pub fn add_methods(&mut self, sk_methods: SkMethods) {
        for (classname, mut new_methods) in sk_methods {
            match self.sk_methods.get_mut(&classname) {
                Some(methods) => {
                    methods.append(&mut new_methods);
                }
                None => {
                    self.sk_methods.insert(classname, new_methods);
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SkIVar {
    pub idx: usize,
    pub name: String, // Includes `@`
    pub ty: TermTy,
    pub readonly: bool,
}

impl SkIVar {
    /// Return "foo" for `@foo`
    pub fn accessor_name(&self) -> String {
        self.name.replace('@', "")
    }

    /// Apply type arguments
    pub fn substitute(&self, tyargs: &[TermTy]) -> SkIVar {
        let mut ivar = self.clone();
        ivar.ty = self.ty.substitute(tyargs, &[]);
        ivar
    }
}

pub type SkIVars = HashMap<String, SkIVar>;

pub type HirLVars = Vec<HirLVar>;

#[derive(Debug, Clone)]
pub struct HirLVar {
    pub name: String,
    pub ty: TermTy,
    /// `true` when this variable is captured by a lambda.
    pub captured: bool,
}

/// Make a HirExpression to refer `::Void`
fn void_const_ref() -> HirExpression {
    Hir::const_ref(
        ty::raw("Void"),
        toplevel_const("Void"),
        LocationSpan::internal(),
    )
}

#[derive(Debug, Clone)]
pub struct HirExpression {
    pub ty: TermTy,
    pub node: HirExpressionBase,
    pub locs: LocationSpan,
}

impl HirExpression {
    pub fn voidify(self) -> HirExpression {
        let exprs = vec![self, void_const_ref()];
        Hir::expressions(exprs)
    }

    pub fn last_expr(&self) -> &HirExpression {
        match &self.node {
            HirExpressionBase::HirParenthesizedExpr { exprs } => exprs.last().unwrap(),
            _ => panic!("unexpected"),
        }
    }

    pub fn to_expr_vec(&self) -> Vec<HirExpression> {
        match &self.node {
            HirExpressionBase::HirParenthesizedExpr { exprs } => exprs.clone(),
            _ => vec![self.clone()],
        }
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum HirExpressionBase {
    HirLogicalNot {
        expr: Box<HirExpression>,
    },
    HirLogicalAnd {
        left: Box<HirExpression>,
        right: Box<HirExpression>,
    },
    HirLogicalOr {
        left: Box<HirExpression>,
        right: Box<HirExpression>,
    },
    HirIfExpression {
        lvars: HirLVars,
        cond_expr: Box<HirExpression>,
        then_exprs: Box<HirExpression>,
        else_exprs: Box<HirExpression>,
    },
    HirMatchExpression {
        cond_assign_expr: Box<HirExpression>,
        clauses: Vec<pattern_match::MatchClause>,
    },
    HirWhileExpression {
        lvars: HirLVars,
        cond_expr: Box<HirExpression>,
        body_exprs: Box<HirExpression>,
    },
    HirBreakExpression {
        from: HirBreakFrom,
    },
    HirReturnExpression {
        from: HirReturnFrom,
        arg: Box<HirExpression>,
    },
    HirLVarAssign {
        name: String,
        rhs: Box<HirExpression>,
    },
    HirIVarAssign {
        name: String,
        idx: usize,
        rhs: Box<HirExpression>,
        writable: bool,
        self_ty: TermTy,
    },
    HirConstAssign {
        fullname: ConstFullname,
        rhs: Box<HirExpression>,
    },
    HirMethodCall {
        receiver_expr: Box<HirExpression>,
        method_fullname: MethodFullname,
        arg_exprs: Vec<HirExpression>,
        tyarg_exprs: Vec<HirExpression>,
    },
    HirModuleMethodCall {
        receiver_expr: Box<HirExpression>,
        module_fullname: ModuleFullname,
        method_name: MethodFirstname,
        method_idx: usize,
        arg_exprs: Vec<HirExpression>,
        tyarg_exprs: Vec<HirExpression>,
    },
    HirLambdaInvocation {
        lambda_expr: Box<HirExpression>,
        arg_exprs: Vec<HirExpression>,
    },
    HirArgRef {
        idx: usize,
    },
    HirLVarRef {
        name: String,
    },
    HirIVarRef {
        name: String,
        idx: usize,
        self_ty: TermTy,
    },
    /// Reference of class-wise type variable
    HirClassTVarRef {
        typaram_ref: TyParamRef,
        self_ty: TermTy,
    },
    /// Reference of method-wise type variable
    HirMethodTVarRef {
        typaram_ref: TyParamRef,
        n_params: usize,
    },
    HirConstRef {
        fullname: ConstFullname,
    },
    HirLambdaExpr {
        name: String,
        params: Vec<MethodParam>,
        exprs: Box<HirExpression>,
        captures: Vec<HirLambdaCapture>,
        lvars: HirLVars,
        ret_ty: TermTy,
        /// true if there is a `break` in this lambda
        has_break: bool,
    },
    HirSelfExpression,
    HirFloatLiteral {
        value: f64,
    },
    HirDecimalLiteral {
        value: i64,
    },
    /// A string literal. Its body is stored in str_literals
    HirStringLiteral {
        idx: usize,
    },
    HirBooleanLiteral {
        value: bool,
    },

    //
    // Special opecodes (does not appear in a source program directly)
    //
    /// Refer a variable in the `captures` of the current lambda
    HirLambdaCaptureRef {
        idx: usize,
        /// Whether this capture is a readonly one (i.e. passed by value)
        readonly: bool,
    },
    /// Reassign to a variable in the `captures` of the current lambda
    HirLambdaCaptureWrite {
        cidx: usize,
        rhs: Box<HirExpression>,
    },
    /// Represents bitcast of an object
    HirBitCast {
        expr: Box<HirExpression>,
    },
    /// A special expression that evaluates to a class object; used for
    /// initializing class constants.
    HirClassLiteral {
        fullname: TypeFullname,
        str_literal_idx: usize,
        includes_modules: bool,
        initializer: Option<MethodSignature>,
    },
    /// Wrap several expressions in to an expression
    HirParenthesizedExpr {
        exprs: Vec<HirExpression>,
    },
    /// Only appears as an method call argument. Denotes using the default value.
    HirDefaultExpr,
    /// Only appears at the beginning of a method body. Evaluates to `true` if
    /// `expr` is a special value (currently a nullptr) which denotes the argument is omitted.
    HirIsOmittedValue {
        expr: Box<HirExpression>,
    },
}

/// Denotes which variable to include in the `captures`
#[derive(Debug, Clone)]
pub struct HirLambdaCapture {
    pub ty: TermTy,
    pub upcast_needed: bool,
    /// True if the captured variable is readonly
    pub readonly: bool,
    pub detail: HirLambdaCaptureDetail,
}

#[derive(Debug, Clone)]
pub enum HirLambdaCaptureDetail {
    /// Local variable
    CaptureLVar { name: String },
    /// Method/Function argument
    CaptureArg { idx: usize },
    /// Method-wise type argument
    CaptureMethodTyArg { idx: usize, n_params: usize },
    /// Variable in the current `captures`
    CaptureFwd { cidx: usize },
}

/// Denotes what a `break` escapes from
#[derive(Debug, Clone)]
pub enum HirBreakFrom {
    While,
    Block,
}

/// Denotes what a `return` escapes from
#[derive(Debug, Clone)]
pub enum HirReturnFrom {
    Fn,
    Block,
    Method,
}

impl Hir {
    pub fn expressions(mut exprs: Vec<HirExpression>) -> HirExpression {
        if exprs.is_empty() {
            exprs.push(void_const_ref());
        }
        let last_expr = exprs.last().unwrap();
        let ty = last_expr.ty.clone();
        let locs = LocationSpan::merge(&exprs.first().unwrap().locs, &last_expr.locs);
        Hir::parenthesized_expression(ty, exprs, locs)
    }

    pub fn logical_not(expr_hir: HirExpression, locs: LocationSpan) -> HirExpression {
        HirExpression {
            ty: ty::raw("Bool"),
            node: HirExpressionBase::HirLogicalNot {
                expr: Box::new(expr_hir),
            },
            locs,
        }
    }

    pub fn logical_and(
        left_hir: HirExpression,
        right_hir: HirExpression,
        locs: LocationSpan,
    ) -> HirExpression {
        HirExpression {
            ty: ty::raw("Bool"),
            node: HirExpressionBase::HirLogicalAnd {
                left: Box::new(left_hir),
                right: Box::new(right_hir),
            },
            locs,
        }
    }

    pub fn logical_or(
        left_hir: HirExpression,
        right_hir: HirExpression,
        locs: LocationSpan,
    ) -> HirExpression {
        HirExpression {
            ty: ty::raw("Bool"),
            node: HirExpressionBase::HirLogicalOr {
                left: Box::new(left_hir),
                right: Box::new(right_hir),
            },
            locs,
        }
    }

    pub fn if_expression(
        ty: TermTy,
        cond_hir: HirExpression,
        then_hir: HirExpression,
        else_hir: HirExpression,
        lvars: HirLVars,
        locs: LocationSpan,
    ) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirIfExpression {
                lvars,
                cond_expr: Box::new(cond_hir),
                then_exprs: Box::new(then_hir),
                else_exprs: Box::new(else_hir),
            },
            locs,
        }
    }

    pub fn match_expression(
        ty: TermTy,
        cond_assign_hir: HirExpression,
        clauses: Vec<pattern_match::MatchClause>,
        locs: LocationSpan,
    ) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirMatchExpression {
                cond_assign_expr: Box::new(cond_assign_hir),
                clauses,
            },
            locs,
        }
    }

    pub fn while_expression(
        cond_hir: HirExpression,
        body_hirs: HirExpression,
        lvars: HirLVars,
        locs: LocationSpan,
    ) -> HirExpression {
        HirExpression {
            ty: ty::raw("Void"),
            node: HirExpressionBase::HirWhileExpression {
                cond_expr: Box::new(cond_hir),
                body_exprs: Box::new(body_hirs),
                lvars,
            },
            locs,
        }
    }

    pub fn break_expression(from: HirBreakFrom, locs: LocationSpan) -> HirExpression {
        HirExpression {
            ty: ty::raw("Never"),
            node: HirExpressionBase::HirBreakExpression { from },
            locs,
        }
    }

    pub fn return_expression(
        from: HirReturnFrom,
        arg_expr: HirExpression,
        locs: LocationSpan,
    ) -> HirExpression {
        HirExpression {
            ty: ty::raw("Never"),
            node: HirExpressionBase::HirReturnExpression {
                from,
                arg: Box::new(arg_expr),
            },
            locs,
        }
    }

    pub fn lvar_assign(name: String, rhs: HirExpression, locs: LocationSpan) -> HirExpression {
        HirExpression {
            ty: rhs.ty.clone(),
            node: HirExpressionBase::HirLVarAssign {
                name,
                rhs: Box::new(rhs),
            },
            locs,
        }
    }

    pub fn ivar_assign(
        name: &str,
        idx: usize,
        rhs: HirExpression,
        writable: bool,
        self_ty: TermTy,
        locs: LocationSpan,
    ) -> HirExpression {
        HirExpression {
            ty: rhs.ty.clone(),
            node: HirExpressionBase::HirIVarAssign {
                name: name.to_string(),
                idx,
                rhs: Box::new(rhs),
                writable,
                self_ty,
            },
            locs,
        }
    }

    pub fn const_assign(
        fullname: ConstFullname,
        rhs: HirExpression,
        locs: LocationSpan,
    ) -> HirExpression {
        HirExpression {
            ty: rhs.ty.clone(),
            node: HirExpressionBase::HirConstAssign {
                fullname,
                rhs: Box::new(rhs),
            },
            locs,
        }
    }

    pub fn method_call(
        result_ty: TermTy,
        receiver_hir: HirExpression,
        method_fullname: MethodFullname,
        arg_exprs: Vec<HirExpression>,
        tyarg_exprs: Vec<HirExpression>,
    ) -> HirExpression {
        let locs = LocationSpan::merge(
            &receiver_hir.locs,
            if let Some(e) = arg_exprs.last() {
                &e.locs
            } else {
                &receiver_hir.locs
            },
        );
        HirExpression {
            ty: result_ty,
            node: HirExpressionBase::HirMethodCall {
                receiver_expr: Box::new(receiver_hir),
                method_fullname,
                arg_exprs,
                tyarg_exprs,
            },
            locs,
        }
    }

    pub fn module_method_call(
        result_ty: TermTy,
        receiver_hir: HirExpression,
        module_fullname: ModuleFullname,
        method_name: MethodFirstname,
        method_idx: usize,
        arg_exprs: Vec<HirExpression>,
        tyarg_exprs: Vec<HirExpression>,
    ) -> HirExpression {
        let locs = LocationSpan::merge(
            &receiver_hir.locs,
            if let Some(e) = arg_exprs.last() {
                &e.locs
            } else {
                &receiver_hir.locs
            },
        );
        HirExpression {
            ty: result_ty,
            node: HirExpressionBase::HirModuleMethodCall {
                receiver_expr: Box::new(receiver_hir),
                module_fullname,
                method_name,
                method_idx,
                arg_exprs,
                tyarg_exprs,
            },
            locs,
        }
    }

    pub fn lambda_invocation(
        result_ty: TermTy,
        varref_expr: HirExpression,
        arg_hirs: Vec<HirExpression>,
        locs: LocationSpan,
    ) -> HirExpression {
        HirExpression {
            ty: result_ty,
            node: HirExpressionBase::HirLambdaInvocation {
                lambda_expr: Box::new(varref_expr),
                arg_exprs: arg_hirs,
            },
            locs,
        }
    }

    pub fn arg_ref(ty: TermTy, idx: usize, locs: LocationSpan) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirArgRef { idx },
            locs,
        }
    }

    pub fn lvar_ref(ty: TermTy, name: String, locs: LocationSpan) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirLVarRef { name },
            locs,
        }
    }

    pub fn ivar_ref(
        ty: TermTy,
        name: String,
        idx: usize,
        self_ty: TermTy,
        locs: LocationSpan,
    ) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirIVarRef { name, idx, self_ty },
            locs,
        }
    }

    pub fn class_tvar_ref(
        ty: TermTy,
        typaram_ref: TyParamRef,
        self_ty: TermTy,
        locs: LocationSpan,
    ) -> HirExpression {
        debug_assert!(typaram_ref.kind == TyParamKind::Class);
        HirExpression {
            ty,
            node: HirExpressionBase::HirClassTVarRef {
                typaram_ref,
                self_ty,
            },
            locs,
        }
    }

    pub fn method_tvar_ref(
        ty: TermTy,
        typaram_ref: TyParamRef,
        n_params: usize,
        locs: LocationSpan,
    ) -> HirExpression {
        debug_assert!(typaram_ref.kind == TyParamKind::Method);
        HirExpression {
            ty,
            node: HirExpressionBase::HirMethodTVarRef {
                typaram_ref,
                n_params,
            },
            locs,
        }
    }

    pub fn const_ref(ty: TermTy, fullname: ConstFullname, locs: LocationSpan) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirConstRef { fullname },
            locs,
        }
    }

    // REFACTOR: `ty` can be omittable?
    #[allow(clippy::too_many_arguments)]
    pub fn lambda_expr(
        ty: TermTy,
        name: String,
        params: Vec<MethodParam>,
        exprs: HirExpression,
        captures: Vec<HirLambdaCapture>,
        lvars: HirLVars,
        has_break: bool,
        locs: LocationSpan,
    ) -> HirExpression {
        let ret_ty = exprs.ty.clone();
        HirExpression {
            ty,
            node: HirExpressionBase::HirLambdaExpr {
                name,
                params,
                exprs: Box::new(exprs),
                captures,
                lvars,
                ret_ty,
                has_break,
            },
            locs,
        }
    }

    pub fn self_expression(ty: TermTy, locs: LocationSpan) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirSelfExpression,
            locs,
        }
    }

    pub fn float_literal(value: f64, locs: LocationSpan) -> HirExpression {
        HirExpression {
            ty: ty::raw("Float"),
            node: HirExpressionBase::HirFloatLiteral { value },
            locs,
        }
    }

    pub fn decimal_literal(value: i64, locs: LocationSpan) -> HirExpression {
        HirExpression {
            ty: ty::raw("Int"),
            node: HirExpressionBase::HirDecimalLiteral { value },
            locs,
        }
    }

    pub fn string_literal(idx: usize, locs: LocationSpan) -> HirExpression {
        HirExpression {
            ty: ty::raw("String"),
            node: HirExpressionBase::HirStringLiteral { idx },
            locs,
        }
    }

    pub fn boolean_literal(value: bool, locs: LocationSpan) -> HirExpression {
        HirExpression {
            ty: ty::raw("Bool"),
            node: HirExpressionBase::HirBooleanLiteral { value },
            locs,
        }
    }

    pub fn bit_cast(ty: TermTy, expr: HirExpression) -> HirExpression {
        let locs = expr.locs.clone();
        HirExpression {
            ty,
            node: HirExpressionBase::HirBitCast {
                expr: Box::new(expr),
            },
            locs,
        }
    }

    pub fn class_literal(
        ty: TermTy,
        fullname: TypeFullname,
        str_literal_idx: usize,
        includes_modules: bool,
        initializer: Option<MethodSignature>,
    ) -> HirExpression {
        debug_assert!(ty.is_metaclass());
        HirExpression {
            ty,
            node: HirExpressionBase::HirClassLiteral {
                fullname,
                str_literal_idx,
                includes_modules,
                initializer,
            },
            locs: LocationSpan::internal(),
        }
    }

    pub fn parenthesized_expression(
        ty: TermTy,
        exprs: Vec<HirExpression>,
        locs: LocationSpan,
    ) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirParenthesizedExpr { exprs },
            locs,
        }
    }

    pub fn default_expression(ty: TermTy) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirDefaultExpr,
            locs: LocationSpan::internal(),
        }
    }

    pub fn is_omitted_value(expr: HirExpression) -> HirExpression {
        HirExpression {
            ty: ty::raw("Bool"),
            node: HirExpressionBase::HirIsOmittedValue {
                expr: Box::new(expr),
            },
            locs: LocationSpan::internal(),
        }
    }

    pub fn lambda_capture_ref(
        ty: TermTy,
        idx: usize,
        readonly: bool,
        locs: LocationSpan,
    ) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirLambdaCaptureRef { idx, readonly },
            locs,
        }
    }

    pub fn lambda_capture_write(
        cidx: usize,
        rhs: HirExpression,
        locs: LocationSpan,
    ) -> HirExpression {
        HirExpression {
            ty: rhs.ty.clone(),
            node: HirExpressionBase::HirLambdaCaptureWrite {
                cidx,
                rhs: Box::new(rhs),
            },
            locs,
        }
    }
}
