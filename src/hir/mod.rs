mod accessors;
pub mod class_dict;
mod convert_exprs;
mod hir_maker;
mod hir_maker_context;
mod method_dict;
pub mod signature;
mod sk_class;
use crate::ast;
use crate::corelib::Corelib;
use crate::names::*;
use crate::ty;
use crate::ty::*;
pub use sk_class::SkClass;
use std::collections::HashMap;

// TODO: This is not true for Fn2, Fn3, ...
const IDX_LAMBDA_CAPTURES: usize = 1;

#[derive(Debug)]
pub struct Hir {
    pub sk_classes: HashMap<ClassFullname, SkClass>,
    pub sk_methods: HashMap<ClassFullname, Vec<SkMethod>>,
    pub constants: HashMap<ConstFullname, TermTy>,
    pub str_literals: Vec<String>,
    pub const_inits: Vec<HirExpression>,
    pub main_exprs: HirExpressions,
}

pub fn build(ast: ast::Program, corelib: Corelib) -> Result<Hir, crate::error::Error> {
    hir_maker::make_hir(ast, corelib)
}

impl Hir {
    pub fn add_methods(&mut self, sk_methods: HashMap<ClassFullname, Vec<SkMethod>>) {
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

#[derive(Debug, PartialEq, Clone)]
pub struct SkIVar {
    pub idx: usize,
    pub name: String, // Without `@`
    pub ty: TermTy,
    pub readonly: bool,
}

type SkIVars = HashMap<String, SkIVar>;

#[derive(Debug)]
pub struct SkMethod {
    pub signature: MethodSignature,
    pub body: SkMethodBody,
}

pub enum SkMethodBody {
    ShiikaMethodBody { exprs: HirExpressions },
    RustMethodBody { gen: GenMethodBody },
    RustClosureMethodBody { boxed_gen: Box<ClosureMethodBody> },
}
// Manually deriving because GenMethodBody is a function (auto-deriving seems unsupported)
impl std::fmt::Debug for SkMethodBody {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "#<SkMethodBody>")
    }
}

pub type GenMethodBody = fn(
    code_gen: &crate::code_gen::CodeGen,
    function: &inkwell::values::FunctionValue,
) -> Result<(), crate::error::Error>;
pub type ClosureMethodBody = dyn Fn(
    &crate::code_gen::CodeGen,
    &inkwell::values::FunctionValue,
) -> Result<(), crate::error::Error>;

#[derive(Debug)]
pub struct HirExpressions {
    pub ty: TermTy,
    pub exprs: Vec<HirExpression>,
}

impl HirExpressions {
    // Destructively convert Vec<HirExpression> into HirExpressions
    pub fn new(mut exprs: Vec<HirExpression>) -> HirExpressions {
        if exprs.is_empty() {
            exprs.push(Hir::const_ref(ty::raw("Void"), const_fullname("::Void")))
        }

        let last_expr = exprs.last().unwrap();
        let ty = last_expr.ty.clone();

        HirExpressions { ty, exprs }
    }
}

#[derive(Debug)]
pub struct HirExpression {
    pub ty: TermTy,
    pub node: HirExpressionBase,
}

#[derive(Debug)]
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
        cond_expr: Box<HirExpression>,
        then_exprs: Box<HirExpressions>,
        else_exprs: Box<Option<HirExpressions>>,
    },
    HirWhileExpression {
        cond_expr: Box<HirExpression>,
        body_exprs: Box<HirExpressions>,
    },
    HirBreakExpression,
    HirLVarAssign {
        name: String,
        rhs: Box<HirExpression>,
    },
    HirIVarAssign {
        name: String,
        idx: usize,
        rhs: Box<HirExpression>,
        writable: bool,
    },
    HirConstAssign {
        fullname: ConstFullname,
        rhs: Box<HirExpression>,
    },
    HirMethodCall {
        receiver_expr: Box<HirExpression>,
        method_fullname: MethodFullname,
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
    },
    HirConstRef {
        fullname: ConstFullname,
    },
    HirLambdaExpr {
        name: String,
        params: Vec<MethodParam>,
        exprs: HirExpressions,
        captures_ary: Box<HirExpression>,
    },
    HirSelfExpression,
    HirArrayLiteral {
        exprs: HirExpressions,
    },
    HirFloatLiteral {
        value: f64,
    },
    HirDecimalLiteral {
        value: i32,
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
    /// Represents bitcast of an object
    HirBitCast {
        expr: Box<HirExpression>,
    },
    /// A special expression that evaluates to a class
    /// (eg. `class A; end; A = 1` shadows A, but this special expr
    /// is never be shadowed)
    HirClassLiteral {
        fullname: ClassFullname,
        str_literal_idx: usize,
    },
}

impl Hir {
    pub fn expressions(exprs: Vec<HirExpression>) -> HirExpressions {
        HirExpressions::new(exprs)
    }

    pub fn logical_not(expr_hir: HirExpression) -> HirExpression {
        HirExpression {
            ty: ty::raw("Bool"),
            node: HirExpressionBase::HirLogicalNot {
                expr: Box::new(expr_hir),
            },
        }
    }

    pub fn logical_and(left_hir: HirExpression, right_hir: HirExpression) -> HirExpression {
        HirExpression {
            ty: ty::raw("Bool"),
            node: HirExpressionBase::HirLogicalAnd {
                left: Box::new(left_hir),
                right: Box::new(right_hir),
            },
        }
    }

    pub fn logical_or(left_hir: HirExpression, right_hir: HirExpression) -> HirExpression {
        HirExpression {
            ty: ty::raw("Bool"),
            node: HirExpressionBase::HirLogicalOr {
                left: Box::new(left_hir),
                right: Box::new(right_hir),
            },
        }
    }

    pub fn if_expression(
        ty: TermTy,
        cond_hir: HirExpression,
        then_hir: HirExpressions,
        else_hir: Option<HirExpressions>,
    ) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirIfExpression {
                cond_expr: Box::new(cond_hir),
                then_exprs: Box::new(then_hir),
                else_exprs: Box::new(else_hir),
            },
        }
    }

    pub fn while_expression(cond_hir: HirExpression, body_hirs: HirExpressions) -> HirExpression {
        HirExpression {
            ty: ty::raw("Void"),
            node: HirExpressionBase::HirWhileExpression {
                cond_expr: Box::new(cond_hir),
                body_exprs: Box::new(body_hirs),
            },
        }
    }

    pub fn break_expression() -> HirExpression {
        HirExpression {
            ty: ty::raw("Never"),
            node: HirExpressionBase::HirBreakExpression {},
        }
    }

    pub fn assign_lvar(name: &str, rhs: HirExpression) -> HirExpression {
        HirExpression {
            ty: rhs.ty.clone(),
            node: HirExpressionBase::HirLVarAssign {
                name: name.to_string(),
                rhs: Box::new(rhs),
            },
        }
    }

    pub fn assign_ivar(
        name: &str,
        idx: usize,
        rhs: HirExpression,
        writable: bool,
    ) -> HirExpression {
        HirExpression {
            ty: rhs.ty.clone(),
            node: HirExpressionBase::HirIVarAssign {
                name: name.to_string(),
                idx,
                rhs: Box::new(rhs),
                writable,
            },
        }
    }

    pub fn assign_const(fullname: ConstFullname, rhs: HirExpression) -> HirExpression {
        HirExpression {
            ty: rhs.ty.clone(),
            node: HirExpressionBase::HirConstAssign {
                fullname,
                rhs: Box::new(rhs),
            },
        }
    }

    pub fn method_call(
        result_ty: TermTy,
        receiver_hir: HirExpression,
        method_fullname: MethodFullname,
        arg_hirs: Vec<HirExpression>,
    ) -> HirExpression {
        HirExpression {
            ty: result_ty,
            node: HirExpressionBase::HirMethodCall {
                receiver_expr: Box::new(receiver_hir),
                method_fullname,
                arg_exprs: arg_hirs,
            },
        }
    }

    // REFACTOR: Remove `hir_`
    pub fn hir_arg_ref(ty: TermTy, idx: usize) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirArgRef { idx },
        }
    }

    pub fn lvar_ref(ty: TermTy, name: String) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirLVarRef { name },
        }
    }

    pub fn ivar_ref(ty: TermTy, name: String, idx: usize) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirIVarRef { name, idx },
        }
    }

    pub fn const_ref(ty: TermTy, fullname: ConstFullname) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirConstRef { fullname },
        }
    }

    pub fn lambda_expr(
        n: usize,
        mut params: Vec<MethodParam>,
        exprs: HirExpressions,
        captures_ary: HirExpression,
    ) -> HirExpression {
        let name = format!("lambda_{}", n);
        let ty = lambda_ty(&params, &exprs.ty);
        params.push(MethodParam {
            name: "(captures)".to_string(),
            ty: ty::ary(ty::raw("Object")),
        });
        HirExpression {
            ty,
            node: HirExpressionBase::HirLambdaExpr {
                name,
                params,
                exprs,
                captures_ary: Box::new(captures_ary),
            },
        }
    }

    pub fn self_expression(ty: TermTy) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirSelfExpression,
        }
    }

    pub fn array_literal(exprs: Vec<HirExpression>, ty: TermTy) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirArrayLiteral {
                exprs: HirExpressions::new(exprs),
            },
        }
    }

    pub fn float_literal(value: f64) -> HirExpression {
        HirExpression {
            ty: ty::raw("Float"),
            node: HirExpressionBase::HirFloatLiteral { value },
        }
    }

    pub fn decimal_literal(value: i32) -> HirExpression {
        HirExpression {
            ty: ty::raw("Int"),
            node: HirExpressionBase::HirDecimalLiteral { value },
        }
    }

    pub fn string_literal(idx: usize) -> HirExpression {
        HirExpression {
            ty: ty::raw("String"),
            node: HirExpressionBase::HirStringLiteral { idx },
        }
    }

    pub fn boolean_literal(value: bool) -> HirExpression {
        HirExpression {
            ty: ty::raw("Bool"),
            node: HirExpressionBase::HirBooleanLiteral { value },
        }
    }

    pub fn bit_cast(ty: TermTy, expr: HirExpression) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirBitCast {
                expr: Box::new(expr),
            },
        }
    }

    pub fn class_literal(fullname: ClassFullname, str_literal_idx: usize) -> HirExpression {
        HirExpression {
            ty: ty::meta(&fullname.0),
            node: HirExpressionBase::HirClassLiteral {
                fullname,
                str_literal_idx,
            },
        }
    }

    // HirArgRef + Array#[]
    pub fn lambda_capture_ref(ty: TermTy, idx: usize) -> HirExpression {
        let ary = Hir::hir_arg_ref(
            ty::spe("Array", vec![ty::raw("Object")]),
            IDX_LAMBDA_CAPTURES,
        );
        let nth_obj = Hir::method_call(
            ty.clone(),
            ary,
            method_fullname(&class_fullname("Array"), "nth"),
            vec![Hir::decimal_literal(idx as i32)],
        );
        Hir::bit_cast(ty, nth_obj)
    }
}

fn lambda_ty(params: &Vec<MethodParam>, ret_ty: &TermTy) -> TermTy {
    if params.len() != 1 {
        panic!("TODO")
    }
    ty::spe("Fn1", vec![params[0].ty.clone(), ret_ty.clone()])
}
