use crate::build;
use crate::mir;
use shiika_core::ty::TermTy;
use skc_hir::HirExpression;
mod constants;
use crate::names::FunctionName;
use anyhow::Result;
use skc_hir::{MethodSignature, SkTypes};

pub fn run(
    uni: build::CompilationUnit,
    target: &build::CompileTarget,
) -> Result<mir::CompilationUnit> {
    log::debug!("Building VTables");
    let vtables = skc_mir::VTables::build(&uni.hir.sk_types, &uni.imports);

    let classes = convert_classes(&uni);

    let externs = {
        let mut externs = convert_externs(&uni.imports.sk_types);
        for method_name in &uni.hir.sk_types.rustlib_methods {
            externs.push(build_extern(
                &uni.hir.sk_types.get_sig(method_name).unwrap(),
            ));
        }
        if let build::CompileTargetDetail::Bin { total_deps, .. } = &target.detail {
            externs.extend(constants::const_init_externs(total_deps));
        }
        externs
    };

    let funcs = {
        let mut funcs = vec![];
        let c = Compiler {
            vtables: &vtables,
            imported_vtables: &uni.imports.vtables,
        };

        log::debug!("Converting top exprs");
        let main_exprs = uni.hir.main_exprs;
        if let build::CompileTargetDetail::Bin { total_deps, .. } = &target.detail {
            funcs.push(c.create_user_main(main_exprs, total_deps));
        } else {
            if main_exprs.len() > 0 {
                panic!("Top level expressions are not allowed in library");
            }
        }
        funcs
    };

    let const_list = uni
        .hir
        .constants
        .iter()
        .map(|(name, ty)| (name.clone(), ty.clone()))
        .collect::<Vec<_>>();

    let program = mir::Program::new(classes, externs, funcs, const_list);
    Ok(mir::CompilationUnit {
        program,
        sk_types: uni.hir.sk_types,
        vtables,
        imported_constants: uni.imports.constants.into_iter().collect(),
        imported_vtables: uni.imports.vtables,
    })
}

struct Compiler<'a> {
    vtables: &'a skc_mir::VTables,
    imported_vtables: &'a skc_mir::VTables,
}

impl<'a> Compiler<'a> {
    fn convert_expr(&self, expr: HirExpression) -> mir::TypedExpr {
        use skc_hir::HirExpressionBase;
        match expr.node {
            HirExpressionBase::HirBooleanLiteral { value } => {
                let b = if value {
                    mir::PseudoVar::True
                } else {
                    mir::PseudoVar::False
                };
                mir::Expr::pseudo_var(b, mir::Ty::Raw("Bool".to_string()))
            }
            HirExpressionBase::HirStringLiteral { idx } => {
                todo!("Handle string literal with idx: {}", idx)
            }
            HirExpressionBase::HirDecimalLiteral { value } => mir::Expr::number(value),
            HirExpressionBase::HirFloatLiteral { value } => {
                todo!("Handle float literal: {}", value)
            }
            HirExpressionBase::HirSelfExpression => {
                // REFACTOR: just get the 0-th arg?
                mir::Expr::pseudo_var(mir::PseudoVar::SelfRef, convert_ty(expr.ty))
            }
            HirExpressionBase::HirLVarRef { name } => {
                mir::Expr::lvar_ref(name, convert_ty(expr.ty))
            }
            HirExpressionBase::HirArgRef { idx } => {
                // +1 for the receiver
                // TODO: Add debug name
                mir::Expr::arg_ref(idx + 1, "?", convert_ty(expr.ty))
            }
            HirExpressionBase::HirIVarRef {
                name,
                idx,
                self_ty: _,
            } => {
                todo!("Handle ivar ref: {} at index {}", name, idx)
            }
            HirExpressionBase::HirConstRef { fullname } => {
                todo!("Handle const ref: {:?}", fullname)
            }
            HirExpressionBase::HirClassTVarRef {
                typaram_ref,
                self_ty: _,
            } => todo!("Handle class tvar ref: {:?}", typaram_ref),
            HirExpressionBase::HirMethodTVarRef {
                typaram_ref,
                n_params: _,
            } => {
                todo!("Handle method tvar ref: {:?}", typaram_ref)
            }
            HirExpressionBase::HirLVarAssign { name, .. } => {
                todo!("Handle lvar assign: {}", name)
            }
            HirExpressionBase::HirIVarAssign { name, idx, .. } => {
                todo!("Handle ivar assign: {} at index {}", name, idx)
            }
            HirExpressionBase::HirConstAssign { fullname, .. } => {
                todo!("Handle const assign: {:?}", fullname)
            }
            HirExpressionBase::HirMethodCall {
                method_fullname, ..
            } => {
                todo!("Handle method call: {:?}", method_fullname)
            }
            HirExpressionBase::HirModuleMethodCall { method_name, .. } => {
                todo!("Handle module method call: {:?}", method_name)
            }
            HirExpressionBase::HirLambdaInvocation { .. } => {
                todo!("Handle lambda invocation")
            }
            HirExpressionBase::HirLambdaExpr { .. } => {
                todo!("Handle lambda expr")
            }
            HirExpressionBase::HirIfExpression { .. } => {
                todo!("Handle if expression")
            }
            HirExpressionBase::HirMatchExpression { .. } => {
                todo!("Handle match expression")
            }
            HirExpressionBase::HirWhileExpression { .. } => {
                todo!("Handle while expression")
            }
            HirExpressionBase::HirBreakExpression { .. } => {
                todo!("Handle break expression")
            }
            HirExpressionBase::HirReturnExpression { .. } => {
                todo!("Handle return expression")
            }
            HirExpressionBase::HirLogicalNot { .. } => {
                todo!("Handle logical not")
            }
            HirExpressionBase::HirLogicalAnd { .. } => {
                todo!("Handle logical and")
            }
            HirExpressionBase::HirLogicalOr { .. } => {
                todo!("Handle logical or")
            }
            HirExpressionBase::HirLambdaCaptureRef { idx, .. } => {
                todo!("Handle lambda capture ref: {}", idx)
            }
            HirExpressionBase::HirLambdaCaptureWrite { cidx, .. } => {
                todo!("Handle lambda capture write: {}", cidx)
            }
            HirExpressionBase::HirBitCast { .. } => {
                todo!("Handle bit cast")
            }
            HirExpressionBase::HirClassLiteral { fullname, .. } => {
                todo!("Handle class literal: {:?}", fullname)
            }
            HirExpressionBase::HirParenthesizedExpr { exprs } => {
                todo!("Handle parenthesized expr with {} expressions", exprs.len())
            }
            HirExpressionBase::HirDefaultExpr => {
                todo!("Handle default expr")
            }
            HirExpressionBase::HirIsOmittedValue { .. } => {
                todo!("Handle is omitted value")
            }
        }
    }

    fn create_user_main(
        &self,
        top_exprs: Vec<HirExpression>,
        total_deps: &[String],
    ) -> mir::Function {
        let mut body_stmts = vec![];
        body_stmts.extend(constants::call_all_const_inits(total_deps));
        body_stmts.extend(top_exprs.into_iter().map(|expr| self.convert_expr(expr)));
        body_stmts.push(mir::Expr::return_(mir::Expr::number(0)));
        mir::Function {
            asyncness: mir::Asyncness::Unknown,
            name: mir::main_function_name(),
            params: vec![],
            ret_ty: mir::Ty::Raw("Int".to_string()),
            body_stmts: mir::Expr::exprs(body_stmts),
            sig: None,
        }
    }
}

fn convert_classes(uni: &build::CompilationUnit) -> Vec<mir::MirClass> {
    let mut v: Vec<_> = uni
        .hir
        .sk_types
        .sk_classes()
        .map(|sk_class| {
            let ivars = sk_class
                .ivars_ordered()
                .iter()
                .map(|ivar| (ivar.name.clone(), convert_ty(ivar.ty.clone())))
                .collect();
            mir::MirClass {
                name: sk_class.fullname().0.clone(),
                ivars,
            }
        })
        .collect();
    for c in uni.imports.sk_types.sk_classes() {
        let ivars = c
            .ivars_ordered()
            .iter()
            .map(|ivar| (ivar.name.clone(), convert_ty(ivar.ty.clone())))
            .collect();
        v.push(mir::MirClass {
            name: c.fullname().0.clone(),
            ivars,
        });
    }
    v
}

fn convert_ty(ty: TermTy) -> mir::Ty {
    match ty.fn_x_info() {
        Some(tys) => {
            let mut param_tys = tys
                .into_iter()
                .map(|x| convert_ty(x.clone()))
                .collect::<Vec<_>>();
            let ret_ty = param_tys.pop().unwrap();
            mir::Ty::Fun(mir::FunTy {
                asyncness: mir::Asyncness::Unknown,
                param_tys,
                ret_ty: Box::new(ret_ty),
            })
        }
        None => match &ty.fullname.0[..] {
            "Shiika::Internal::Ptr" => mir::Ty::Ptr,
            "Shiika::Internal::Int64" => mir::Ty::Int64,
            _ => mir::Ty::Raw(ty.fullname.0),
        },
    }
}

fn convert_externs(imports: &SkTypes) -> Vec<mir::Extern> {
    imports
        .types
        .values()
        .flat_map(|sk_type| {
            sk_type
                .base()
                .method_sigs
                .unordered_iter()
                .map(|(sig, _)| build_extern(sig))
        })
        .collect()
}

fn build_extern(sig: &MethodSignature) -> mir::Extern {
    mir::Extern {
        name: FunctionName::from_sig(&sig),
        fun_ty: build_fun_ty(sig),
    }
}

fn build_fun_ty(sig: &MethodSignature) -> mir::FunTy {
    let mut param_tys = sig
        .params
        .iter()
        .map(|x| convert_ty(x.ty.clone()))
        .collect::<Vec<_>>();
    param_tys.insert(0, convert_ty(sig.fullname.type_name.to_ty()));
    mir::FunTy::new(
        sig.asyncness.clone().into(),
        param_tys,
        convert_ty(sig.ret_ty.clone()),
    )
}

fn call_string_new(s: String) -> mir::TypedExpr {
    let string_new = mir::Expr::func_ref(
        FunctionName::method("Meta:String", "new"),
        mir::FunTy {
            asyncness: mir::Asyncness::Unknown,
            param_tys: vec![mir::Ty::raw("Meta:String"), mir::Ty::Ptr, mir::Ty::Int64],
            ret_ty: Box::new(mir::Ty::raw("String")),
        },
    );
    let bytesize = s.len() as i64;
    mir::Expr::fun_call(
        string_new,
        vec![
            mir::Expr::const_ref("::String", mir::Ty::raw("Meta:String")),
            mir::Expr::string_ref(s),
            mir::Expr::raw_i64(bytesize),
        ],
    )
}

fn method_func_ref(sig: MethodSignature) -> mir::TypedExpr {
    let fname = FunctionName::from_sig(&sig);
    let fun_ty = build_fun_ty(&sig);
    mir::Expr::func_ref(fname, fun_ty)
}
