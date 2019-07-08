mod hir_maker;
mod index;
use crate::ast;
use crate::ty;
use crate::ty::*;

pub struct Hir {
    pub sk_classes: Vec<SkClass>,
    pub main_stmts: Vec<HirStatement>,
}
impl Hir {
    pub fn from_ast(ast: ast::Program, stdlib: &Vec<SkClass>) -> Result<Hir, crate::error::Error> {
        let index = index::Index::new(stdlib, &ast.toplevel_defs)?;
        hir_maker::HirMaker::new(index).convert_program(ast)
    }
}

#[derive(Debug, PartialEq)]
pub struct SkClass {
    pub fullname: String,
    pub methods: Vec<SkMethod>,
}
impl SkClass {
    pub fn instance_ty(&self) -> TermTy {
        ty::raw(&self.fullname)
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
        stmts: Vec<HirStatement>
    },
    RustMethodBody {
        gen: GenMethodBody // TODO: better name
    }
}
pub type GenMethodBody = fn(code_gen: &crate::code_gen::CodeGen,
                function: &inkwell::values::FunctionValue) -> Result<(), crate::error::Error>;

#[derive(Debug, PartialEq)]
pub enum HirStatement {
    HirExpressionStatement {
        expr: HirExpression
    }
}

#[derive(Debug, PartialEq)]
pub struct HirExpression {
    pub ty: TermTy,
    pub node: HirExpressionBase,
}
impl HirExpression {
    pub fn to_hir_statement(self) -> HirStatement {
        HirStatement::HirExpressionStatement { expr: self }
    }
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

    // TODO: get self as argument
    pub fn self_expression() -> HirExpression {
        HirExpression {
            ty: ty::raw("Object"),
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
    let name = MethodName(sig.name.clone());
    let fullname = MethodFullname(class_fullname + "#" + &sig.name);
    let ret_ty = convert_typ(&sig.ret_typ);
    let params = sig.params.iter().map(|param|
        MethodParam { name: param.name.to_string(), ty: convert_typ(&param.typ) }
    ).collect();

    MethodSignature { name, fullname, ret_ty, params }
}

fn convert_typ(typ: &ast::Typ) -> TermTy {
    ty::raw(&typ.name)
}
