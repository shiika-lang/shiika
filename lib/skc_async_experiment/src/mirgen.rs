mod collect_allocs;
use crate::build;
use crate::mir;
use shiika_core::ty::TermTy;
use skc_hir::{HirExpression, SkMethod};
mod constants;
use crate::names::FunctionName;
use anyhow::Result;
use skc_hir::{HirExpressionBase, MethodParam, MethodSignature, SkMethodBody, SkTypes};

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
            str_literals: &uni.hir.str_literals,
        };

        for (_, ms) in uni.hir.sk_methods {
            for m in ms {
                log::debug!("Converting method: {}", &m.signature);
                funcs.push(c.convert_method(m));
            }
        }

        let consts = uni.hir.const_inits.into_iter().map(|e| {
            let HirExpressionBase::HirConstAssign { fullname, rhs } = e.node else {
                panic!("Expected HirConstAssign, got {:?}", e);
            };
            (fullname, c.convert_expr(*rhs))
        });
        funcs.push(constants::create_const_init_func(
            uni.package_name.as_ref(),
            consts.collect(),
        ));

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
    str_literals: &'a Vec<String>,
}

impl<'a> Compiler<'a> {
    fn convert_method(&self, method: SkMethod) -> mir::Function {
        let orig_params = if let SkMethodBody::New { initializer, .. } = &method.body {
            // REFACTOR: method.signature.params should be available for this case too
            if let Some(ini) = initializer {
                ini.params.clone()
            } else {
                vec![]
            }
        } else {
            method.signature.params.clone()
        };
        let mut params = orig_params
            .into_iter()
            .map(|x| convert_param(x))
            .collect::<Vec<_>>();
        params.insert(
            0,
            mir::Param {
                ty: convert_ty(method.signature.receiver_ty()),
                name: "self".to_string(),
            },
        );
        let body_stmts = self.convert_method_body(method.body);
        let allocs = collect_allocs::run(&body_stmts);
        let body_stmts = self.insert_allocs(allocs, body_stmts);
        mir::Function {
            asyncness: method.signature.asyncness.clone().into(),
            name: method.signature.fullname.clone().into(),
            params,
            ret_ty: convert_ty(method.signature.ret_ty.clone()),
            body_stmts,
            sig: Some(method.signature),
        }
    }

    fn insert_allocs(
        &self,
        allocs: Vec<(String, mir::Ty)>,
        stmts: mir::TypedExpr,
    ) -> mir::TypedExpr {
        let mut stmts_vec = mir::expr::into_exprs(stmts);
        let mut new_stmts = vec![];
        for (name, ty) in allocs {
            new_stmts.push(mir::Expr::alloc(name, ty));
        }
        new_stmts.extend(stmts_vec.drain(..));
        mir::Expr::exprs(new_stmts)
    }

    fn convert_method_body(&self, body: SkMethodBody) -> mir::TypedExpr {
        match body {
            SkMethodBody::Normal { exprs } => self.convert_expr(exprs),
            SkMethodBody::RustLib => {
                panic!("RustLib method cannot be converted to MIR")
            }
            SkMethodBody::New {
                classname,
                initializer,
                arity: _,
                const_is_obj: _,
            } => self.create_new_body(classname.to_ty(), initializer),
            SkMethodBody::Getter {
                idx: _,
                name: _,
                ty: _,
                self_ty: _,
            } => {
                todo!();
                //let self_expr = mir::Expr::lvar_ref("self", convert_ty(ty.clone()));
                //mir::Expr::ivar_ref(self_expr, idx, convert_ty(ty))
            }
            SkMethodBody::Setter {
                idx: _,
                name: _,
                ty: _,
                self_ty: _,
            } => {
                todo!();
                //let self_expr = mir::Expr::lvar_ref("self", convert_ty(ty.clone()));
                //let value_expr = mir::Expr::arg_ref(1, "?", convert_ty(ty.clone()));
                //mir::Expr::ivar_set(self_expr, idx, value_expr, convert_ty(ty))
            }
        }
    }

    fn convert_expr(&self, expr: HirExpression) -> mir::TypedExpr {
        use skc_hir::HirExpressionBase;
        let result_ty = convert_ty(expr.ty.clone());
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
                // REFACTOR: embed string directly
                call_string_new(self.str_literals[idx].clone())
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
                mir::Expr::const_ref(mir::mir_const_name(fullname), convert_ty(expr.ty))
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
            HirExpressionBase::HirLVarAssign { name, rhs } => {
                let mir_rhs = self.convert_expr(*rhs);
                (mir::Expr::LVarSet(name, Box::new(mir_rhs)), result_ty)
            }
            HirExpressionBase::HirIVarAssign { name, idx, .. } => {
                todo!("Handle ivar assign: {} at index {} with rhs", name, idx)
            }
            HirExpressionBase::HirConstAssign { fullname, rhs } => {
                mir::Expr::const_set(mir::mir_const_name(fullname), self.convert_expr(*rhs))
            }
            HirExpressionBase::HirMethodCall {
                receiver_expr,
                method_fullname,
                arg_exprs,
                is_virtual,
                ..
            } => {
                let receiver_ty = receiver_expr.ty.clone();
                let mir_receiver = self.convert_expr(*receiver_expr);
                let method_name = &method_fullname.first_name;

                let fun_ty = {
                    let mut param_tys = arg_exprs
                        .iter()
                        .map(|e| e.ty.clone().into())
                        .collect::<Vec<_>>();
                    param_tys.insert(0, convert_ty(method_fullname.type_name.to_ty()));
                    mir::FunTy::new(mir::Asyncness::Unknown, param_tys, expr.ty.clone().into())
                };

                let func_ref = if is_virtual {
                    // For now, assume all method calls are virtual calls
                    let method_idx = self
                        .lookup_vtable(&receiver_ty, method_name)
                        .unwrap_or_else(|| {
                            panic!("Method not found in vtable: {}", method_fullname)
                        });

                    mir::Expr::vtable_ref(
                        mir_receiver.clone(),
                        method_idx,
                        method_name.0.clone(),
                        fun_ty,
                    )
                } else {
                    mir::Expr::func_ref(method_fullname.into(), fun_ty)
                };
                let mut mir_args: Vec<mir::TypedExpr> = arg_exprs
                    .into_iter()
                    .map(|arg| self.convert_expr(arg))
                    .collect();
                mir_args.insert(0, mir_receiver);

                (mir::Expr::FunCall(Box::new(func_ref), mir_args), result_ty)
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
            HirExpressionBase::HirIfExpression {
                cond_expr,
                then_exprs,
                else_exprs,
                ..
            } => mir::Expr::if_(
                self.convert_expr(*cond_expr),
                self.convert_expr(*then_exprs),
                self.convert_expr(*else_exprs),
            ),
            HirExpressionBase::HirMatchExpression { .. } => {
                todo!("Handle match expression")
            }
            HirExpressionBase::HirWhileExpression {
                cond_expr,
                body_exprs,
                ..
            } => mir::Expr::while_(
                self.convert_expr(*cond_expr),
                self.convert_expr(*body_exprs),
            ),
            HirExpressionBase::HirBreakExpression { .. } => {
                todo!("Handle break expression")
            }
            HirExpressionBase::HirReturnExpression { arg, .. } => {
                mir::Expr::return_(self.convert_expr(*arg))
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
            HirExpressionBase::HirBitCast { expr: e } => mir::Expr::cast(
                mir::expr::CastType::Force(expr.ty.into()),
                self.convert_expr(*e),
            ),
            HirExpressionBase::HirClassLiteral { fullname, .. } => {
                mir::Expr::create_type_object(fullname.to_ty())
            }
            HirExpressionBase::HirParenthesizedExpr { exprs } => {
                let mir_exprs = exprs
                    .into_iter()
                    .map(|expr| self.convert_expr(expr))
                    .collect();
                mir::Expr::exprs(mir_exprs)
            }
            HirExpressionBase::HirDefaultExpr => {
                todo!("Handle default expr")
            }
            HirExpressionBase::HirIsOmittedValue { .. } => {
                todo!("Handle is omitted value")
            }
        }
    }

    fn create_new_body(
        &self,
        instance_ty: TermTy,
        initializer: Option<MethodSignature>,
    ) -> mir::TypedExpr {
        let mut exprs = vec![];
        let tmp_name = "tmp";
        exprs.push(mir::Expr::alloc(tmp_name, instance_ty.clone().into()));
        exprs.push(mir::Expr::lvar_set(
            tmp_name,
            mir::Expr::create_object(instance_ty.clone()),
        ));
        if let Some(ini_sig) = initializer {
            let call_initialize = {
                let mut args: Vec<_> = ini_sig
                    .clone()
                    .params
                    .into_iter()
                    .enumerate()
                    .map(|(i, param)| mir::Expr::arg_ref(i + 1, param.name, param.ty.into()))
                    .collect();
                args.insert(
                    0,
                    mir::Expr::lvar_ref(tmp_name.to_string(), instance_ty.clone().into()),
                );
                let ini_func =
                    mir::Expr::func_ref(ini_sig.fullname.clone().into(), build_fun_ty(&ini_sig));
                mir::Expr::fun_call(ini_func, args)
            };
            exprs.push(call_initialize);
        }
        exprs.push(mir::Expr::return_(mir::Expr::lvar_ref(
            tmp_name.to_string(),
            instance_ty.into(),
        )));

        mir::Expr::exprs(exprs)
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

    fn lookup_vtable(
        &self,
        ty: &TermTy,
        method_name: &shiika_core::names::MethodFirstname,
    ) -> Option<usize> {
        self.vtables
            .find(ty, method_name)
            .or_else(|| self.imported_vtables.find(ty, method_name))
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

fn convert_param(param: MethodParam) -> mir::Param {
    mir::Param {
        ty: convert_ty(param.ty),
        name: param.name,
    }
}

fn convert_ty(ty: TermTy) -> mir::Ty {
    ty.into()
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
