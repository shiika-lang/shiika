use crate::code_gen::code_gen_context::*;
use crate::code_gen::values::*;
use crate::code_gen::*;
use crate::error;
use crate::error::Error;
use crate::hir::HirExpressionBase::*;
use crate::hir::*;
use crate::names::*;
use crate::ty;
use crate::ty::*;
use inkwell::values::*;
use std::convert::TryFrom;
use std::rc::Rc;

/// Index of @func of FnX
const FN_X_FUNC_IDX: usize = 0;
/// Index of @the_self of FnX
const FN_X_THE_SELF_IDX: usize = 1;
/// Index of @captures of FnX
const FN_X_CAPTURES_IDX: usize = 2;
/// Index of @exit_status of FnX
const FN_X_EXIT_STATUS_IDX: usize = 3;
/// Fn::EXIT_BREAK
const EXIT_BREAK: u64 = 1;

impl<'hir, 'run, 'ictx> CodeGen<'hir, 'run, 'ictx> {
    /// Generate LLVM IR from HirExpressions.
    /// May return `None` when, for example, it ends with a `return`
    /// expression.
    pub fn gen_exprs(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        exprs: &'hir HirExpressions,
    ) -> Result<Option<SkObj<'run>>, Error> {
        debug_assert!(!exprs.exprs.is_empty());
        let mut last_value = None;
        for expr in &exprs.exprs {
            let value = self.gen_expr(ctx, expr)?;
            if value.is_none() {
                log::warn!("detected unreachable code");
                return Ok(None);
            } else {
                last_value = Some(value);
            }
        }
        Ok(last_value.unwrap())
    }

    pub fn gen_expr(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        expr: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>, Error> {
        match &expr.node {
            HirLogicalNot { expr } => self.gen_logical_not(ctx, expr),
            HirLogicalAnd { left, right } => self.gen_logical_and(ctx, left, right),
            HirLogicalOr { left, right } => self.gen_logical_or(ctx, left, right),
            HirIfExpression {
                cond_expr,
                then_exprs,
                else_exprs,
            } => self.gen_if_expr(ctx, &expr.ty, cond_expr, then_exprs, else_exprs),
            HirWhileExpression {
                cond_expr,
                body_exprs,
            } => self.gen_while_expr(ctx, cond_expr, body_exprs),
            HirBreakExpression { from } => self.gen_break_expr(ctx, from),
            HirReturnExpression { arg, .. } => self.gen_return_expr(ctx, arg),
            HirLVarAssign { name, rhs } => self.gen_lvar_assign(ctx, name, rhs),
            HirIVarAssign {
                name,
                idx,
                rhs,
                self_ty,
                ..
            } => self.gen_ivar_assign(ctx, name, idx, rhs, self_ty),
            HirConstAssign { fullname, rhs } => self.gen_const_assign(ctx, fullname, rhs),
            HirMethodCall {
                receiver_expr,
                method_fullname,
                arg_exprs,
            } => self.gen_method_call(ctx, method_fullname, receiver_expr, arg_exprs, &expr.ty),
            HirLambdaInvocation {
                lambda_expr,
                arg_exprs,
            } => self.gen_lambda_invocation(ctx, lambda_expr, arg_exprs, &expr.ty),
            HirArgRef { idx } => Ok(Some(self.gen_arg_ref(ctx, idx))),
            HirLVarRef { name } => Ok(Some(self.gen_lvar_ref(ctx, name))),
            HirIVarRef { name, idx, self_ty } => {
                Ok(Some(self.gen_ivar_ref(ctx, name, idx, self_ty)))
            }
            HirConstRef { fullname } => Ok(Some(self.gen_const_ref(fullname))),
            HirLambdaExpr {
                name,
                params,
                captures,
                ret_ty,
                ..
            } => Ok(Some(
                self.gen_lambda_expr(ctx, name, params, captures, ret_ty),
            )),
            HirSelfExpression => Ok(Some(self.gen_self_expression(ctx, &expr.ty))),
            HirArrayLiteral { exprs } => self.gen_array_literal(ctx, exprs),
            HirFloatLiteral { value } => Ok(Some(self.gen_float_literal(*value))),
            HirDecimalLiteral { value } => Ok(Some(self.gen_decimal_literal(*value))),
            HirStringLiteral { idx } => Ok(Some(self.gen_string_literal(idx))),
            HirBooleanLiteral { value } => Ok(Some(self.gen_boolean_literal(*value))),

            HirLambdaCaptureRef { idx, readonly } => Ok(Some(
                self.gen_lambda_capture_ref(ctx, idx, !readonly, &expr.ty),
            )),
            HirLambdaCaptureWrite { cidx, rhs } => {
                self.gen_lambda_capture_write(ctx, cidx, rhs, &rhs.ty)
            }
            HirBitCast { expr: target } => self.gen_bitcast(ctx, target, &expr.ty),
            HirClassLiteral {
                fullname,
                str_literal_idx,
            } => Ok(Some(self.gen_class_literal(fullname, str_literal_idx))),
            HirParenthesizedExpr { exprs } => self.gen_exprs(ctx, exprs),
        }
    }

    fn gen_logical_not(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        expr: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>, Error> {
        if let Some(b) = self.gen_expr(ctx, expr)? {
            let i = self.unbox_bool(b);
            let b2 = self.builder.build_not(i, "b2");
            Ok(Some(self.box_bool(b2)))
        } else {
            Ok(None)
        }
    }

    fn gen_logical_and(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        left: &'hir HirExpression,
        right: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>, Error> {
        let begin_block = self.context.append_basic_block(ctx.function, "AndBegin");
        let more_block = self.context.append_basic_block(ctx.function, "AndMore");
        let merge_block = self.context.append_basic_block(ctx.function, "AndEnd");
        // AndBegin:
        self.builder.build_unconditional_branch(begin_block);
        self.builder.position_at_end(begin_block);
        let left_value = self.gen_expr(ctx, left)?.unwrap();
        self.gen_conditional_branch(left_value.clone(), more_block, merge_block);
        let begin_block_end = self.builder.get_insert_block().unwrap();
        // AndMore:
        self.builder.position_at_end(more_block);
        let right_value = self.gen_expr(ctx, right)?.unwrap();
        self.builder.build_unconditional_branch(merge_block);
        let more_block_end = self.builder.get_insert_block().unwrap();
        // AndEnd:
        self.builder.position_at_end(merge_block);

        let phi_node = self
            .builder
            .build_phi(self.llvm_type(&ty::raw("Bool")), "AndResult");
        phi_node.add_incoming(&[
            (&left_value.0, begin_block_end),
            (&right_value.0, more_block_end),
        ]);
        Ok(Some(SkObj(phi_node.as_basic_value())))
    }

    fn gen_logical_or(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        left: &'hir HirExpression,
        right: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>, Error> {
        let begin_block = self.context.append_basic_block(ctx.function, "OrBegin");
        let else_block = self.context.append_basic_block(ctx.function, "OrElse");
        let merge_block = self.context.append_basic_block(ctx.function, "OrEnd");
        // OrBegin:
        self.builder.build_unconditional_branch(begin_block);
        self.builder.position_at_end(begin_block);
        let left_value = self.gen_expr(ctx, left)?.unwrap();
        self.gen_conditional_branch(left_value.clone(), merge_block, else_block);
        let begin_block_end = self.builder.get_insert_block().unwrap();
        // OrElse:
        self.builder.position_at_end(else_block);
        let right_value = self.gen_expr(ctx, right)?.unwrap();
        self.builder.build_unconditional_branch(merge_block);
        let else_block_end = self.builder.get_insert_block().unwrap();
        // OrEnd:
        self.builder.position_at_end(merge_block);

        let phi_node = self
            .builder
            .build_phi(self.llvm_type(&ty::raw("Bool")), "OrResult");
        phi_node.add_incoming(&[
            (&left_value.0, begin_block_end),
            (&right_value.0, else_block_end),
        ]);
        Ok(Some(SkObj(phi_node.as_basic_value())))
    }

    fn gen_if_expr(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        ty: &TermTy,
        cond_expr: &'hir HirExpression,
        then_exprs: &'hir HirExpressions,
        else_exprs: &'hir HirExpressions,
    ) -> Result<Option<SkObj<'run>>, Error> {
        let begin_block = self.context.append_basic_block(ctx.function, "IfBegin");
        let then_block = self.context.append_basic_block(ctx.function, "IfThen");
        let else_block = self.context.append_basic_block(ctx.function, "IfElse");
        let merge_block = self.context.append_basic_block(ctx.function, "IfEnd");
        // IfBegin:
        self.builder.build_unconditional_branch(begin_block);
        self.builder.position_at_end(begin_block);
        let cond_value = self.gen_expr(ctx, cond_expr)?.unwrap();
        self.gen_conditional_branch(cond_value, then_block, else_block);
        // IfThen:
        self.builder.position_at_end(then_block);
        let then_value = self.gen_exprs(ctx, then_exprs)?;
        if then_value.is_some() {
            self.builder.build_unconditional_branch(merge_block);
        }
        let then_block_end = self.builder.get_insert_block().unwrap();
        // IfElse:
        self.builder.position_at_end(else_block);
        let else_value = self.gen_exprs(ctx, else_exprs)?;
        if else_value.is_some() {
            self.builder.build_unconditional_branch(merge_block);
        }
        let else_block_end = self.builder.get_insert_block().unwrap();

        // IfEnd:
        self.builder.position_at_end(merge_block);
        match (then_value, else_value) {
            (None, None) => {
                self.builder.build_unreachable();
                Ok(None)
            }
            (None, else_value) => Ok(else_value),
            (then_value, None) => Ok(then_value),
            (Some(then_val), Some(else_val)) => {
                let phi_node = self.builder.build_phi(self.llvm_type(ty), "ifResult");
                phi_node
                    .add_incoming(&[(&then_val.0, then_block_end), (&else_val.0, else_block_end)]);
                Ok(Some(SkObj(phi_node.as_basic_value())))
            }
        }
    }

    fn gen_while_expr(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        cond_expr: &'hir HirExpression,
        body_exprs: &'hir HirExpressions,
    ) -> Result<Option<SkObj<'run>>, Error> {
        let begin_block = self.context.append_basic_block(ctx.function, "WhileBegin");
        self.builder.build_unconditional_branch(begin_block);
        // WhileBegin:
        self.builder.position_at_end(begin_block);
        let cond_value = self.gen_expr(ctx, cond_expr)?.unwrap();
        let body_block = self.context.append_basic_block(ctx.function, "WhileBody");
        let end_block = self.context.append_basic_block(ctx.function, "WhileEnd");
        self.gen_conditional_branch(cond_value, body_block, end_block);
        // WhileBody:
        self.builder.position_at_end(body_block);
        let rc1 = Rc::new(end_block);
        let rc2 = Rc::clone(&rc1);
        let orig_loop_end = ctx.current_loop_end.as_ref().map(|e| Rc::clone(e));
        ctx.current_loop_end = Some(rc1);
        self.gen_exprs(ctx, body_exprs)?;
        ctx.current_loop_end = orig_loop_end;
        self.builder.build_unconditional_branch(begin_block);

        // WhileEnd:
        self.builder.position_at_end(*rc2);
        Ok(Some(self.gen_const_ref(&toplevel_const("Void"))))
    }

    fn gen_break_expr(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        from: &HirBreakFrom,
    ) -> Result<Option<SkObj<'run>>, Error> {
        match from {
            HirBreakFrom::While => match &ctx.current_loop_end {
                Some(b) => {
                    self.builder.build_unconditional_branch(*Rc::clone(b));
                    Ok(None)
                }
                None => Err(error::bug("break outside of a loop")),
            },
            HirBreakFrom::Block => {
                debug_assert!(ctx.function_origin == FunctionOrigin::Lambda);
                // Set @exit_status
                let fn_x = self.get_nth_param(&ctx.function, 0);
                let i = self.box_int(&self.i64_type.const_int(EXIT_BREAK, false));
                self.build_ivar_store(&fn_x, FN_X_EXIT_STATUS_IDX, i, "@exit_status");

                // Jump to the end of the llvm func
                self.builder
                    .build_unconditional_branch(*Rc::clone(&ctx.current_func_end));
                Ok(None)
            }
        }
    }

    fn gen_return_expr(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        arg: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>, Error> {
        let value = self.gen_expr(ctx, arg)?.unwrap();
        // Jump to the end of the llvm func
        self.builder
            .build_unconditional_branch(*Rc::clone(&ctx.current_func_end));
        let block_end = self.builder.get_insert_block().unwrap();
        ctx.returns.push((value, block_end));
        Ok(None)
    }

    fn gen_lvar_assign(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        name: &str,
        rhs: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>, Error> {
        let value = self.gen_expr(ctx, rhs)?.unwrap();
        let ptr = ctx
            .lvars
            .get(name)
            .unwrap_or_else(|| panic!("[BUG] lvar `{}' not alloca'ed", name));
        self.builder.build_store(*ptr, value.0);
        Ok(Some(value))
    }

    fn gen_ivar_assign(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        name: &str,
        idx: &usize,
        rhs: &'hir HirExpression,
        self_ty: &TermTy,
    ) -> Result<Option<SkObj<'run>>, Error> {
        let object = self.gen_self_expression(ctx, self_ty);
        let value = self.gen_expr(ctx, rhs)?.unwrap();
        self.build_ivar_store(&object, *idx, value.clone(), name);
        Ok(Some(value))
    }

    fn gen_const_assign(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        fullname: &ConstFullname,
        rhs: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>, Error> {
        let value = self.gen_expr(ctx, rhs)?.unwrap();
        let ptr = self
            .module
            .get_global(&fullname.0)
            .unwrap_or_else(|| panic!("[BUG] global for Constant `{}' not created", fullname.0))
            .as_pointer_value();
        self.builder.build_store(ptr, value.0);
        Ok(Some(value))
    }

    /// Generate method call
    fn gen_method_call(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        method_fullname: &MethodFullname,
        receiver_expr: &'hir HirExpression,
        arg_exprs: &'hir [HirExpression],
        ret_ty: &TermTy,
    ) -> Result<Option<SkObj<'run>>, Error> {
        // Prepare arguments
        let receiver_value = self.gen_expr(ctx, receiver_expr)?.unwrap();
        let mut arg_values = vec![];
        for arg_expr in arg_exprs {
            arg_values.push(self.gen_expr(ctx, arg_expr)?.unwrap());
        }

        // Create basic block
        let start_block = self
            .context
            .append_basic_block(ctx.function, &format!("Invoke_{}", method_fullname));
        self.builder.build_unconditional_branch(start_block);
        self.builder.position_at_end(start_block);

        // Get the llvm function from vtable of the class of the object
        let func_type = self.llvm_func_type(
            Some(&receiver_expr.ty),
            &arg_exprs.iter().map(|x| &x.ty).collect::<Vec<_>>(),
            ret_ty,
        );
        let func = self._get_method_func(
            &method_fullname.first_name,
            &receiver_expr.ty,
            receiver_value.clone(),
            func_type,
        );

        let result = self.gen_llvm_function_call(
            CallableValue::try_from(func).unwrap(),
            receiver_value,
            arg_values,
        );
        if ret_ty.is_never_type() {
            self.builder.build_unreachable();
            Ok(None)
        } else {
            let end_block = self
                .context
                .append_basic_block(ctx.function, &format!("Invoke_{}_end", method_fullname));
            self.builder.build_unconditional_branch(end_block);
            self.builder.position_at_end(end_block);
            Ok(Some(result))
        }
    }

    /// Retrieve the llvm func
    fn _get_method_func(
        &self,
        method_name: &MethodFirstname,
        receiver_ty: &TermTy,
        receiver_value: SkObj<'run>,
        func_type: inkwell::types::FunctionType<'ictx>,
    ) -> inkwell::values::PointerValue<'run> {
        //let class = self.get_class_of_obj(receiver_value);
        let vtable = self.get_vtable_of_obj(receiver_value);
        let (idx, size) = self.__lookup_vtable(&receiver_ty, &method_name);
        let func_raw = self.build_vtable_ref(vtable, *idx, size);
        self.builder
            .build_bitcast(func_raw, func_type.ptr_type(AddressSpace::Generic), "func")
            .into_pointer_value()
    }

    /// Get the idx and size of vtable
    fn __lookup_vtable(&self, ty: &TermTy, method_name: &MethodFirstname) -> (&usize, usize) {
        if let Some(found) = self.vtables.method_idx(ty, method_name) {
            found
        } else if let Some(found) = self.imported_vtables.method_idx(ty, method_name) {
            found
        } else {
            panic!("[BUG] method_idx: vtable of {} not found", &ty.fullname);
        }
    }

    /// Generate invocation of a lambda
    fn gen_lambda_invocation(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        lambda_expr: &'hir HirExpression,
        arg_exprs: &'hir [HirExpression],
        ret_ty: &TermTy,
    ) -> Result<Option<SkObj<'run>>, Error> {
        let lambda_obj = self.gen_expr(ctx, lambda_expr)?.unwrap();
        let n_args = arg_exprs.len();

        // Prepare arguments
        let mut args = vec![lambda_obj.0.clone()];
        for e in arg_exprs {
            args.push(self.gen_expr(ctx, e)?.unwrap().0);
        }

        // Create basic block
        let start_block = self
            .context
            .append_basic_block(ctx.function, "Invoke_lambda");
        self.builder.build_unconditional_branch(start_block);
        self.builder.position_at_end(start_block);
        let end_block = self
            .context
            .append_basic_block(ctx.function, "Invoke_lambda_end");

        // Create the type of lambda_xx()
        let fn_x_type = self.llvm_type(&ty::raw(&format!("Fn{}", n_args)));
        let mut arg_types = vec![fn_x_type];
        for e in arg_exprs {
            arg_types.push(self.llvm_type(&e.ty));
        }
        let fntype = self.llvm_type(ret_ty).fn_type(&arg_types, false);
        let fnptype = fntype.ptr_type(AddressSpace::Generic);

        // Cast `fnptr` to that type
        let fnptr =
            self.unbox_i8ptr(self.build_ivar_load(lambda_obj.clone(), FN_X_FUNC_IDX, "@func"));
        let func = self
            .builder
            .build_bitcast(fnptr.0, fnptype, "")
            .into_pointer_value();

        // Generate function call
        let result = self
            .builder
            .build_call(CallableValue::try_from(func).unwrap(), &args, "result")
            .try_as_basic_value()
            .left()
            .unwrap();

        // Check `break` in block
        if ret_ty.is_void_type() {
            let exit_status =
                self.build_ivar_load(lambda_obj, FN_X_EXIT_STATUS_IDX, "@exit_status");
            let eq = self.gen_llvm_func_call(
                "Int#==",
                exit_status,
                vec![self.box_int(&self.i64_type.const_int(EXIT_BREAK, false))],
            );
            self.gen_conditional_branch(eq, *ctx.current_func_end, end_block);
        } else {
            self.builder.build_unconditional_branch(end_block);
        }
        self.builder.position_at_end(end_block);
        Ok(Some(SkObj(result)))
    }

    /// Generate llvm function call
    // REFACTOR: make this public and make `receiver_value` optional
    fn gen_llvm_func_call(
        &self,
        func_name: &str,
        receiver_value: SkObj<'run>,
        arg_values: Vec<SkObj<'run>>,
    ) -> SkObj<'run> {
        let function = self.get_llvm_func(func_name);
        self.gen_llvm_function_call(function.into(), receiver_value, arg_values)
    }

    pub(super) fn gen_llvm_function_call(
        &self,
        function: CallableValue<'run>,
        receiver_value: SkObj<'run>,
        arg_values: Vec<SkObj<'run>>,
    ) -> SkObj<'run> {
        let mut llvm_args = vec![receiver_value.0];
        llvm_args.append(&mut arg_values.iter().map(|x| x.0).collect());
        match self
            .builder
            .build_call(function, &llvm_args, "result")
            .try_as_basic_value()
            .left()
        {
            Some(result_value) => SkObj(result_value),
            None => self.gen_const_ref(&toplevel_const("Void")),
        }
    }

    /// Generate IR for HirArgRef.
    fn gen_arg_ref(&self, ctx: &mut CodeGenContext<'hir, 'run>, idx: &usize) -> SkObj<'run> {
        match ctx.function_origin {
            FunctionOrigin::Method => {
                SkObj(ctx.function.get_nth_param((*idx as u32) + 1).unwrap()) // +1 for the first %self
            }
            FunctionOrigin::Lambda => {
                // +1 for the first %self
                let obj = self.get_nth_param(&ctx.function, *idx + 1);
                // Bitcast is needed because lambda params are always `%Object*`
                self.bitcast(obj, &ctx.function_params.unwrap()[*idx].ty, "value")
            }
            _ => panic!("[BUG] arg ref in invalid place"),
        }
    }

    fn gen_lvar_ref(&self, ctx: &mut CodeGenContext<'hir, 'run>, name: &str) -> SkObj<'run> {
        let ptr = ctx.lvars.get(name).expect("[BUG] lvar not alloca'ed");
        SkObj(self.builder.build_load(*ptr, name))
    }

    fn gen_ivar_ref(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        name: &str,
        idx: &usize,
        self_ty: &TermTy,
    ) -> SkObj<'run> {
        let object = self.gen_self_expression(ctx, self_ty);
        self.build_ivar_load(object, *idx, name)
    }

    pub fn gen_const_ref(&self, fullname: &ConstFullname) -> SkObj<'run> {
        let ptr = self
            .module
            .get_global(&fullname.0)
            .unwrap_or_else(|| panic!("[BUG] global for Constant `{}' not created", fullname));
        SkObj(self.builder.build_load(ptr.as_pointer_value(), &fullname.0))
    }

    fn gen_lambda_expr(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        func_name: &str,
        params: &[MethodParam],
        captures: &'hir [HirLambdaCapture],
        ret_ty: &TermTy,
    ) -> SkObj<'run> {
        let fn_x_type = &ty::raw(&format!("Fn{}", params.len()));
        let obj_type = ty::raw("Object");
        let mut arg_types = (1..=params.len()).map(|_| &obj_type).collect::<Vec<_>>();
        arg_types.insert(0, fn_x_type);
        let func_type = self.llvm_func_type(None, &arg_types, ret_ty);
        self.module.add_function(func_name, func_type, None);

        // eg. Fn1.new(fnptr, the_self, captures)
        let cls_name = format!("Fn{}", params.len());
        let meta = self.gen_const_ref(&toplevel_const(&cls_name));
        let fnptr = self
            .get_llvm_func(func_name)
            .as_global_value()
            .as_basic_value_enum();
        let fnptr_i8 = self.builder.build_bitcast(fnptr, self.i8ptr_type, "");
        let sk_ptr = self.box_i8ptr(fnptr_i8);
        let the_self = self.gen_self_expression(ctx, &ty::raw("Object"));
        let arg_values = vec![sk_ptr, the_self, self._gen_lambda_captures(ctx, captures)];
        self.gen_llvm_func_call(&format!("Meta:{}#new", cls_name), meta, arg_values)
    }

    fn _gen_lambda_captures(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        captures: &'hir [HirLambdaCapture],
    ) -> SkObj<'run> {
        let ary = self.gen_llvm_func_call(
            "Meta:Array#new",
            self.gen_const_ref(&toplevel_const("Array")),
            vec![],
        );
        for cap in captures {
            let item = match cap {
                HirLambdaCapture::CaptureLVar { name } => {
                    // Local vars are captured by pointer
                    SkObj(ctx.lvars.get(name).unwrap().as_basic_value_enum())
                }
                HirLambdaCapture::CaptureArg { idx } => {
                    // Args are captured by value
                    self.gen_arg_ref(ctx, idx)
                }
                HirLambdaCapture::CaptureFwd { cidx, ty } => {
                    let deref = false; // When forwarding, pass the item as is
                    self.gen_lambda_capture_ref(ctx, cidx, deref, ty)
                }
            };
            let obj = self.bitcast(item, &ty::raw("Object"), "capture_item");
            self.call_method_func("Array#push", ary.clone(), &[obj], "_");
        }
        ary
    }

    /// Get the object referred by `self`
    fn gen_self_expression(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        ty: &TermTy,
    ) -> SkObj<'run> {
        let the_main = if ctx.function.get_name().to_str().unwrap() == "user_main" {
            self.the_main.clone().unwrap()
        } else if ctx.function_origin == FunctionOrigin::Lambda {
            let fn_x = self.get_nth_param(&ctx.function, 0);
            self.build_ivar_load(fn_x, FN_X_THE_SELF_IDX, "@obj")
        } else {
            self.get_nth_param(&ctx.function, 0)
        };
        self.bitcast(the_main, ty, "the_main")
    }

    /// Generate code for creating an array
    fn gen_array_literal(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        exprs: &'hir [HirExpression],
    ) -> Result<Option<SkObj<'run>>, Error> {
        let ary = self.call_method_func(
            "Meta:Array#new",
            self.gen_const_ref(&toplevel_const("Array")),
            Default::default(),
            "ary",
        );
        for expr in exprs {
            let item = self.gen_expr(ctx, expr)?.unwrap();
            let obj = self.bitcast(item, &ty::raw("Object"), "obj");
            self.call_method_func("Array#push", ary.clone(), &[obj], "_");
        }
        Ok(Some(ary))
    }

    fn gen_float_literal(&self, value: f64) -> SkObj<'run> {
        self.box_float(&self.f64_type.const_float(value))
    }

    fn gen_decimal_literal(&self, value: i64) -> SkObj<'run> {
        self.box_int(&self.i64_type.const_int(value as u64, false))
    }

    /// Create a string object
    fn gen_string_literal(&self, idx: &usize) -> SkObj<'run> {
        let byte_ary = self
            .module
            .get_global(&format!("str_{}", idx))
            .unwrap_or_else(|| panic!("[BUG] global for str_{} not created", idx))
            .as_pointer_value();
        let i8ptr = self
            .builder
            .build_bitcast(byte_ary, self.i8ptr_type, "i8ptr");
        let bytesize = self
            .i64_type
            .const_int(self.str_literals[*idx].len() as u64, false);
        SkObj(self.call_llvm_func("gen_literal_string", &[i8ptr, bytesize.into()], "sk_str"))
    }

    fn gen_boolean_literal(&self, value: bool) -> SkObj<'run> {
        let n = if value { 1 } else { 0 };
        let i = self.i1_type.const_int(n, false);
        self.box_bool(i)
    }

    /// Generate conditional branch by Shiika Bool
    fn gen_conditional_branch(
        &self,
        cond: SkObj,
        then_block: inkwell::basic_block::BasicBlock,
        else_block: inkwell::basic_block::BasicBlock,
    ) {
        let i = self.unbox_bool(cond);
        let one = self.i1_type.const_int(1, false);
        let istrue = self
            .builder
            .build_int_compare(inkwell::IntPredicate::EQ, i, one, "istrue");
        self.builder
            .build_conditional_branch(istrue, then_block, else_block);
    }

    /// Get an object from `captures`
    fn gen_lambda_capture_ref(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        idx_in_captures: &usize,
        deref: bool,
        ty: &TermTy,
    ) -> SkObj<'run> {
        let block = self
            .context
            .append_basic_block(ctx.function, &format!("CaptureRef_{}th", idx_in_captures));
        self.builder.build_unconditional_branch(block);
        self.builder.position_at_end(block);

        let captures = self._gen_get_lambda_captures(ctx);
        let item = self.gen_llvm_func_call(
            "Array#[]",
            captures,
            vec![self.gen_decimal_literal(*idx_in_captures as i64)],
        );
        let ret = if deref {
            // `item` is a pointer
            let ptr_ty = self.llvm_type(ty).ptr_type(AddressSpace::Generic);
            let ptr = self
                .builder
                .build_bitcast(item.0, ptr_ty, "ptr")
                .into_pointer_value();
            SkObj(self.builder.build_load(ptr, "ret"))
        } else {
            // `item` is a value
            self.bitcast(item, ty, "ret")
        };

        let block = self.context.append_basic_block(
            ctx.function,
            &format!("CaptureRef_{}th_end", idx_in_captures),
        );
        self.builder.build_unconditional_branch(block);
        self.builder.position_at_end(block);
        ret
    }

    fn gen_lambda_capture_write(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        idx_in_captures: &usize,
        rhs: &'hir HirExpression,
        ty: &TermTy,
    ) -> Result<Option<SkObj<'run>>, Error> {
        let block = self
            .context
            .append_basic_block(ctx.function, &format!("CaptureWrite_{}th", idx_in_captures));
        self.builder.build_unconditional_branch(block);
        self.builder.position_at_end(block);

        let captures = self._gen_get_lambda_captures(ctx);
        let ptr_ = self.gen_llvm_func_call(
            "Array#[]",
            captures,
            vec![self.gen_decimal_literal(*idx_in_captures as i64)],
        );
        let ptr_type = self.llvm_type(ty).ptr_type(AddressSpace::Generic);
        let ptr = self
            .builder
            .build_bitcast(ptr_.0, ptr_type, "ptr")
            .into_pointer_value();
        let value = self.gen_expr(ctx, rhs)?.unwrap();
        self.builder.build_store(ptr, value.0);

        let block = self.context.append_basic_block(
            ctx.function,
            &format!("CaptureWrite_{}th_end", idx_in_captures),
        );
        self.builder.build_unconditional_branch(block);
        self.builder.position_at_end(block);
        Ok(Some(value))
    }

    fn _gen_get_lambda_captures(&self, ctx: &mut CodeGenContext<'hir, 'run>) -> SkObj<'run> {
        let fn_x = self.get_nth_param(&ctx.function, 0);
        self.build_ivar_load(fn_x, FN_X_CAPTURES_IDX, "@captures")
    }

    fn gen_bitcast(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        expr: &'hir HirExpression,
        ty: &TermTy,
    ) -> Result<Option<SkObj<'run>>, Error> {
        if let Some(obj) = self.gen_expr(ctx, expr)? {
            if expr.ty.equals_to(ty) {
                // No bitcast needed
                Ok(Some(obj))
            } else {
                Ok(Some(self.bitcast(obj, ty, "as")))
            }
        } else {
            Ok(None)
        }
    }

    /// Create a class object
    /// ("class literal" is a special Hir that does not appear directly
    /// on a source text.)
    fn gen_class_literal(&self, fullname: &ClassFullname, str_literal_idx: &usize) -> SkObj<'run> {
        debug_assert!(!fullname.is_meta());
        if fullname.0 == "Metaclass" {
            self.gen_the_metaclass(str_literal_idx)
        } else {
            let meta_name = fullname.meta_name(); // eg. "Meta:Int"

            // Create metaclass object
            let metacls_obj = self.allocate_sk_obj(&class_fullname("Metaclass"), "metaclass_obj");
            self.build_ivar_store(
                &metacls_obj,
                0,
                self.gen_string_literal(str_literal_idx),
                "@base_name",
            );

            let cls_obj = self._allocate_sk_obj(&meta_name, "cls_obj", metacls_obj.as_class_obj());
            self.build_ivar_store(
                &cls_obj,
                0,
                self.gen_string_literal(str_literal_idx),
                "@name",
            );

            // We assume class objects never have custom `initialize` method
            cls_obj
        }
    }

    fn gen_the_metaclass(&self, str_literal_idx: &usize) -> SkObj<'run> {
        // We need a trick here to achieve `Metaclass.class == Metaclass`.
        let null = self.i8ptr_type.const_null().as_basic_value_enum();
        let cls_obj = self._allocate_sk_obj(
            &class_fullname("Metaclass"),
            "the_metaclass",
            SkClassObj(null),
        );
        self.build_ivar_store(
            &cls_obj,
            0,
            self.gen_string_literal(str_literal_idx),
            "@base_name",
        );
        self.set_class_of_obj(&cls_obj, SkClassObj(cls_obj.0));
        cls_obj
    }
}
