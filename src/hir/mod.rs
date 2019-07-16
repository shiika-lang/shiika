mod hir_maker;
mod hir_maker_context;
mod index;
use crate::ast;
use crate::ty;
use crate::ty::*;
use crate::names::*;

pub struct Hir {
    pub sk_classes: Vec<SkClass>,
    pub main_exprs: HirExpressions,
}
impl Hir {
    pub fn from_ast(ast: ast::Program, stdlib: &Vec<SkClass>) -> Result<Hir, crate::error::Error> {
        let index = index::Index::new(stdlib, &ast.toplevel_defs)?;
        hir_maker::HirMaker::new(index).convert_program(ast)
    }
}

#[derive(Debug, PartialEq)]
pub struct SkClass {
    pub fullname: ClassFullname,
    pub instance_ty: TermTy,
    pub methods: Vec<SkMethod>,
}
impl SkClass {
    pub fn class_ty(&self) -> TermTy {
        self.instance_ty.meta_ty()
    }
}

#[derive(Debug, PartialEq)]
pub struct SkMethod {
    pub signature: MethodSignature,
    pub body: SkMethodBody,
}

#[derive(Debug, PartialEq)]
pub enum SkMethodBody {
    ShiikaMethodBody {
        exprs: HirExpressions
    },
    RustMethodBody {
        gen: GenMethodBody // TODO: better name
    }
}
pub type GenMethodBody = fn(code_gen: &crate::code_gen::CodeGen,
                function: &inkwell::values::FunctionValue) -> Result<(), crate::error::Error>;

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
    HirIfExpression {
        cond_expr: Box<HirExpression>,
        then_expr: Box<HirExpression>,
        else_expr: Box<HirExpression>,
    },
    HirMethodCall {
        receiver_expr: Box<HirExpression>,
        method_fullname: MethodFullname,
        arg_exprs: Vec<HirExpression>,
    },
    HirArgRef {
        idx: usize,
    },
    HirConstRef {
        fullname: ConstFullname,
    },
    HirSelfExpression,
    HirFloatLiteral {
        value: f32,
    },
    HirDecimalLiteral {
        value: i32,
    },
    HirNop  // For else-less if expr
}

impl Hir {
    pub fn if_expression(ty: TermTy,
                         cond_hir: HirExpression,
                         then_hir: HirExpression,
                         else_hir: HirExpression) -> HirExpression {
        HirExpression {
            ty: ty,
            node: HirExpressionBase::HirIfExpression {
                cond_expr: Box::new(cond_hir),
                then_expr: Box::new(then_hir),
                else_expr: Box::new(else_hir),
            }
        }
    }

    pub fn method_call(result_ty: TermTy, receiver_hir: HirExpression, method_fullname: MethodFullname, arg_hirs: Vec<HirExpression>) -> HirExpression {
        HirExpression {
            ty: result_ty,
            node: HirExpressionBase::HirMethodCall {
                receiver_expr: Box::new(receiver_hir),
                method_fullname: method_fullname,
                arg_exprs: arg_hirs,
            }
        }
    }

    // REFACTOR: Remove `hir_`
    pub fn hir_arg_ref(ty: TermTy, idx: usize) -> HirExpression {
        HirExpression {
            ty: ty,
            node: HirExpressionBase::HirArgRef { idx: idx },
        }
    }

    pub fn const_ref(ty: TermTy, fullname: ConstFullname) -> HirExpression {
        HirExpression {
            ty: ty,
            node: HirExpressionBase::HirConstRef { fullname },
        }
    }

    pub fn self_expression(ty: TermTy) -> HirExpression {
        HirExpression {
            ty: ty,
            node: HirExpressionBase::HirSelfExpression,
        }
    }

    pub fn float_literal(value: f32) -> HirExpression {
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
    
    pub fn nop() -> HirExpression {
        HirExpression {
            ty: ty::raw(" NOP "), // must not be used
            node: HirExpressionBase::HirNop,
        }
    }
}

pub fn create_signature(class_fullname: String, sig: &ast::MethodSignature) -> MethodSignature {
    let name = sig.name.clone();
    let fullname = MethodFullname(class_fullname + "#" + &sig.name.0);
    let ret_ty = convert_typ(&sig.ret_typ);
    let params = sig.params.iter().map(|param|
        MethodParam { name: param.name.to_string(), ty: convert_typ(&param.typ) }
    ).collect();

    MethodSignature { name, fullname, ret_ty, params }
}

fn convert_typ(typ: &ast::Typ) -> TermTy {
    ty::raw(&typ.name)
}
