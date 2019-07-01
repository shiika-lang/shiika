use crate::shiika::ty::*;

pub struct Hir {
    //pub class_defs: Vec<SkClass>,
    //pub main_stmts: Vec<HirStatement>,
    pub hir_expr: HirExpression
}
impl Hir {
    pub fn new(hir_expr: HirExpression) -> Hir {
        Hir { hir_expr }
    }
}

//pub struct SkClass {
//    pub name: String,
//    pub methods: Vec<SkMethod>,
//}

#[derive(Debug, PartialEq)]
pub struct SkMethod {
    pub fullname: String,
    pub signature: MethodSignature,
    pub body: SkMethodBody,
}
impl SkMethod {
    pub fn llvm_func_name(&self) -> &str {
        &self.fullname
    }
}
#[derive(Debug, PartialEq)]
pub enum SkMethodBody {
    ShiikaMethodBody {
        stmts: Vec<HirStatement>
    },
    RustMethodBody {
        // TODO: better name
        gen: fn(code_gen: &crate::shiika::code_gen::CodeGen,
                function: &inkwell::values::FunctionValue) -> Result<(), crate::shiika::code_gen::Error>
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

#[derive(Debug, PartialEq)]
pub enum HirExpressionBase {
    HirIfExpression {
        cond_expr: Box<HirExpression>,
        then_expr: Box<HirExpression>,
        else_expr: Box<HirExpression>,
    },
    HirFloatLiteral {
        value: f32,
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

    pub fn float_literal(value: f32) -> HirExpression {
        HirExpression {
            ty: TermTy::TyRaw{ fullname: "Float".to_string() },
            node: HirExpressionBase::HirFloatLiteral { value }
        }
    }
    
    pub fn nop() -> HirExpression {
        HirExpression {
            ty: TermTy::TyRaw{ fullname: "NOP".to_string() }, // must not be used
            node: HirExpressionBase::HirNop,
        }
    }
}
