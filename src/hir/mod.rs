mod hir_maker;
use std::collections::HashMap;
use crate::ast;
use crate::ty;
use crate::ty::*;

pub struct Hir {
    //pub class_defs: Vec<SkClass>,
    pub main_stmts: Vec<HirStatement>,
}
impl Hir {
    pub fn from_ast(ast: ast::Program, stdlib: &HashMap<String, SkClass>) -> Result<Hir, hir_maker::Error> {
        hir_maker::HirMaker::new(stdlib).convert_program(ast)
    }

    pub fn new(hir_stmts: Vec<HirStatement>) -> Hir {
        Hir { main_stmts: hir_stmts }
    }
}

#[derive(Debug, PartialEq)]
pub struct SkClass {
    pub fullname: String,
    pub methods: HashMap<String, SkMethod>,
}
impl SkClass {
    pub fn find_method(&self, name: &str) -> Option<&SkMethod> {
        self.methods.get(name)
    }

    pub fn instance_ty(&self) -> TermTy {
        ty::raw(&self.fullname)
    }
}

#[derive(Debug, PartialEq)]
pub struct SkMethod {
    pub id: MethodId,
    pub signature: MethodSignature,
    pub body: Option<SkMethodBody>, // None on creation
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
                function: &inkwell::values::FunctionValue) -> Result<(), crate::code_gen::Error>;

#[derive(Debug, PartialEq, Clone)]
pub struct MethodId(pub String);
impl MethodId {
    pub fn llvm_func_name(&self) -> &str {
        &self.0
    }
}

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
        method_id: MethodId,
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

    pub fn method_call(result_ty: TermTy, receiver_hir: HirExpression, method_id: MethodId, arg_hirs: Vec<HirExpression>) -> HirExpression {
        HirExpression {
            ty: result_ty,
            node: HirExpressionBase::HirMethodCall {
                receiver_expr: Box::new(receiver_hir),
                method_id: method_id,
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
            ty: TermTy::TyRaw{ fullname: "NOP".to_string() }, // must not be used
            node: HirExpressionBase::HirNop,
        }
    }
}
