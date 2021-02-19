use crate::code_gen::code_gen_context::*;
use crate::code_gen::*;
use crate::error;
use crate::error::Error;
use crate::hir::HirExpressionBase::*;
use crate::hir::*;
use crate::names::*;
use crate::ty;
use crate::ty::*;
use inkwell::values::*;
use std::rc::Rc;

/// Number of items preceed actual arguments
pub const METHOD_FUNC_ARG_HEADER_LEN: u32 = 1;
/// Index of the receiver object in arguments of llvm func for Shiika method
const METHOD_FUNC_ARG_SELF_IDX: u32 = 0;
/// Number of items preceed actual arguments
pub const LAMBDA_FUNC_ARG_HEADER_LEN: u32 = 2;
/// Index of the FnX object in arguments of llvm func for Shiika lambda
const LAMBDA_FUNC_ARG_FN_X_IDX: u32 = 0;
/// Index of exit_status
const LAMBDA_FUNC_ARG_EXIT_STATUS_INDEX: u32 = 1;
/// Index of @the_self of FnX
const FN_X_THE_SELF_IDX: usize = 1;
/// Index of @captures of FnX
const FN_X_CAPTURES_IDX: usize = 2;
/// Index of @exit_status of FnX
const FN_X_EXIT_STATUS_IDX: usize = 3;
/// Fn::EXIT_BREAK
const EXIT_BREAK: u64 = 1;
/// Fn::EXIT_RETURN
const EXIT_RETURN: u64 = 2;

impl<'hir, 'run, 'ictx> CodeGen<'hir, 'run, 'ictx> {
    pub fn gen_exprs(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        exprs: &'hir HirExpressions,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        let mut last_value = None;
        exprs.exprs.iter().try_for_each(|expr| {
            let value: inkwell::values::BasicValueEnum = self.gen_expr(ctx, &expr)?;
            last_value = Some(value);
            Ok(())
        })?;
        Ok(last_value.expect("[BUG] HirExpressions must have at least one expr"))
    }

    pub fn gen_expr(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        expr: &'hir HirExpression,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        match &expr.node {
            HirLogicalNot { expr } => self.gen_logical_not(ctx, &expr),
            HirLogicalAnd { left, right } => self.gen_logical_and(ctx, &left, &right),
            HirLogicalOr { left, right } => self.gen_logical_or(ctx, &left, &right),
            HirIfExpression {
                cond_expr,
                then_exprs,
                else_exprs,
            } => self.gen_if_expr(ctx, &expr.ty, &cond_expr, &then_exprs, &else_exprs),
            HirWhileExpression {
                cond_expr,
                body_exprs,
            } => self.gen_while_expr(ctx, &cond_expr, &body_exprs),
            HirBreakExpression { from } => self.gen_break_expr(ctx, from),
            HirReturnExpression { from, arg } => self.gen_return_expr(ctx, arg, from),
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
            HirArgRef { idx } => self.gen_arg_ref(ctx, idx),
            HirLVarRef { name } => self.gen_lvar_ref(ctx, name),
            HirIVarRef { name, idx, self_ty } => self.gen_ivar_ref(ctx, name, idx, self_ty),
            HirConstRef { fullname } => Ok(self.gen_const_ref(fullname)),
            HirLambdaExpr {
                name,
                params,
                captures,
                ret_ty,
                ..
            } => self.gen_lambda_expr(ctx, name, params, captures, ret_ty),
            HirSelfExpression => self.gen_self_expression(ctx, &expr.ty),
            HirArrayLiteral { exprs } => self.gen_array_literal(ctx, exprs),
            HirFloatLiteral { value } => Ok(self.gen_float_literal(*value)),
            HirDecimalLiteral { value } => Ok(self.gen_decimal_literal(*value)),
            HirStringLiteral { idx } => Ok(self.gen_string_literal(idx)),
            HirBooleanLiteral { value } => Ok(self.gen_boolean_literal(*value)),

            HirLambdaCaptureRef { idx, readonly } => {
                self.gen_lambda_capture_ref(ctx, idx, !readonly, &expr.ty)
            }
            HirLambdaCaptureWrite { cidx, rhs } => {
                self.gen_lambda_capture_write(ctx, cidx, rhs, &rhs.ty)
            }
            HirBitCast { expr: target } => self.gen_bitcast(ctx, target, &expr.ty),
            HirClassLiteral {
                fullname,
                str_literal_idx,
            } => Ok(self.gen_class_literal(fullname, str_literal_idx)),
        }
    }

    fn gen_logical_not(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        expr: &'hir HirExpression,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        let b = self.gen_expr(ctx, expr)?;
        let i = self.unbox_bool(b);
        let one = self.i1_type.const_int(1, false);
        let b2 = self.builder.build_int_sub(one, i, "b2");
        Ok(self.box_bool(b2))
    }

    fn gen_logical_and(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        left: &'hir HirExpression,
        right: &'hir HirExpression,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        // REFACTOR: use `and` of LLVM
        let begin_block = self.context.append_basic_block(ctx.function, "AndBegin");
        let more_block = self.context.append_basic_block(ctx.function, "AndMore");
        let merge_block = self.context.append_basic_block(ctx.function, "AndEnd");
        // AndBegin:
        self.builder.build_unconditional_branch(begin_block);
        self.builder.position_at_end(begin_block);
        let left_value = self.gen_expr(ctx, left)?;
        self.gen_conditional_branch(left_value, more_block, merge_block);
        let begin_block_end = self.builder.get_insert_block().unwrap();
        // AndMore:
        self.builder.position_at_end(more_block);
        let right_value = self.gen_expr(ctx, right)?;
        self.builder.build_unconditional_branch(merge_block);
        let more_block_end = self.builder.get_insert_block().unwrap();
        // AndEnd:
        self.builder.position_at_end(merge_block);

        let phi_node = self
            .builder
            .build_phi(self.llvm_type(&ty::raw("Bool")), "AndResult");
        phi_node.add_incoming(&[
            (&left_value, begin_block_end),
            (&right_value, more_block_end),
        ]);
        Ok(phi_node.as_basic_value())
    }

    fn gen_logical_or(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        left: &'hir HirExpression,
        right: &'hir HirExpression,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        let begin_block = self.context.append_basic_block(ctx.function, "OrBegin");
        let else_block = self.context.append_basic_block(ctx.function, "OrElse");
        let merge_block = self.context.append_basic_block(ctx.function, "OrEnd");
        // OrBegin:
        self.builder.build_unconditional_branch(begin_block);
        self.builder.position_at_end(begin_block);
        let left_value = self.gen_expr(ctx, left)?;
        self.gen_conditional_branch(left_value, merge_block, else_block);
        let begin_block_end = self.builder.get_insert_block().unwrap();
        // OrElse:
        self.builder.position_at_end(else_block);
        let right_value = self.gen_expr(ctx, right)?;
        self.builder.build_unconditional_branch(merge_block);
        let else_block_end = self.builder.get_insert_block().unwrap();
        // OrEnd:
        self.builder.position_at_end(merge_block);

        let phi_node = self
            .builder
            .build_phi(self.llvm_type(&ty::raw("Bool")), "OrResult");
        phi_node.add_incoming(&[
            (&left_value, begin_block_end),
            (&right_value, else_block_end),
        ]);
        Ok(phi_node.as_basic_value())
    }

    fn gen_if_expr(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        ty: &TermTy,
        cond_expr: &'hir HirExpression,
        then_exprs: &'hir HirExpressions,
        else_exprs: &'hir HirExpressions,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        let begin_block = self.context.append_basic_block(ctx.function, "IfBegin");
        let then_block = self.context.append_basic_block(ctx.function, "IfThen");
        let else_block = self.context.append_basic_block(ctx.function, "IfElse");
        let merge_block = self.context.append_basic_block(ctx.function, "IfEnd");
        // IfBegin:
        self.builder.build_unconditional_branch(begin_block);
        self.builder.position_at_end(begin_block);
        let cond_value = self.gen_expr(ctx, cond_expr)?;
        self.gen_conditional_branch(cond_value, then_block, else_block);
        // IfThen:
        self.builder.position_at_end(then_block);
        let then_value = self.gen_exprs(ctx, then_exprs)?;
        self.builder.build_unconditional_branch(merge_block);
        let then_block_end = self.builder.get_insert_block().unwrap();
        // IfElse:
        self.builder.position_at_end(else_block);
        let else_value = self.gen_exprs(ctx, else_exprs)?;
        self.builder.build_unconditional_branch(merge_block);
        let else_block_end = self.builder.get_insert_block().unwrap();
        // IfEnd:
        self.builder.position_at_end(merge_block);

        if then_exprs.ty.is_never_type() {
            Ok(then_value)
        } else if else_exprs.ty.is_never_type() {
            Ok(else_value)
        } else {
            let phi_node = self.builder.build_phi(self.llvm_type(ty), "ifResult");
            phi_node.add_incoming(&[(&then_value, then_block_end), (&else_value, else_block_end)]);
            Ok(phi_node.as_basic_value())
        }
    }

    fn gen_while_expr(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        cond_expr: &'hir HirExpression,
        body_exprs: &'hir HirExpressions,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        let begin_block = self.context.append_basic_block(ctx.function, "WhileBegin");
        self.builder.build_unconditional_branch(begin_block);
        // WhileBegin:
        self.builder.position_at_end(begin_block);
        let cond_value = self.gen_expr(ctx, cond_expr)?;
        let body_block = self.context.append_basic_block(ctx.function, "WhileBody");
        let end_block = self.context.append_basic_block(ctx.function, "WhileEnd");
        self.gen_conditional_branch(cond_value, body_block, end_block);
        // WhileBody:
        self.builder.position_at_end(body_block);
        let rc1 = Rc::new(end_block);
        let rc2 = Rc::clone(&rc1);
        ctx.current_loop_end = Some(rc1);
        self.gen_exprs(ctx, body_exprs)?;
        ctx.current_loop_end = None;
        self.builder.build_unconditional_branch(begin_block);

        // WhileEnd:
        self.builder.position_at_end(*rc2);
        Ok(self.gen_const_ref(&const_fullname("::Void")))
    }

    fn gen_break_expr(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        from: &HirBreakFrom,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        let dummy_value = self.i1_type.const_int(0, false).as_basic_value_enum();
        match from {
            HirBreakFrom::While => match &ctx.current_loop_end {
                Some(b) => {
                    self.builder.build_unconditional_branch(*Rc::clone(b));
                    Ok(dummy_value)
                }
                None => Err(error::bug("break outside of a loop")),
            },
            HirBreakFrom::Block => {
                debug_assert!(ctx.function_origin == FunctionOrigin::Lambda);
                // Set @exit_status
                let fn_x = ctx.function.get_first_param().unwrap();
                let i = self.box_int(&self.i64_type.const_int(EXIT_BREAK, false));
                self.build_ivar_store(&fn_x, FN_X_EXIT_STATUS_IDX, i, "@exit_status");

                // Jump to the end of the llvm func
                self.builder
                    .build_unconditional_branch(*Rc::clone(&ctx.current_func_end));
                Ok(dummy_value)
            }
        }
    }

    fn gen_return_expr(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        arg: &'hir HirExpression,
        from: &HirReturnFrom,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        if *from == HirReturnFrom::Block {
            // This `return` escapes from the enclosing method
            debug_assert!(ctx.function_origin == FunctionOrigin::Lambda);
            // Set @exit_status
            let fn_x = ctx.function.get_first_param().unwrap();
            let i = self.box_int(&self.i64_type.const_int(EXIT_RETURN, false));
            self.build_ivar_store(&fn_x, FN_X_EXIT_STATUS_IDX, i, "@exit_status");
            self.builder.build_unconditional_branch(*ctx.current_func_end);
        } else {
            let value = self.gen_expr(ctx, arg)?;
            // Jump to the end of the llvm func
            self.builder
                .build_unconditional_branch(*Rc::clone(&ctx.current_func_end));
            let block_end = self.builder.get_insert_block().unwrap();
            ctx.returns.push((value, block_end));
        }
        let dummy_value = self.i1_type.const_int(0, false).as_basic_value_enum();
        Ok(dummy_value)
    }

    fn gen_lvar_assign(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        name: &str,
        rhs: &'hir HirExpression,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        let value = self.gen_expr(ctx, rhs)?;
        let ptr = ctx
            .lvars
            .get(name)
            .unwrap_or_else(|| panic!("[BUG] lvar `{}' not alloca'ed", name));
        self.builder.build_store(*ptr, value);
        Ok(value)
    }

    fn gen_ivar_assign(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        name: &str,
        idx: &usize,
        rhs: &'hir HirExpression,
        self_ty: &TermTy,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        let object = self.gen_self_expression(ctx, self_ty)?;
        let value = self.gen_expr(ctx, rhs)?;
        self.build_ivar_store(&object, *idx, value, name);
        Ok(value)
    }

    fn gen_const_assign(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        fullname: &ConstFullname,
        rhs: &'hir HirExpression,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        let value = self.gen_expr(ctx, rhs)?;
        let ptr = self
            .module
            .get_global(&fullname.0)
            .unwrap_or_else(|| panic!("[BUG] global for Constant `{}' not created", fullname.0))
            .as_pointer_value();
        self.builder.build_store(ptr, value);
        Ok(value)
    }

    /// Generate method call
    fn gen_method_call(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        method_fullname: &MethodFullname,
        receiver_expr: &'hir HirExpression,
        arg_exprs: &'hir [HirExpression],
        ret_ty: &TermTy,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        // Prepare arguments
        let method_name = &method_fullname.first_name;
        let receiver_value = self.gen_expr(ctx, receiver_expr)?;
        let arg_values = arg_exprs
            .iter()
            .map(|arg_expr| self.gen_expr(ctx, arg_expr))
            .collect::<Result<Vec<_>, _>>()?;

        // Create basic block
        let start_block = self
            .context
            .append_basic_block(ctx.function, &format!("Invoke_{}", method_fullname));
        self.builder.build_unconditional_branch(start_block);
        self.builder.position_at_end(start_block);
        let end_block = self
            .context
            .append_basic_block(ctx.function, &format!("Invoke_{}_end", method_fullname));

        // Get the llvm function from vtable
        let (idx, size) = self.vtables.method_idx(&receiver_expr.ty, &method_name);
        let func_raw = self.build_vtable_ref(receiver_value, *idx, size);
        let func_type = self
            .llvm_func_type(
                Some(&receiver_expr.ty),
                &arg_exprs.iter().map(|x| &x.ty).collect::<Vec<_>>(),
                ret_ty,
            )
            .ptr_type(AddressSpace::Generic);
        let func = self
            .builder
            .build_bitcast(func_raw, func_type, "func")
            .into_pointer_value();

        // Invoke the llvm function
        let result = self.gen_llvm_function_call(func, receiver_value, arg_values);

        // Check `break`|`return` in block
        if method_fullname.is_fn_x_call() && ret_ty.is_void_type() {
            let fn_x = receiver_value;
            let exit_status = self.build_ivar_load(fn_x, FN_X_EXIT_STATUS_IDX, "@exit_status");
            let eq = self.gen_llvm_func_call(
                "Int#==",
                exit_status,
                vec![self.box_int(&self.i64_type.const_int(EXIT_BREAK, false))],
            )?;
            self.gen_conditional_branch(eq, *ctx.current_func_end, end_block);
        } else {
            self.builder.build_unconditional_branch(end_block);
        }

        self.builder.position_at_end(end_block);

        result
    }

    /// Generate llvm function call
    fn gen_llvm_func_call(
        &self,
        func_name: &str,
        receiver_value: inkwell::values::BasicValueEnum<'run>,
        arg_values: Vec<inkwell::values::BasicValueEnum<'run>>,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        let function = self.get_llvm_func(func_name);
        self.gen_llvm_function_call(function, receiver_value, arg_values)
    }

    // REFACTOR: why returns Result?
    fn gen_llvm_function_call<F>(
        &self,
        function: F,
        receiver_value: inkwell::values::BasicValueEnum<'run>,
        mut arg_values: Vec<inkwell::values::BasicValueEnum<'run>>,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error>
    where
        F: Into<
            either::Either<inkwell::values::FunctionValue<'run>, inkwell::values::PointerValue<'run>>,
        >,
    {
        let mut llvm_args = vec![receiver_value];
        llvm_args.append(&mut arg_values);
        match self
            .builder
            .build_call(function, &llvm_args, "result")
            .try_as_basic_value()
            .left()
        {
            Some(result_value) => Ok(result_value),
            None => Ok(self.gen_const_ref(&const_fullname("::Void"))),
        }
    }

    /// Generate IR for HirArgRef.
    fn gen_arg_ref(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        idx: &usize,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        match ctx.function_origin {
            FunctionOrigin::Method => {
                let n = METHOD_FUNC_ARG_HEADER_LEN + (*idx as u32);
                Ok(ctx.function.get_nth_param(n).unwrap())
            }
            FunctionOrigin::Lambda => {
                let n = LAMBDA_FUNC_ARG_HEADER_LEN + (*idx as u32);
                // Bitcast is needed because lambda params are always `%Object*`
                let obj = ctx
                    .function
                    .get_nth_param(n)
                    .unwrap_or_else(|| {
                        panic!(format!(
                            "{:?}\ngen_arg_ref: no param of idx={}",
                            &ctx.function, idx
                        ))
                    });
                let llvm_type = self.llvm_type(&ctx.function_params.unwrap()[*idx].ty);
                let value = self.builder.build_bitcast(obj, llvm_type, "value");
                Ok(value)
            }
            _ => panic!("[BUG] arg ref in invalid place"),
        }
    }

    fn gen_lvar_ref(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        name: &str,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        let ptr = ctx.lvars.get(name).expect("[BUG] lvar not alloca'ed");
        Ok(self.builder.build_load(*ptr, name))
    }

    fn gen_ivar_ref(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        name: &str,
        idx: &usize,
        self_ty: &TermTy,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        let object = self.gen_self_expression(ctx, self_ty)?;
        Ok(self.build_ivar_load(object, *idx, name))
    }

    pub fn gen_const_ref(&self, fullname: &ConstFullname) -> inkwell::values::BasicValueEnum<'run> {
        let ptr = self
            .module
            .get_global(&fullname.0)
            .unwrap_or_else(|| panic!("[BUG] global for Constant `{}' not created", fullname));
        self.builder.build_load(ptr.as_pointer_value(), &fullname.0)
    }

    fn gen_lambda_expr(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        func_name: &str,
        params: &[MethodParam],
        captures: &'hir [HirLambdaCapture],
        ret_ty: &TermTy,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        let fn_x_type = &ty::raw(&format!("Fn{}", params.len()));
        let exit_status_type = &ty::raw("Int");
        let obj_type = ty::raw("Object");
        let mut arg_types = (1..=params.len()).map(|_| &obj_type).collect::<Vec<_>>();
        arg_types.insert(0, &fn_x_type);
        arg_types.insert(1, &exit_status_type);
        let func_type = self.llvm_func_type(None, &arg_types, &ret_ty);
        self.module.add_function(&func_name, func_type, None);

        // eg. Fn1.new(fnptr, the_self, captures)
        let cls_name = format!("Fn{}", params.len());
        let meta = self.gen_const_ref(&const_fullname(&("::".to_string() + &cls_name)));
        let fnptr = self
            .get_llvm_func(&func_name)
            .as_global_value()
            .as_basic_value_enum();
        let fnptr_i8 = self.builder.build_bitcast(fnptr, self.i8ptr_type, "");
        let sk_ptr = self.box_i8ptr(fnptr_i8.into_pointer_value());
        let the_self = self.gen_self_expression(ctx, &ty::raw("Object"))?;
        let arg_values = vec![sk_ptr, the_self, self.gen_lambda_captures(ctx, captures)?];
        self.gen_llvm_func_call(&format!("Meta:{}#new", cls_name), meta, arg_values)
    }

    fn gen_lambda_captures(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        captures: &'hir [HirLambdaCapture],
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        let ary = self.gen_llvm_func_call(
            "Meta:Array#new",
            self.gen_const_ref(&const_fullname("::Array")),
            vec![],
        )?;
        for cap in captures {
            let item = match cap {
                HirLambdaCapture::CaptureLVar { name } => {
                    // Local vars are captured by pointer
                    ctx.lvars.get(name).unwrap().as_basic_value_enum()
                }
                HirLambdaCapture::CaptureArg { idx } => {
                    // Args are captured by value
                    self.gen_arg_ref(ctx, idx)?
                }
                HirLambdaCapture::CaptureFwd { cidx, ty } => {
                    let deref = false; // When forwarding, pass the item as is
                    self.gen_lambda_capture_ref(ctx, cidx, deref, ty)?
                }
            };
            let obj = self.builder.build_bitcast(
                item,
                self.llvm_type(&ty::raw("Object")),
                "capture_item",
            );
            self.gen_llvm_func_call("Array#push", ary, vec![obj])?;
        }
        Ok(ary)
    }

    /// Get the object referred by `self`
    fn gen_self_expression(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        ty: &TermTy,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        let the_main = if ctx.function.get_name().to_str().unwrap() == "user_main" {
            // Toplevel
            self.the_main.unwrap()
        } else if ctx.function_origin == FunctionOrigin::Lambda {
            // In a lambda
            let fn_x = ctx.function.get_nth_param(LAMBDA_FUNC_ARG_FN_X_IDX).unwrap();
            self.build_ivar_load(fn_x, FN_X_THE_SELF_IDX, "@obj")
        } else {
            // In a method
            ctx.function.get_nth_param(METHOD_FUNC_ARG_SELF_IDX).unwrap()
        };
        Ok(self
            .builder
            .build_bitcast(the_main, self.llvm_type(ty), "the_main"))
    }

    /// Generate code for creating an array
    fn gen_array_literal(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        exprs: &'hir [HirExpression],
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        let ary = self.gen_llvm_func_call(
            "Meta:Array#new",
            self.gen_const_ref(&const_fullname("::Array")),
            vec![],
        )?;
        for expr in exprs {
            let item = self.gen_expr(ctx, expr)?;
            let obj = self
                .builder
                .build_bitcast(item, self.llvm_type(&ty::raw("Object")), "obj");
            self.gen_llvm_func_call("Array#push", ary, vec![obj])?;
        }
        Ok(ary)
    }

    fn gen_float_literal(&self, value: f64) -> inkwell::values::BasicValueEnum<'run> {
        self.box_float(&self.f64_type.const_float(value))
    }

    fn gen_decimal_literal(&self, value: i64) -> inkwell::values::BasicValueEnum<'run> {
        self.box_int(&self.i64_type.const_int(value as u64, false))
    }

    /// Create a string object
    fn gen_string_literal(&self, idx: &usize) -> inkwell::values::BasicValueEnum<'run> {
        let func = self.get_llvm_func(&"Meta:String#new");
        let receiver_value = self.gen_const_ref(&const_fullname("::String"));
        let global = self
            .module
            .get_global(&format!("str_{}", idx))
            .unwrap_or_else(|| panic!("[BUG] global for str_{} not created", idx))
            .as_pointer_value();
        let glob_i8 = self
            .builder
            .build_bitcast(global, self.i8ptr_type, "")
            .into_pointer_value();
        let bytesize = self
            .i64_type
            .const_int(self.str_literals[*idx].len() as u64, false);
        let arg_values = vec![self.box_i8ptr(glob_i8), self.box_int(&bytesize)];

        self.gen_llvm_function_call(func, receiver_value, arg_values)
            .unwrap()
    }

    fn gen_boolean_literal(&self, value: bool) -> inkwell::values::BasicValueEnum<'run> {
        let n = if value { 1 } else { 0 };
        let i = self.i1_type.const_int(n, false);
        self.box_bool(i)
    }

    /// Generate conditional branch by Shiika Bool
    fn gen_conditional_branch(
        &self,
        cond: inkwell::values::BasicValueEnum,
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
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
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
        )?;
        let ret = if deref {
            // `item` is a pointer
            let ptr_ty = self.llvm_type(ty).ptr_type(AddressSpace::Generic);
            let ptr = self
                .builder
                .build_bitcast(item, ptr_ty, "ptr")
                .into_pointer_value();
            self.builder.build_load(ptr, "ret")
        } else {
            // `item` is a value
            self.builder.build_bitcast(item, self.llvm_type(ty), "ret")
        };

        let block = self.context.append_basic_block(
            ctx.function,
            &format!("CaptureRef_{}th_end", idx_in_captures),
        );
        self.builder.build_unconditional_branch(block);
        self.builder.position_at_end(block);
        Ok(ret)
    }

    fn gen_lambda_capture_write(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        idx_in_captures: &usize,
        rhs: &'hir HirExpression,
        ty: &TermTy,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
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
        )?;
        let ptr_type = self.llvm_type(ty).ptr_type(AddressSpace::Generic);
        let ptr = self
            .builder
            .build_bitcast(ptr_, ptr_type, "ptr")
            .into_pointer_value();
        let value = self.gen_expr(ctx, rhs)?;
        self.builder.build_store(ptr, value);

        let block = self.context.append_basic_block(
            ctx.function,
            &format!("CaptureWrite_{}th_end", idx_in_captures),
        );
        self.builder.build_unconditional_branch(block);
        self.builder.position_at_end(block);
        Ok(value)
    }

    fn _gen_get_lambda_captures(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let fn_x = ctx.function.get_first_param().unwrap();
        self.build_ivar_load(fn_x, FN_X_CAPTURES_IDX, "@captures")
    }

    fn gen_bitcast(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        expr: &'hir HirExpression,
        ty: &TermTy,
    ) -> Result<inkwell::values::BasicValueEnum<'run>, Error> {
        let obj = self.gen_expr(ctx, expr)?;
        Ok(self.builder.build_bitcast(obj, self.llvm_type(ty), "as"))
    }

    #[allow(clippy::let_and_return)]
    fn gen_class_literal(
        &self,
        fullname: &ClassFullname,
        str_literal_idx: &usize,
    ) -> inkwell::values::BasicValueEnum<'run> {
        let cls_obj = self.allocate_sk_obj(&fullname.meta_name(), &format!("class_{}", fullname.0));
        self.build_ivar_store(
            &cls_obj,
            0,
            self.gen_string_literal(str_literal_idx),
            "@name",
        );

        cls_obj
    }
}
