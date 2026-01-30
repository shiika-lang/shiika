mod constants;
mod prepare_asyncness;
mod wtables;
use crate::build;
use crate::codegen;
use crate::mir;
use crate::names::FunctionName;
use anyhow::Result;
use shiika_core::names::ConstFullname;
use shiika_core::ty;
use shiika_core::ty::TermTy;
use skc_hir::{HirExpression, HirLVars, SkMethod};
use skc_hir::{HirExpressionBase, MethodParam, MethodSignature, SkMethodBody, SkTypes};

pub fn run(
    mut uni: build::CompilationUnit,
    target: &build::CompileTarget,
) -> Result<mir::CompilationUnit> {
    log::debug!("Preparing asyncness");
    prepare_asyncness::run(
        &mut uni.hir.sk_types,
        &mut uni.hir.sk_methods,
        &uni.imports.sk_types,
    );

    log::debug!("Building VTables");
    let vtables = skc_mir::VTables::build(&uni.hir.sk_types, &uni.imports);

    let classes = convert_classes(&uni);

    let externs = {
        let mut externs = codegen::prelude::core_externs();
        externs.extend(convert_externs(&uni.imports.sk_types));
        for sk_type in uni.hir.sk_types.types.values() {
            for sig in sk_type.base().method_sigs.iter() {
                if sig.is_rust {
                    externs.push(build_extern(sig));
                }
            }
        }
        if let build::CompileTargetDetail::Bin { total_deps, .. } = &target.detail {
            externs.extend(constants::const_init_externs(total_deps));
        }
        externs
    };

    let funcs = {
        let mut funcs = vec![];
        let mut c = Compiler {
            vtables: &vtables,
            imported_vtables: &uni.imports.vtables,
            str_literals: &uni.hir.str_literals,
            lambda_funcs: vec![],
        };

        funcs.extend(const_init_funcs(&uni, &mut c));
        if target.is_bin() {
            funcs.extend(wtables::inserter_funcs(&uni.hir.sk_types));
        }

        for (_, ms) in uni.hir.sk_methods {
            for m in ms {
                let signature = uni.hir.sk_types.get_sig(&m.fullname).unwrap();
                log::debug!("Converting method: {}", signature);
                funcs.push(c.convert_method(m, &uni.hir.sk_types));
            }
        }

        log::debug!("Converting top exprs");
        let main_exprs = uni.hir.main_exprs;
        if let build::CompileTargetDetail::Bin { total_deps, .. } = &target.detail {
            funcs.push(c.create_user_main());
            funcs.push(c.create_user_main_inner(main_exprs, total_deps));
        } else {
            if main_exprs.len() > 0 {
                panic!("Top level expressions are not allowed in library");
            }
        }

        // Add generated lambda functions
        funcs.extend(c.lambda_funcs);

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

fn const_init_funcs(uni: &build::CompilationUnit, c: &mut Compiler) -> Vec<mir::Function> {
    let consts = uni.hir.const_inits.iter().map(|e| {
        let HirExpressionBase::HirConstAssign { fullname, rhs } = &e.node else {
            panic!("Expected HirConstAssign, got {:?}", e);
        };
        (fullname.clone(), c.convert_expr(rhs.as_ref().clone()))
    });

    constants::create_const_init_funcs(uni.package_name.as_ref(), consts.collect())
}

struct Compiler<'a> {
    vtables: &'a skc_mir::VTables,
    imported_vtables: &'a skc_mir::VTables,
    str_literals: &'a Vec<String>,
    /// Collects generated lambda functions
    lambda_funcs: Vec<mir::Function>,
}

impl<'a> Compiler<'a> {
    fn convert_method(&mut self, method: SkMethod, sk_types: &SkTypes) -> mir::Function {
        let signature = sk_types.get_sig(&method.fullname).unwrap();
        let orig_params = if let SkMethodBody::New { initializer, .. } = &method.body {
            // REFACTOR: method.signature.params should be available for this case too
            if let Some(ini) = initializer {
                ini.params.clone()
            } else {
                vec![]
            }
        } else {
            signature.params.clone()
        };
        let mut params = orig_params
            .into_iter()
            .map(|x| convert_param(x))
            .collect::<Vec<_>>();
        params.insert(
            0,
            mir::Param {
                ty: convert_ty(signature.receiver_ty()),
                name: "self".to_string(),
            },
        );
        let body_stmts = self.convert_method_body(method.body, &signature);
        mir::Function {
            asyncness: signature.asyncness.clone().into(),
            name: method.fullname.clone().into(),
            params,
            ret_ty: convert_ty(signature.ret_ty.clone()),
            body_stmts,
            sig: Some(signature.clone()),
            lvar_count: None,
        }
    }

    fn convert_method_body(
        &mut self,
        body: SkMethodBody,
        signature: &MethodSignature,
    ) -> mir::TypedExpr {
        match body {
            SkMethodBody::Normal { exprs } => self.convert_expr(exprs),
            SkMethodBody::RustLib => {
                panic!("RustLib method cannot be converted to MIR")
            }
            SkMethodBody::New {
                classname: _,
                initializer,
                arity: _,
                const_is_obj: _,
            } => self.create_new_body(signature.ret_ty.clone(), initializer),
            SkMethodBody::Getter {
                idx,
                name,
                ty,
                self_ty,
            } => {
                let v = mir::Expr::ivar_ref(self.compile_self_expr(self_ty), idx, name, ty.into());
                mir::Expr::return_(v)
            }
            SkMethodBody::Setter {
                idx,
                name,
                ty,
                self_ty,
            } => {
                let self_expr = self.compile_self_expr(self_ty);
                let value_expr = mir::Expr::arg_ref(1, name.clone(), ty.clone().into());
                mir::Expr::exprs(vec![
                    mir::Expr::ivar_set(self_expr.clone(), idx, value_expr.clone(), name.clone()),
                    mir::Expr::return_(mir::Expr::ivar_ref(
                        self_expr.clone(),
                        idx,
                        name,
                        ty.into(),
                    )),
                ])
            }
        }
    }

    fn convert_expr(&mut self, expr: HirExpression) -> mir::TypedExpr {
        use skc_hir::HirExpressionBase;
        let result_ty = convert_ty(expr.ty.clone());
        match expr.node {
            HirExpressionBase::HirBooleanLiteral { value } => {
                let b = if value {
                    mir::PseudoVar::True
                } else {
                    mir::PseudoVar::False
                };
                mir::Expr::pseudo_var(b, mir::Ty::raw("Bool"))
            }
            HirExpressionBase::HirStringLiteral { idx } => {
                mir::Expr::string_literal(self.str_literals[idx].clone())
            }
            HirExpressionBase::HirDecimalLiteral { value } => mir::Expr::number(value),
            HirExpressionBase::HirFloatLiteral { value } => {
                todo!("Handle float literal: {}", value)
            }
            HirExpressionBase::HirArrayLiteral { elem_exprs } => {
                let mir_elements: Vec<mir::TypedExpr> = elem_exprs
                    .into_iter()
                    .map(|e| self.convert_expr(e))
                    .collect();
                let native_array_expr = (
                    mir::Expr::CreateNativeArray(mir_elements.clone()),
                    mir::Ty::Ptr,
                );

                // Call Meta:Array#new with the native array and element count
                let element_count = mir_elements.len();
                let count_expr = mir::Expr::raw_i64(element_count as i64);
                let func_name = FunctionName::method("Meta:Array", "new");
                let fun_ty = mir::FunTy::new(
                    mir::Asyncness::Unknown,
                    vec![mir::Ty::meta("Array"), mir::Ty::Ptr, mir::Ty::Int64],
                    result_ty,
                );
                let func_ref = mir::Expr::func_ref(func_name, fun_ty.into());
                let the_array =
                    mir::Expr::const_ref(ConstFullname::toplevel("Array"), mir::Ty::meta("Array"));

                mir::Expr::fun_call(func_ref, vec![the_array, native_array_expr, count_expr])
            }
            HirExpressionBase::HirSelfExpression => self.compile_self_expr(expr.ty),
            HirExpressionBase::HirLVarRef { name } => {
                mir::Expr::lvar_ref(name, convert_ty(expr.ty))
            }
            HirExpressionBase::HirArgRef { idx, is_lambda } => {
                // In methods, +1 for the receiver (self). In lambdas, no receiver.
                let actual_idx = if is_lambda { idx } else { idx + 1 };
                mir::Expr::arg_ref(actual_idx, "?", convert_ty(expr.ty))
            }
            HirExpressionBase::HirIVarRef { name, idx, self_ty } => {
                mir::Expr::ivar_ref(self.compile_self_expr(self_ty), idx, name, expr.ty.into())
            }
            HirExpressionBase::HirConstRef { fullname } => {
                mir::Expr::const_ref(fullname, convert_ty(expr.ty))
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
            HirExpressionBase::HirLVarDecl {
                name,
                rhs,
                readonly,
            } => {
                let mir_rhs = self.convert_expr(*rhs);
                (
                    mir::Expr::LVarDecl(name, Box::new(mir_rhs), !readonly),
                    result_ty,
                )
            }
            HirExpressionBase::HirLVarAssign { name, rhs } => {
                let mir_rhs = self.convert_expr(*rhs);
                (mir::Expr::LVarSet(name, Box::new(mir_rhs)), result_ty)
            }
            HirExpressionBase::HirIVarAssign {
                name,
                idx,
                rhs,
                self_ty,
                ..
            } => {
                let self_expr = self.compile_self_expr(self_ty);
                let mir_rhs = self.convert_expr(*rhs);
                mir::Expr::ivar_set(self_expr, idx, mir_rhs, name)
            }
            HirExpressionBase::HirConstAssign { fullname, rhs } => {
                mir::Expr::const_set(fullname, self.convert_expr(*rhs))
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
            HirExpressionBase::HirModuleMethodCall {
                receiver_expr,
                module_fullname,
                method_name,
                method_idx,
                arg_exprs,
                ..
            } => {
                let receiver_ty = receiver_expr.ty.clone();
                let mir_receiver = self.convert_expr(*receiver_expr);

                let func_ref = {
                    let fun_ty = {
                        let mut param_tys = arg_exprs
                            .iter()
                            .map(|e| e.ty.clone().into())
                            .collect::<Vec<_>>();
                        param_tys.insert(0, convert_ty(receiver_ty));
                        mir::FunTy::new(mir::Asyncness::Unknown, param_tys, expr.ty.clone().into())
                    };

                    mir::Expr::wtable_ref(
                        mir_receiver.clone(),
                        module_fullname.clone(),
                        method_idx,
                        method_name.0.clone(),
                        fun_ty,
                    )
                };

                let mut mir_args: Vec<mir::TypedExpr> = arg_exprs
                    .into_iter()
                    .map(|arg| self.convert_expr(arg))
                    .collect();
                mir_args.insert(0, mir_receiver);

                let result_ty = convert_ty(expr.ty.clone());
                (mir::Expr::FunCall(Box::new(func_ref), mir_args), result_ty)
            }
            HirExpressionBase::HirLambdaInvocation {
                lambda_expr,
                arg_exprs,
            } => {
                let mir_lambda = self.convert_expr(*lambda_expr);
                let mir_args: Vec<mir::TypedExpr> = arg_exprs
                    .into_iter()
                    .map(|arg| self.convert_expr(arg))
                    .collect();
                mir::Expr::fun_call(mir_lambda, mir_args)
            }
            HirExpressionBase::HirLambdaExpr {
                name,
                params,
                exprs,
                captures,
                lvars,
                ret_ty,
                has_break,
            } => {
                // MVP: reject captures and break
                if !captures.is_empty() {
                    todo!("Lambda captures not yet supported")
                }
                if has_break {
                    todo!("Lambda break not yet supported")
                }

                // Generate the lambda function and store it
                let lambda_func =
                    self.create_lambda_function(&name, &params, &exprs, &lvars, &ret_ty);
                let func_name = lambda_func.name.clone();
                self.lambda_funcs.push(lambda_func);

                // Build function type (Async for all lambdas)
                let param_tys: Vec<mir::Ty> =
                    params.iter().map(|p| convert_ty(p.ty.clone())).collect();
                let fun_ty = mir::FunTy::new(mir::Asyncness::Async, param_tys, convert_ty(ret_ty));

                // Return FuncRef to the lambda function
                mir::Expr::func_ref(func_name, fun_ty)
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

    fn compile_self_expr(&self, ty: TermTy) -> mir::TypedExpr {
        // In MIR, 'self' is always the first argument (index 0)
        mir::Expr::arg_ref(0, "self", convert_ty(ty))
    }

    fn create_lambda_function(
        &mut self,
        name: &str,
        params: &[MethodParam],
        exprs: &HirExpression,
        lvars: &HirLVars,
        ret_ty: &TermTy,
    ) -> mir::Function {
        // Convert params (NO implicit self)
        let mir_params: Vec<mir::Param> = params
            .iter()
            .map(|p| mir::Param {
                ty: convert_ty(p.ty.clone()),
                name: p.name.clone(),
            })
            .collect();

        // Convert body
        let body = self.convert_expr(exprs.clone());

        // Add allocs and return
        let mut stmts: Vec<mir::TypedExpr> = lvars
            .iter()
            .map(|lvar| mir::Expr::alloc(lvar.name.clone(), lvar.ty.clone().into()))
            .collect();

        let body_with_return = if ret_ty.is_void_type() {
            mir::Expr::exprs(vec![body, mir::Expr::return_(mir::Expr::void_const_ref())])
        } else {
            mir::Expr::return_(body)
        };

        stmts.extend(mir::expr::into_exprs(body_with_return));
        let final_body = mir::Expr::exprs(stmts);

        mir::Function {
            // All lambdas are treated as async now
            asyncness: mir::Asyncness::Async,
            name: FunctionName::Generated(name.to_string()),
            params: mir_params,
            ret_ty: convert_ty(ret_ty.clone()),
            body_stmts: final_body,
            sig: None,
        }
    }

    fn create_new_body(
        &self,
        instance_ty: TermTy,
        initializer: Option<MethodSignature>,
    ) -> mir::TypedExpr {
        let mut exprs = vec![];
        let tmp_name = "tmp";
        exprs.push(mir::Expr::lvar_decl(
            tmp_name,
            mir::Expr::create_object(instance_ty.clone()),
            false,
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
                let receiver = {
                    let mut r =
                        mir::Expr::lvar_ref(tmp_name.to_string(), instance_ty.clone().into());
                    let defined_type = ini_sig.fullname.type_name.clone();
                    if instance_ty.fullname != defined_type {
                        r = mir::Expr::cast(mir::CastType::Upcast(defined_type.to_ty().into()), r);
                    }
                    r
                };
                args.insert(0, receiver);
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

    /// Creates the user main function that creates the toplevel main object and calls main_inner
    fn create_user_main(&self) -> mir::Function {
        let mut body_stmts = vec![];
        body_stmts.push(mir::Expr::fun_call(
            mir::Expr::func_ref(
                mir::main_function_inner_name(),
                mir::FunTy::new(
                    mir::Asyncness::Unknown,
                    vec![mir::Ty::raw("Object")],
                    mir::Ty::raw("Int"),
                ),
            ),
            vec![mir::Expr::create_object(ty::raw("Object"))],
        ));
        body_stmts.push(mir::Expr::return_(mir::Expr::number(0)));
        mir::Function {
            asyncness: mir::Asyncness::Unknown,
            name: mir::main_function_name(),
            params: vec![],
            ret_ty: mir::Ty::raw("Int"),
            body_stmts: mir::Expr::exprs(body_stmts),
            sig: None,
            lvar_count: None,
        }
    }

    /// Creates the main_inner function that contains top-level expressions
    fn create_user_main_inner(
        &mut self,
        top_exprs: Vec<HirExpression>,
        total_deps: &[String],
    ) -> mir::Function {
        let mut body_stmts = vec![];
        body_stmts.extend(constants::call_all_const_inits(total_deps));
        body_stmts.push(wtables::call_main_inserter());
        body_stmts.extend(top_exprs.into_iter().map(|expr| self.convert_expr(expr)));
        body_stmts.push(mir::Expr::return_(mir::Expr::number(0)));
        mir::Function {
            asyncness: mir::Asyncness::Unknown,
            name: mir::main_function_inner_name(),
            params: vec![mir::Param::new(mir::Ty::raw("Object"), "self")],
            ret_ty: mir::Ty::raw("Int"),
            body_stmts: mir::Expr::exprs(body_stmts),
            sig: None,
            lvar_count: None,
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
