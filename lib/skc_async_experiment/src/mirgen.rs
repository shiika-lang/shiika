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
use skc_hir::visitor::{walk_expr, HirVisitor};
use skc_hir::{HirExpression, HirExpressionBase, HirLVars, SkMethod};
use skc_hir::{HirLambdaCaptureDetail, MethodParam, MethodSignature, SkMethodBody, SkTypes};
use std::collections::{HashMap, HashSet};

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
            cell_vars: HashSet::new(),
            current_fn_class: None,
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
    /// Names of local variables that need Cell wrapping (captured as non-readonly)
    cell_vars: HashSet<String>,
    /// The Fn class name for the current lambda being compiled (e.g. "Fn1")
    current_fn_class: Option<String>,
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
        // Pass 1: collect cell vars before converting body
        self.cell_vars = if let SkMethodBody::Normal { exprs } = &method.body {
            collect_cell_vars(exprs)
        } else {
            HashSet::new()
        };
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
                if self.cell_vars.contains(&name) {
                    let cell = mir::Expr::lvar_ref(name, mir::Ty::Ptr);
                    mir::Expr::cell_get(cell, convert_ty(expr.ty))
                } else {
                    mir::Expr::lvar_ref(name, convert_ty(expr.ty))
                }
            }
            HirExpressionBase::HirArgRef { idx, is_lambda: _ } => {
                // +1 for the receiver (self) in methods, or +1 for $fn in lambdas
                mir::Expr::arg_ref(idx + 1, "?", convert_ty(expr.ty))
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
                if self.cell_vars.contains(&name) {
                    // var y = 5 → var y = cell_new(5)
                    let cell = mir::Expr::cell_new(mir_rhs);
                    (mir::Expr::LVarDecl(name, Box::new(cell), true), result_ty)
                } else {
                    (
                        mir::Expr::LVarDecl(name, Box::new(mir_rhs), !readonly),
                        result_ty,
                    )
                }
            }
            HirExpressionBase::HirLVarAssign { name, rhs } => {
                let mir_rhs = self.convert_expr(*rhs);
                if self.cell_vars.contains(&name) {
                    // y = v → cell_set(y, v); cell_get(y)
                    let cell = mir::Expr::lvar_ref(name.clone(), mir::Ty::Ptr);
                    let cell2 = mir::Expr::lvar_ref(name, mir::Ty::Ptr);
                    mir::Expr::exprs(vec![
                        mir::Expr::cell_set(cell, mir_rhs),
                        mir::Expr::cell_get(cell2, result_ty),
                    ])
                } else {
                    (mir::Expr::LVarSet(name, Box::new(mir_rhs)), result_ty)
                }
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
                let fn_obj = self.convert_expr(*lambda_expr);

                // Extract @func from Fn object (ivar index 0)
                // Give it Ty::Fun so that FunCall knows the function signature
                let param_tys: Vec<mir::Ty> = {
                    // First param is the Fn object itself, then the explicit args
                    let mut tys = vec![fn_obj.1.clone()];
                    tys.extend(arg_exprs.iter().map(|e| convert_ty(e.ty.clone())));
                    tys
                };
                let fun_ty = mir::FunTy::new(mir::Asyncness::Async, param_tys, result_ty.clone());
                let func_ptr = mir::Expr::ivar_ref(
                    fn_obj.clone(),
                    0,
                    "@func".to_string(),
                    mir::Ty::Fun(fun_ty),
                );

                // Build args: [fn_obj, arg0, arg1, ...]
                let mut mir_args = vec![fn_obj];
                mir_args.extend(arg_exprs.into_iter().map(|arg| self.convert_expr(arg)));

                mir::Expr::fun_call(func_ptr, mir_args)
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
                if has_break {
                    todo!("Lambda break not yet supported")
                }

                // Generate the lambda function (with fn_obj as first param)
                let lambda_func =
                    self.create_lambda_function(&name, &params, &captures, &exprs, &lvars, &ret_ty);
                let func_name = lambda_func.name.clone();
                self.lambda_funcs.push(lambda_func);

                // Build captures array
                let capture_values: Vec<mir::TypedExpr> = captures
                    .iter()
                    .map(|cap| self.get_capture_value(cap))
                    .collect();
                let captures_ptr = if capture_values.is_empty() {
                    mir::Expr::raw_i64(0) // null pointer for no captures
                } else {
                    mir::Expr::create_native_array(capture_values)
                };

                // Create Fn object: call Meta:FnN#new(Meta:FnN, func_ptr, captures)
                let fn_class = format!("Fn{}", params.len());
                let fn_ty = convert_ty(expr.ty.clone()); // e.g. Fn1<Int, Void>

                // Build function type for the lambda (fn_obj + explicit params)
                let mut lambda_param_tys: Vec<mir::Ty> = vec![fn_ty.clone()];
                lambda_param_tys.extend(params.iter().map(|p| convert_ty(p.ty.clone())));
                let lambda_fun_ty =
                    mir::FunTy::new(mir::Asyncness::Async, lambda_param_tys, convert_ty(ret_ty));

                // func_ptr: FuncRef with Ty::Fun (as expected by FunCall)
                let func_ptr = mir::Expr::func_ref(func_name, lambda_fun_ty);
                // Cast to Ptr for storing in Fn object's @func ivar
                let func_ptr_as_ptr = mir::Expr::cast(mir::CastType::Force(mir::Ty::Ptr), func_ptr);

                // Call Meta:FnN#new(Meta:FnN, Ptr, Ptr) -> FnN
                let meta_fn_class = format!("Meta:{}", fn_class);
                let new_func_name = FunctionName::method(&meta_fn_class, "new");
                let new_fun_ty = mir::FunTy::new(
                    mir::Asyncness::Unknown,
                    vec![mir::Ty::raw(&meta_fn_class), mir::Ty::Ptr, mir::Ty::Ptr],
                    fn_ty.clone(),
                );
                let new_func_ref = mir::Expr::func_ref(new_func_name, new_fun_ty);
                let meta_fn_obj = mir::Expr::const_ref(
                    ConstFullname::toplevel(&fn_class),
                    mir::Ty::raw(&meta_fn_class),
                );

                mir::Expr::fun_call(
                    new_func_ref,
                    vec![meta_fn_obj, func_ptr_as_ptr, captures_ptr],
                )
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
            HirExpressionBase::HirLambdaCaptureRef { idx, readonly } => {
                // $fn is first arg (index 0)
                let fn_class = self
                    .current_fn_class
                    .as_ref()
                    .expect("[BUG] HirLambdaCaptureRef outside lambda");
                let fn_obj = mir::Expr::arg_ref(0, "$fn", mir::Ty::raw(fn_class));
                // @captures is ivar index 1
                let captures =
                    mir::Expr::ivar_ref(fn_obj, 1, "@captures".to_string(), mir::Ty::Ptr);

                if !readonly {
                    // var capture: read through cell
                    let cell = mir::Expr::native_array_ref(captures, idx, mir::Ty::Ptr);
                    mir::Expr::cell_get(cell, convert_ty(expr.ty))
                } else {
                    // let capture: direct value
                    mir::Expr::native_array_ref(captures, idx, convert_ty(expr.ty))
                }
            }
            HirExpressionBase::HirLambdaCaptureWrite { cidx, rhs } => {
                // $fn is first arg (index 0)
                let fn_class = self
                    .current_fn_class
                    .clone()
                    .expect("[BUG] HirLambdaCaptureWrite outside lambda");
                let result_ty = convert_ty(expr.ty);
                let fn_obj = mir::Expr::arg_ref(0, "$fn", mir::Ty::raw(&fn_class));
                // @captures is ivar index 1
                let captures =
                    mir::Expr::ivar_ref(fn_obj, 1, "@captures".to_string(), mir::Ty::Ptr);
                let cell = mir::Expr::native_array_ref(captures, cidx, mir::Ty::Ptr);
                let value = self.convert_expr(*rhs);
                // cell_set returns Void, but assignment in Shiika returns the value.
                // Re-read the cell to produce the assigned value.
                let fn_obj2 = mir::Expr::arg_ref(0, "$fn", mir::Ty::raw(&fn_class));
                let captures2 =
                    mir::Expr::ivar_ref(fn_obj2, 1, "@captures".to_string(), mir::Ty::Ptr);
                let cell2 = mir::Expr::native_array_ref(captures2, cidx, mir::Ty::Ptr);
                mir::Expr::exprs(vec![
                    mir::Expr::cell_set(cell, value),
                    mir::Expr::cell_get(cell2, result_ty),
                ])
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

    fn get_capture_value(&self, cap: &skc_hir::HirLambdaCapture) -> mir::TypedExpr {
        match &cap.detail {
            HirLambdaCaptureDetail::CaptureLVar { name } => {
                if !cap.readonly {
                    // var lvar: pass cell pointer (lvar already holds a cell due to cell_vars)
                    mir::Expr::lvar_ref(name.clone(), mir::Ty::Ptr)
                } else {
                    // let lvar: pass value directly
                    mir::Expr::lvar_ref(name.clone(), convert_ty(cap.ty.clone()))
                }
            }
            HirLambdaCaptureDetail::CaptureArg { idx } => {
                // Args are captured by value (+1 for self receiver)
                mir::Expr::arg_ref(idx + 1, "captured_arg", convert_ty(cap.ty.clone()))
            }
            _ => todo!("Unsupported capture type: {:?}", cap.detail),
        }
    }

    fn create_lambda_function(
        &mut self,
        name: &str,
        params: &[MethodParam],
        _captures: &[skc_hir::HirLambdaCapture],
        exprs: &HirExpression,
        lvars: &HirLVars,
        ret_ty: &TermTy,
    ) -> mir::Function {
        // First param is the Fn object
        let fn_class = format!("Fn{}", params.len());
        let mut mir_params: Vec<mir::Param> = vec![mir::Param::new(mir::Ty::raw(&fn_class), "$fn")];

        // Then explicit params
        mir_params.extend(params.iter().map(|p| mir::Param {
            ty: convert_ty(p.ty.clone()),
            name: p.name.clone(),
        }));

        // Save and clear cell_vars for the lambda body (lambda has its own scope)
        let saved_cell_vars = std::mem::take(&mut self.cell_vars);
        let saved_fn_class = self.current_fn_class.take();
        // Collect cell vars for the lambda body (nested lambdas)
        self.cell_vars = collect_cell_vars(exprs);
        self.current_fn_class = Some(fn_class.clone());

        // Convert body
        let body = self.convert_expr(exprs.clone());

        // Restore cell_vars and current_fn_class
        self.cell_vars = saved_cell_vars;
        self.current_fn_class = saved_fn_class;

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
            lvar_count: None,
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
        // Pass 1: collect cell vars from top-level expressions
        self.cell_vars = {
            let mut vars = HashSet::new();
            for expr in &top_exprs {
                vars.extend(collect_cell_vars(expr));
            }
            vars
        };
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
    // Collect all classes with their superclass names
    let all_sk_classes: Vec<&skc_hir::SkClass> = uni
        .hir
        .sk_types
        .sk_classes()
        .chain(uni.imports.sk_types.sk_classes())
        .collect();

    // Build name -> ivars map
    let mut ivar_map: HashMap<String, Vec<(String, mir::Ty)>> = HashMap::new();
    for sk_class in &all_sk_classes {
        let ivars = sk_class
            .ivars_ordered()
            .iter()
            .map(|ivar| (ivar.name.clone(), convert_ty(ivar.ty.clone())))
            .collect();
        ivar_map.insert(sk_class.fullname().0.clone(), ivars);
    }

    // Propagate superclass ivars to subclasses that have none
    for sk_class in &all_sk_classes {
        let name = sk_class.fullname().0.clone();
        if ivar_map[&name].is_empty() {
            if let Some(sup) = &sk_class.superclass {
                let super_name = sup.base_fullname().0.clone();
                if let Some(super_ivars) = ivar_map.get(&super_name) {
                    let super_ivars = super_ivars.clone();
                    ivar_map.insert(name, super_ivars);
                }
            }
        }
    }

    all_sk_classes
        .iter()
        .map(|sk_class| {
            let name = sk_class.fullname().0.clone();
            mir::MirClass {
                ivars: ivar_map.remove(&name).unwrap_or_default(),
                name,
            }
        })
        .collect()
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

/// Scan HIR expressions for HirLambdaExpr nodes and collect
/// lvar names that need Cell wrapping (captured as non-readonly).
fn collect_cell_vars(expr: &HirExpression) -> HashSet<String> {
    struct CellVarCollector(HashSet<String>);
    impl<'hir> HirVisitor<'hir> for CellVarCollector {
        fn visit_expr(&mut self, expr: &'hir HirExpression) -> anyhow::Result<()> {
            if let HirExpressionBase::HirLambdaExpr { captures, .. } = &expr.node {
                for cap in captures {
                    if !cap.readonly {
                        if let HirLambdaCaptureDetail::CaptureLVar { name } = &cap.detail {
                            self.0.insert(name.clone());
                        }
                    }
                }
            }
            Ok(())
        }
    }
    let mut collector = CellVarCollector(HashSet::new());
    walk_expr(&mut collector, expr).unwrap();
    collector.0
}
