mod hir_maker;
mod hir_maker_context;
mod index;
mod sk_class;
use std::collections::HashMap;
use crate::ast;
use crate::ty;
use crate::ty::*;
use crate::names::*;
use crate::corelib::Corelib;
pub use sk_class::SkClass;

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
                },
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
    pub name: String,  // Starts with `@`
    pub ty: TermTy,
    pub readonly: bool,
}

#[derive(Debug, PartialEq)]
pub struct SkMethod {
    pub signature: MethodSignature,
    pub body: SkMethodBody,
}

pub enum SkMethodBody {
    ShiikaMethodBody {
        exprs: HirExpressions
    },
    RustMethodBody {
        gen: GenMethodBody
    },
    RustClosureMethodBody {
        boxed_gen: Box<ClosureMethodBody>
    }
}
// Manually deriving because GenMethodBody is a function (auto-deriving seems unsupported)
impl std::fmt::Debug for SkMethodBody {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "#<SkMethodBody>")
    }
}
impl std::cmp::PartialEq for SkMethodBody {
    fn eq(&self, other: &SkMethodBody) -> bool {
        match self {
            SkMethodBody::ShiikaMethodBody { exprs } => {
                match other {
                    SkMethodBody::ShiikaMethodBody { exprs: exprs2 } => return exprs == exprs2,
                    SkMethodBody::RustMethodBody { .. } => (),
                    SkMethodBody::RustClosureMethodBody { .. } => (),
                }
            },
            SkMethodBody::RustMethodBody { .. } => (),
            SkMethodBody::RustClosureMethodBody { .. } => (),
        }
        panic!("cannot compare RustMethodBody");
    }
}

pub type GenMethodBody = fn(code_gen: &crate::code_gen::CodeGen, function: &inkwell::values::FunctionValue) -> Result<(), crate::error::Error>;
pub type ClosureMethodBody = dyn Fn(&crate::code_gen::CodeGen, &inkwell::values::FunctionValue) -> Result<(), crate::error::Error>;

#[derive(Debug, PartialEq)]
pub struct HirExpressions {
    pub ty: TermTy,
    pub exprs: Vec<HirExpression>,
}

#[derive(Debug, PartialEq)]
pub struct HirExpression {
    pub ty: TermTy,
    pub node: HirExpressionBase,
}

#[derive(Debug, PartialEq)]
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
    HirSelfExpression,
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
    pub fn logical_not(expr_hir: HirExpression) -> HirExpression {
        HirExpression {
            ty: ty::raw("Bool"),
            node: HirExpressionBase::HirLogicalNot {
                expr: Box::new(expr_hir),
            }
        }
    }

    pub fn logical_and(left_hir: HirExpression,
                       right_hir: HirExpression) -> HirExpression {
        HirExpression {
            ty: ty::raw("Bool"),
            node: HirExpressionBase::HirLogicalAnd {
                left: Box::new(left_hir),
                right: Box::new(right_hir),
            }
        }
    }

    pub fn logical_or(left_hir: HirExpression,
                      right_hir: HirExpression) -> HirExpression {
        HirExpression {
            ty: ty::raw("Bool"),
            node: HirExpressionBase::HirLogicalOr {
                left: Box::new(left_hir),
                right: Box::new(right_hir),
            }
        }
    }

    pub fn if_expression(ty: TermTy,
                         cond_hir: HirExpression,
                         then_hir: HirExpressions,
                         else_hir: Option<HirExpressions>) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirIfExpression {
                cond_expr: Box::new(cond_hir),
                then_exprs: Box::new(then_hir),
                else_exprs: Box::new(else_hir),
            }
        }
    }

    pub fn while_expression(cond_hir: HirExpression,
                            body_hirs: HirExpressions) -> HirExpression {
        HirExpression {
            ty: ty::raw("Void"),
            node: HirExpressionBase::HirWhileExpression {
                cond_expr: Box::new(cond_hir),
                body_exprs: Box::new(body_hirs),
            }
        }
    }

    pub fn break_expression() -> HirExpression {
        HirExpression {
            ty: ty::raw("Never"),
            node: HirExpressionBase::HirBreakExpression {}
        }
    }

    pub fn assign_lvar(name: &str, rhs: HirExpression) -> HirExpression {
        HirExpression {
            ty: rhs.ty.clone(),
            node: HirExpressionBase::HirLVarAssign {
                name: name.to_string(),
                rhs: Box::new(rhs),
            }
        }
    }

    pub fn assign_ivar(name: &str, idx: usize, rhs: HirExpression, writable: bool) -> HirExpression {
        HirExpression {
            ty: rhs.ty.clone(),
            node: HirExpressionBase::HirIVarAssign {
                name: name.to_string(),
                idx,
                rhs: Box::new(rhs),
                writable,
            }
        }
    }

    pub fn assign_const(fullname: ConstFullname, rhs: HirExpression) -> HirExpression {
        HirExpression {
            ty: rhs.ty.clone(),
            node: HirExpressionBase::HirConstAssign {
                fullname,
                rhs: Box::new(rhs),
            }
        }
    }

    pub fn method_call(result_ty: TermTy, receiver_hir: HirExpression, method_fullname: MethodFullname, arg_hirs: Vec<HirExpression>) -> HirExpression {
        HirExpression {
            ty: result_ty,
            node: HirExpressionBase::HirMethodCall {
                receiver_expr: Box::new(receiver_hir),
                method_fullname,
                arg_exprs: arg_hirs,
            }
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

    pub fn self_expression(ty: TermTy) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirSelfExpression,
        }
    }

    pub fn float_literal(value: f64) -> HirExpression {
        HirExpression {
            ty: ty::raw("Float"),
            node: HirExpressionBase::HirFloatLiteral { value }
        }
    }
    
    pub fn decimal_literal(value: i32) -> HirExpression {
        HirExpression {
            ty: ty::raw("Int"),
            node: HirExpressionBase::HirDecimalLiteral { value }
        }
    }
    
    pub fn string_literal(idx: usize) -> HirExpression {
        HirExpression {
            ty: ty::raw("String"),
            node: HirExpressionBase::HirStringLiteral { idx }
        }
    }

    pub fn boolean_literal(value: bool) -> HirExpression {
        HirExpression {
            ty: ty::raw("Bool"),
            node: HirExpressionBase::HirBooleanLiteral { value }
        }
    }

    pub fn bit_cast(ty: TermTy, expr: HirExpression) -> HirExpression {
        HirExpression {
            ty,
            node: HirExpressionBase::HirBitCast { expr: Box::new(expr) }
        }
    }

    pub fn class_literal(fullname: ClassFullname, str_literal_idx: usize) -> HirExpression {
        HirExpression {
            ty: ty::meta(&fullname.0),
            node: HirExpressionBase::HirClassLiteral { fullname, str_literal_idx }
        }
    }
}

/// Create `hir::MethodSignature` from `ast::MethodSignature`
pub fn create_signature(class_fullname: String, sig: &ast::AstMethodSignature) -> MethodSignature {
    let fullname = MethodFullname {
        full_name: (class_fullname + "#" + &sig.name.0),
        first_name: sig.name.clone(),
    };
    let ret_ty = convert_typ(&sig.ret_typ);
    let params = convert_params(&sig.params);
    MethodSignature { fullname, ret_ty, params }
}

fn convert_typ(typ: &ast::Typ) -> TermTy {
    ty::raw(&typ.name)
}

fn convert_params(params: &[ast::Param]) -> Vec<MethodParam> {
    params.iter().map(|param|
        MethodParam {
            name: param.name.to_string(),
            ty: convert_typ(&param.typ),
        }
    ).collect()
}

/// Create a signature of `.new`
fn signature_of_new(metaclass_fullname: &ClassFullname,
                    initialize_params: &[ast::Param],
                    instance_ty: &TermTy) -> MethodSignature {
    MethodSignature {
        fullname: method_fullname(metaclass_fullname, "new"),
        ret_ty: instance_ty.clone(),
        params: convert_params(initialize_params),
    }
}
