mod constants;
mod lambda;
mod pattern_match;
mod prepare_asyncness;
mod wtables;
use crate::build;
use crate::codegen;
use crate::gensym;
use crate::mir;
use crate::names::FunctionName;
use anyhow::Result;
use shiika_core::names::{ConstFullname, MethodFullname, TypeFullname};
use shiika_core::ty;
use shiika_core::ty::TermTy;
use skc_hir::{HirExpression, HirExpressionBase, SkMethod};
use skc_hir::{MethodParam, MethodSignature, SkMethodBody, SkTypes};
use std::collections::HashSet;

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
            lambda: lambda::LambdaContext::new(),
            current_method_sig: None,
            gensym: gensym::Gensym::new(gensym::PREFIX_MIRGEN_RECV),
        };

        funcs.extend(const_init_funcs(&uni, &mut c));
        if target.is_bin() {
            funcs.extend(wtables::inserter_funcs(
                &uni.hir.sk_types,
                &uni.imports.sk_types,
            ));
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
        funcs.extend(c.lambda.lambda_funcs);

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
        imported_sk_types: uni.imports.sk_types,
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
    lambda: lambda::LambdaContext,
    current_method_sig: Option<MethodSignature>,
    gensym: gensym::Gensym,
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
        // Method type parameters are passed as additional Class arguments
        for typaram in &signature.typarams {
            params.push(mir::Param {
                ty: mir::Ty::raw("Class"),
                name: typaram.name.clone(),
            });
        }
        // Pass 1: collect cell vars before converting body
        self.lambda.cell_vars = if let SkMethodBody::Normal { exprs } = &method.body {
            lambda::collect_cell_vars(exprs)
        } else {
            HashSet::new()
        };
        let saved_sig = self.current_method_sig.replace(signature.clone());
        let saved_ret_ty = self
            .lambda
            .current_ret_ty
            .replace(convert_ty(signature.ret_ty.clone()));
        let body_stmts = self.convert_method_body(method.body, &signature);
        self.lambda.current_ret_ty = saved_ret_ty;
        self.current_method_sig = saved_sig;
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

    /// Bind `recv` to a fresh local variable so it can be referenced multiple
    /// times (once for vtable lookup, once as the actual `self` argument)
    /// without re-evaluating its side effects. Returns the binding statement
    /// (to be prepended) plus two references to the bound value. If the
    /// receiver is already a simple lvar/arg/etc. reference, no binding is
    /// emitted and the receiver is just cloned.
    fn bind_receiver(
        &mut self,
        recv: mir::TypedExpr,
    ) -> (Option<mir::TypedExpr>, mir::TypedExpr, mir::TypedExpr) {
        if is_trivial_expr(&recv.0) {
            let r = recv.clone();
            return (None, recv, r);
        }
        let name = self.gensym.new_name();
        let ty = recv.1.clone();
        let decl = mir::Expr::lvar_decl(name.clone(), recv, false);
        let r1 = mir::Expr::lvar_ref(name.clone(), ty.clone());
        let r2 = mir::Expr::lvar_ref(name, ty);
        (Some(decl), r1, r2)
    }

    /// Build a runtime `Class` object expression for a type used as a tyarg
    /// to `Meta:FnN#new` (i.e. lambda type parameters).
    fn build_class_obj_for_tyarg(&self, t: &TermTy) -> mir::TypedExpr {
        use shiika_core::ty::{TyBody, TyParamKind};
        match &t.body {
            TyBody::TyRaw(_) => {
                // Reference the un-specialized class object via const ref.
                let erasure_meta_ty = t.erasure_ty().meta_ty();
                let const_name = erasure_meta_ty.erasure().to_const_fullname();
                let cref = mir::Expr::const_ref(const_name, erasure_meta_ty.into());
                let class_obj = mir::Expr::cast(mir::CastType::Force(mir::Ty::raw("Class")), cref);

                if t.has_type_args() {
                    self.build_specialize_call(class_obj, t.type_args())
                } else {
                    class_obj
                }
            }
            TyBody::TyPara(typaram_ref) => {
                let sig = self
                    .current_method_sig
                    .as_ref()
                    .expect("[BUG] lambda tyarg references typaram outside method scope");
                match typaram_ref.kind {
                    TyParamKind::Method => {
                        let n_params = sig.params.len();
                        let idx = 1 + n_params + typaram_ref.idx;
                        mir::Expr::arg_ref(idx, typaram_ref.name.clone(), mir::Ty::raw("Class"))
                    }
                    TyParamKind::Class => {
                        self.class_tvar_class_obj(sig.receiver_ty(), typaram_ref.idx)
                    }
                }
            }
        }
    }

    /// Build a call to `Class#_specialize1` (or `Class#<>` for multiple
    /// tyargs) to produce a specialized class object (e.g. `Array<Int>`)
    /// from an un-specialized one (e.g. `Array`).
    fn build_specialize_call(
        &self,
        class_obj: mir::TypedExpr,
        tyargs: &[TermTy],
    ) -> mir::TypedExpr {
        let class_ty = mir::Ty::raw("Class");
        if tyargs.len() == 1 {
            let arg_class_obj = self.build_class_obj_for_tyarg(&tyargs[0]);
            let fun_ty = mir::FunTy::new(
                mir::Asyncness::Unknown,
                vec![class_ty.clone(), class_ty.clone()],
                class_ty,
            );
            let func_ref =
                mir::Expr::func_ref(FunctionName::method("Class", "_specialize1"), fun_ty);
            mir::Expr::fun_call(func_ref, vec![class_obj, arg_class_obj])
        } else {
            let arg_class_objs: Vec<mir::TypedExpr> = tyargs
                .iter()
                .map(|t| self.build_class_obj_for_tyarg(t))
                .collect();
            let n = arg_class_objs.len();
            let native_array_expr = (mir::Expr::CreateNativeArray(arg_class_objs), mir::Ty::Ptr);
            let count_expr = mir::Expr::raw_i64(n as i64);
            let array_class_ty: mir::Ty = ty::ary(ty::raw("Class")).into();
            let from_raw_fun_ty = mir::FunTy::new(
                mir::Asyncness::Sync,
                vec![mir::Ty::meta("Array"), mir::Ty::Ptr, mir::Ty::Int64],
                array_class_ty.clone(),
            );
            let from_raw_ref = mir::Expr::func_ref(
                FunctionName::method("Meta:Array", "_from_raw"),
                from_raw_fun_ty.into(),
            );
            let array_class_obj_expr =
                mir::Expr::const_ref(ConstFullname::toplevel("Array"), mir::Ty::meta("Array"));
            let tyargs_array = mir::Expr::fun_call(
                from_raw_ref,
                vec![array_class_obj_expr, native_array_expr, count_expr],
            );

            let specialize_fun_ty = mir::FunTy::new(
                mir::Asyncness::Unknown,
                vec![class_ty.clone(), array_class_ty],
                class_ty,
            );
            let specialize_ref =
                mir::Expr::func_ref(FunctionName::method("Class", "<>"), specialize_fun_ty);
            mir::Expr::fun_call(specialize_ref, vec![class_obj, tyargs_array])
        }
    }

    /// Build the runtime `Class` object for the i-th class typaram of `self_ty`
    /// (matches the codegen done for `HirClassTVarRef`).
    fn class_tvar_class_obj(&self, self_ty: TermTy, idx: usize) -> mir::TypedExpr {
        let self_expr = self.compile_self_expr(self_ty);

        let object_ty = mir::Ty::raw("Object");
        let self_as_object = if self_expr.1 != object_ty {
            mir::Expr::cast(mir::CastType::Upcast(object_ty.clone()), self_expr)
        } else {
            self_expr
        };

        let class_ty = mir::Ty::raw("Class");
        let object_class_fun_ty =
            mir::FunTy::new(mir::Asyncness::Unknown, vec![object_ty], class_ty.clone());
        let object_class_ref = mir::Expr::func_ref(
            FunctionName::method("Object", "class"),
            object_class_fun_ty.into(),
        );
        let class_obj = mir::Expr::fun_call(object_class_ref, vec![self_as_object]);

        let type_arg_fun_ty = mir::FunTy::new(
            mir::Asyncness::Unknown,
            vec![class_ty.clone(), mir::Ty::raw("Int")],
            class_ty.clone(),
        );
        let type_arg_ref = mir::Expr::func_ref(
            FunctionName::method("Class", "_type_argument"),
            type_arg_fun_ty.into(),
        );
        let idx_expr = mir::Expr::number(idx as i64);
        mir::Expr::fun_call(type_arg_ref, vec![class_obj, idx_expr])
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
            } => self.create_new_body(
                signature.ret_ty.clone(),
                signature.receiver_ty(),
                initializer,
            ),
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
                mir::Expr::pseudo_var(b)
            }
            HirExpressionBase::HirStringLiteral { idx } => {
                mir::Expr::string_literal(self.str_literals[idx].clone())
            }
            HirExpressionBase::HirDecimalLiteral { value } => mir::Expr::number(value),
            HirExpressionBase::HirFloatLiteral { value } => mir::Expr::float(value),
            HirExpressionBase::HirArrayLiteral { elem_exprs } => {
                let mir_elements: Vec<mir::TypedExpr> = elem_exprs
                    .into_iter()
                    .map(|e| self.convert_expr(e))
                    .collect();
                let native_array_expr = (
                    mir::Expr::CreateNativeArray(mir_elements.clone()),
                    mir::Ty::Ptr,
                );
                let element_count = mir_elements.len();
                let count_expr = mir::Expr::raw_i64(element_count as i64);

                // Build the specialized `Array<T>` class object so that the
                // resulting instance carries a class with non-null type_args
                // (needed for runtime `Class#_type_argument` lookups).
                let specialized_class = self.build_class_obj_for_tyarg(&expr.ty);
                let receiver_expr = mir::Expr::cast(
                    mir::CastType::Force(mir::Ty::meta("Array")),
                    specialized_class,
                );

                // Call Meta:Array#_from_raw(class_obj, ptr, len) -> Array<T>
                let from_raw_fun_ty = mir::FunTy::new(
                    mir::Asyncness::Sync,
                    vec![mir::Ty::meta("Array"), mir::Ty::Ptr, mir::Ty::Int64],
                    result_ty.clone(),
                );
                let func_ref = mir::Expr::func_ref(
                    FunctionName::method("Meta:Array", "_from_raw"),
                    from_raw_fun_ty.into(),
                );
                mir::Expr::fun_call(func_ref, vec![receiver_expr, native_array_expr, count_expr])
            }
            HirExpressionBase::HirSelfExpression => self.compile_self_expr(expr.ty),
            HirExpressionBase::HirLVarRef { name } => {
                if self.lambda.cell_vars.contains(&name) {
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
                self_ty,
            } => {
                debug_assert!(typaram_ref.kind == shiika_core::ty::TyParamKind::Class);

                let self_expr = self.compile_self_expr(self_ty);

                let object_ty = mir::Ty::raw("Object");
                let self_as_object = if self_expr.1 != object_ty {
                    mir::Expr::cast(mir::CastType::Upcast(object_ty.clone()), self_expr)
                } else {
                    self_expr
                };

                let class_ty = mir::Ty::raw("Class");
                let object_class_fun_ty =
                    mir::FunTy::new(mir::Asyncness::Unknown, vec![object_ty], class_ty.clone());
                let object_class_ref = mir::Expr::func_ref(
                    FunctionName::method("Object", "class"),
                    object_class_fun_ty.into(),
                );
                let class_obj = mir::Expr::fun_call(object_class_ref, vec![self_as_object]);

                let type_arg_fun_ty = mir::FunTy::new(
                    mir::Asyncness::Unknown,
                    vec![class_ty.clone(), mir::Ty::raw("Int")],
                    class_ty.clone(),
                );
                let type_arg_ref = mir::Expr::func_ref(
                    FunctionName::method("Class", "_type_argument"),
                    type_arg_fun_ty.into(),
                );
                let idx_expr = mir::Expr::number(typaram_ref.idx as i64);
                let tyarg_class = mir::Expr::fun_call(type_arg_ref, vec![class_obj, idx_expr]);

                mir::Expr::cast(mir::CastType::Force(result_ty.clone()), tyarg_class)
            }
            HirExpressionBase::HirMethodTVarRef {
                typaram_ref,
                n_params,
            } => {
                // Method type parameters are passed as additional Class
                // arguments after self and explicit params.
                let idx = 1 + n_params + typaram_ref.idx;
                let arg = mir::Expr::arg_ref(idx, typaram_ref.name.clone(), mir::Ty::raw("Class"));
                mir::Expr::cast(mir::CastType::Force(result_ty.clone()), arg)
            }
            HirExpressionBase::HirLVarDecl {
                name,
                rhs,
                readonly,
            } => {
                let mir_rhs = self.convert_expr(*rhs);
                if self.lambda.cell_vars.contains(&name) {
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
                if self.lambda.cell_vars.contains(&name) {
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
                tyarg_exprs,
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
                    for _ in &tyarg_exprs {
                        param_tys.push(mir::Ty::raw("Class"));
                    }
                    mir::FunTy::new(mir::Asyncness::Unknown, param_tys, expr.ty.clone().into())
                };

                // Bind the receiver to a temp lvar when needed; virtual
                // dispatch evaluates it twice (vtable lookup + self arg) and
                // duplicating a non-trivial expression breaks scoped lets
                // inside it (see splice_exprs).
                let (recv_decl, recv_for_vtable, recv_for_call) = if is_virtual {
                    self.bind_receiver(mir_receiver)
                } else {
                    (None, mir_receiver.clone(), mir_receiver)
                };

                let func_ref = if is_virtual {
                    // For now, assume all method calls are virtual calls
                    let method_idx = self
                        .lookup_vtable(&receiver_ty, method_name)
                        .unwrap_or_else(|| {
                            panic!("Method not found in vtable: {}", method_fullname)
                        });

                    mir::Expr::vtable_ref(
                        recv_for_vtable,
                        method_idx,
                        method_name.0.clone(),
                        fun_ty.clone(),
                    )
                } else {
                    mir::Expr::func_ref(method_fullname.into(), fun_ty.clone())
                };
                let mut mir_args: Vec<mir::TypedExpr> = arg_exprs
                    .into_iter()
                    .map(|arg| self.convert_expr(arg))
                    .collect();
                // Upcast receiver to the method's declared owner type if they differ
                // (the actual receiver may be a more specific generic type).
                let expected_recv_ty = &fun_ty.param_tys[0];
                let receiver_for_call = if &recv_for_call.1 != expected_recv_ty {
                    mir::Expr::cast(
                        mir::CastType::Upcast(expected_recv_ty.clone()),
                        recv_for_call,
                    )
                } else {
                    recv_for_call
                };
                mir_args.insert(0, receiver_for_call);
                for tyarg in tyarg_exprs {
                    let mir_tyarg = self.convert_expr(tyarg);
                    let casted =
                        mir::Expr::cast(mir::CastType::Force(mir::Ty::raw("Class")), mir_tyarg);
                    mir_args.push(casted);
                }

                let call = (mir::Expr::FunCall(Box::new(func_ref), mir_args), result_ty);
                if let Some(decl) = recv_decl {
                    mir::Expr::exprs(vec![decl, call])
                } else {
                    call
                }
            }
            HirExpressionBase::HirModuleMethodCall {
                receiver_expr,
                module_fullname,
                method_name,
                method_idx,
                arg_exprs,
                tyarg_exprs,
                ..
            } => {
                let receiver_ty = receiver_expr.ty.clone();
                let mir_receiver = self.convert_expr(*receiver_expr);

                // Same as virtual dispatch above: avoid evaluating the
                // receiver twice when looking up via the wtable.
                let (recv_decl, recv_for_wtable, recv_for_call) = self.bind_receiver(mir_receiver);

                let func_ref = {
                    let fun_ty = {
                        let mut param_tys = arg_exprs
                            .iter()
                            .map(|e| e.ty.clone().into())
                            .collect::<Vec<_>>();
                        param_tys.insert(0, convert_ty(receiver_ty));
                        for _ in &tyarg_exprs {
                            param_tys.push(mir::Ty::raw("Class"));
                        }
                        mir::FunTy::new(mir::Asyncness::Unknown, param_tys, expr.ty.clone().into())
                    };

                    mir::Expr::wtable_ref(
                        recv_for_wtable,
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
                mir_args.insert(0, recv_for_call);
                for tyarg in tyarg_exprs {
                    let mir_tyarg = self.convert_expr(tyarg);
                    let casted =
                        mir::Expr::cast(mir::CastType::Force(mir::Ty::raw("Class")), mir_tyarg);
                    mir_args.push(casted);
                }

                let result_ty = convert_ty(expr.ty.clone());
                let call = (mir::Expr::FunCall(Box::new(func_ref), mir_args), result_ty);
                if let Some(decl) = recv_decl {
                    mir::Expr::exprs(vec![decl, call])
                } else {
                    call
                }
            }
            HirExpressionBase::HirLambdaInvocation {
                lambda_expr,
                arg_exprs,
            } => {
                let lambda_ty = lambda_expr.ty.clone();
                let fn_obj = self.convert_expr(*lambda_expr);
                let fn_obj_for_check = fn_obj.clone();
                let mir_args: Vec<_> = arg_exprs
                    .into_iter()
                    .map(|a| self.convert_expr(a))
                    .collect();
                let call_result = lambda::compile_lambda_invocation(&lambda_ty, fn_obj, mir_args);
                // After lambda call, check @exit_status for break support.
                // The check emits `return ::Void`, so only emit it when the
                // surrounding function's return type is Void; otherwise the
                // generated MIR would be ill-typed.
                let in_void_ctx = self
                    .lambda
                    .current_ret_ty
                    .as_ref()
                    .map_or(false, |t| *t == mir::Ty::raw("Void"));
                if in_void_ctx && call_result.1 == mir::Ty::raw("Void") {
                    let exit_status = mir::Expr::ivar_ref(
                        fn_obj_for_check,
                        2,
                        "@exit_status",
                        mir::Ty::raw("Int"),
                    );
                    // Call Int#==(exit_status, 1) -> Bool
                    let eq_fun_ty = mir::FunTy::new(
                        mir::Asyncness::Unknown,
                        vec![mir::Ty::raw("Int"), mir::Ty::raw("Int")],
                        mir::Ty::raw("Bool"),
                    );
                    let eq_func = mir::Expr::func_ref(
                        MethodFullname::new(TypeFullname("Int".to_string()), "==").into(),
                        eq_fun_ty,
                    );
                    let is_break =
                        mir::Expr::fun_call(eq_func, vec![exit_status, mir::Expr::number(1)]);
                    let check = mir::Expr::if_(
                        is_break,
                        mir::Expr::return_(mir::Expr::void_const_ref()),
                        mir::Expr::pseudo_var(mir::PseudoVar::Void),
                    );
                    mir::Expr::exprs(vec![call_result, check])
                } else {
                    call_result
                }
            }
            HirExpressionBase::HirLambdaExpr {
                name,
                params,
                exprs,
                captures,
                lvars,
                ret_ty,
                has_break: _,
            } => {
                let fn_class = format!("Fn{}", params.len());

                // Save state and set up lambda scope
                let saved_cell_vars = std::mem::take(&mut self.lambda.cell_vars);
                let saved_fn_class = self.lambda.current_fn_class.take();
                let saved_ret_ty = self
                    .lambda
                    .current_ret_ty
                    .replace(convert_ty(ret_ty.clone()));
                self.lambda.cell_vars = lambda::collect_cell_vars(&*exprs);
                self.lambda.current_fn_class = Some(fn_class.clone());

                // Convert body
                let body = self.convert_expr(*exprs);

                // Restore state
                self.lambda.cell_vars = saved_cell_vars;
                self.lambda.current_fn_class = saved_fn_class;
                self.lambda.current_ret_ty = saved_ret_ty;

                // Collect capture values. `current_fn_class` was just restored
                // to the enclosing lambda's class (if any), which is needed
                // to forward captures from a parent lambda's @captures.
                let capture_values: Vec<_> = captures
                    .iter()
                    .map(|cap| lambda::get_capture_value(cap, &self.lambda.current_fn_class))
                    .collect();

                // Build the lambda function and push it
                let lambda_func =
                    lambda::build_lambda_function(&name, &params, body, &lvars, &ret_ty, &fn_class);
                let func_name = lambda_func.name.clone();
                self.lambda.lambda_funcs.push(lambda_func);

                // Resolve runtime Class objects for the lambda's type args.
                let tyarg_class_objs: Vec<_> = expr
                    .ty
                    .type_args()
                    .iter()
                    .map(|t| self.build_class_obj_for_tyarg(t))
                    .collect();

                // Build and return the Fn object expression
                lambda::build_fn_object(
                    func_name,
                    &params,
                    capture_values,
                    ret_ty,
                    expr.ty,
                    tyarg_class_objs,
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
            HirExpressionBase::HirMatchExpression {
                cond_assign_expr,
                clauses,
            } => self.convert_match_expr(*cond_assign_expr, clauses),
            HirExpressionBase::HirWhileExpression {
                cond_expr,
                body_exprs,
                ..
            } => mir::Expr::while_(
                self.convert_expr(*cond_expr),
                self.convert_expr(*body_exprs),
            ),
            HirExpressionBase::HirBreakExpression { from } => match from {
                skc_hir::HirBreakFrom::Block => {
                    let fn_class = self
                        .lambda
                        .current_fn_class
                        .as_ref()
                        .expect("[BUG] break from block outside lambda");
                    let fn_obj = mir::Expr::arg_ref(0, "$fn", mir::Ty::raw(fn_class));
                    let set_exit_status =
                        mir::Expr::ivar_set(fn_obj, 2, mir::Expr::number(1), "@exit_status");
                    let return_void = mir::Expr::return_(mir::Expr::void_const_ref());
                    mir::Expr::exprs(vec![set_exit_status, return_void])
                }
                skc_hir::HirBreakFrom::While => mir::Expr::break_(),
            },
            HirExpressionBase::HirReturnExpression { arg, .. } => {
                mir::Expr::return_(self.convert_expr(*arg))
            }
            HirExpressionBase::HirLogicalNot { expr } => mir::Expr::if_(
                self.convert_expr(*expr),
                mir::Expr::pseudo_var(mir::PseudoVar::False),
                mir::Expr::pseudo_var(mir::PseudoVar::True),
            ),
            HirExpressionBase::HirLogicalAnd { left, right } => mir::Expr::if_(
                self.convert_expr(*left),
                self.convert_expr(*right),
                mir::Expr::pseudo_var(mir::PseudoVar::False),
            ),
            HirExpressionBase::HirLogicalOr { left, right } => mir::Expr::if_(
                self.convert_expr(*left),
                mir::Expr::pseudo_var(mir::PseudoVar::True),
                self.convert_expr(*right),
            ),
            HirExpressionBase::HirLambdaCaptureRef { idx, readonly } => {
                lambda::compile_lambda_capture_ref(
                    &self.lambda.current_fn_class,
                    idx,
                    readonly,
                    expr.ty,
                )
            }
            HirExpressionBase::HirLambdaCaptureWrite { cidx, rhs } => {
                let fn_class = self
                    .lambda
                    .current_fn_class
                    .clone()
                    .expect("[BUG] HirLambdaCaptureWrite outside lambda");
                let value = self.convert_expr(*rhs);
                lambda::compile_lambda_capture_write(&fn_class, cidx, value, result_ty)
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

    fn create_new_body(
        &self,
        instance_ty: TermTy,
        receiver_ty: TermTy,
        initializer: Option<MethodSignature>,
    ) -> mir::TypedExpr {
        let mut exprs = vec![];
        let tmp_name = "tmp";
        exprs.push(mir::Expr::lvar_decl(
            tmp_name,
            mir::Expr::create_object(instance_ty.clone()),
            false,
        ));
        exprs.push(mir::Expr::set_class_obj(
            mir::Expr::lvar_ref(tmp_name.to_string(), instance_ty.clone().into()),
            mir::Expr::arg_ref(0, "self", convert_ty(receiver_ty)),
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
        self.lambda.cell_vars = {
            let mut vars = HashSet::new();
            for expr in &top_exprs {
                vars.extend(lambda::collect_cell_vars(expr));
            }
            vars
        };
        let saved_ret_ty = self.lambda.current_ret_ty.replace(mir::Ty::raw("Int"));
        let mut body_stmts = vec![];
        body_stmts.extend(constants::call_all_const_inits(total_deps));
        body_stmts.push(wtables::call_main_inserter());
        body_stmts.extend(top_exprs.into_iter().map(|expr| self.convert_expr(expr)));
        body_stmts.push(mir::Expr::return_(mir::Expr::number(0)));
        self.lambda.current_ret_ty = saved_ret_ty;
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

/// Returns true if `e` is safe to evaluate multiple times (no side effects,
/// no nested let-bindings).
fn is_trivial_expr(e: &mir::Expr) -> bool {
    match e {
        mir::Expr::Number(_)
        | mir::Expr::Float(_)
        | mir::Expr::PseudoVar(_)
        | mir::Expr::StringLiteral(_)
        | mir::Expr::LVarRef(_)
        | mir::Expr::ArgRef(_, _)
        | mir::Expr::EnvRef(_, _)
        | mir::Expr::ConstRef(_)
        | mir::Expr::FuncRef(_)
        | mir::Expr::RawI64(_)
        | mir::Expr::Nop
        | mir::Expr::NullPtr => true,
        mir::Expr::Cast(_, inner) => is_trivial_expr(&inner.0),
        _ => false,
    }
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
    // Method type parameters are passed as additional Class arguments.
    for _ in &sig.typarams {
        param_tys.push(mir::Ty::raw("Class"));
    }
    mir::FunTy::new(
        sig.asyncness.clone().into(),
        param_tys,
        convert_ty(sig.ret_ty.clone()),
    )
}
