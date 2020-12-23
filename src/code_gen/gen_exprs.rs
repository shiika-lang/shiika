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

/// Index of @the_self of FnX
const FN_X_THE_SELF_IDX: usize = 1;
/// Index of @captures of FnX
const FN_X_CAPTURES_IDX: usize = 2;

impl<'hir, 'run, 'ictx> CodeGen<'hir, 'run, 'ictx> {
    pub fn gen_exprs(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        exprs: &'hir HirExpressions,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
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
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
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
            HirBreakExpression => self.gen_break_expr(ctx),
            HirLVarAssign { name, rhs } => self.gen_lvar_assign(ctx, name, rhs),
            HirIVarAssign { name, idx, rhs, .. } => self.gen_ivar_assign(ctx, name, idx, rhs),
            HirConstAssign { fullname, rhs } => self.gen_const_assign(ctx, fullname, rhs),
            HirMethodCall {
                receiver_expr,
                method_fullname,
                arg_exprs,
            } => self.gen_method_call(ctx, method_fullname, receiver_expr, arg_exprs, &expr.ty),
            HirArgRef { idx } => self.gen_arg_ref(ctx, idx),
            HirLVarRef { name } => self.gen_lvar_ref(ctx, name),
            HirIVarRef { name, idx } => self.gen_ivar_ref(ctx, name, idx),
            HirConstRef { name } => Ok(self.gen_const_ref(name)),
            HirLambdaExpr {
                name,
                params,
                exprs,
                captures,
                lvars,
            } => self.gen_lambda_expr(ctx, name, params, exprs, captures, lvars),
            HirSelfExpression => self.gen_self_expression(ctx),
            HirArrayLiteral { exprs } => self.gen_array_literal(ctx, exprs),
            HirFloatLiteral { value } => Ok(self.gen_float_literal(*value)),
            HirDecimalLiteral { value } => Ok(self.gen_decimal_literal(*value)),
            HirStringLiteral { idx } => Ok(self.gen_string_literal(idx)),
            HirBooleanLiteral { value } => Ok(self.gen_boolean_literal(*value)),

            HirLambdaCaptureRef { idx, readonly } => {
                self.gen_lambda_capture_ref(ctx, idx, !readonly, &expr.ty)
            }
            HirLambdaCaptureWrite { cidx, rhs, .. } => {
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
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
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
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
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
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
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
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
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
            phi_node
                .add_incoming(&[(&then_value, then_block_end), (&else_value, else_block_end)]);
            Ok(phi_node.as_basic_value())
        }
    }

    fn gen_while_expr(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        cond_expr: &'hir HirExpression,
        body_exprs: &'hir HirExpressions,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
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
        Ok(self.i32_type.const_int(0, false).as_basic_value_enum()) // return Void
    }

    fn gen_break_expr(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        match &ctx.current_loop_end {
            Some(b) => {
                self.builder.build_unconditional_branch(*Rc::clone(b));
                Ok(self.i32_type.const_int(0, false).as_basic_value_enum()) // return Void
            }
            None => Err(error::program_error("break outside of a loop")),
        }
    }

    fn gen_lvar_assign(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        name: &str,
        rhs: &'hir HirExpression,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
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
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        let theself = self.gen_self_expression(ctx)?;
        let value = self.gen_expr(ctx, rhs)?;
        self.build_ivar_store(&theself, *idx, value, name);
        Ok(value)
    }

    fn gen_const_assign(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        fullname: &ConstFullname,
        rhs: &'hir HirExpression,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
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
        method_fullname: &MethodFullname, // TODO: this should be MethodFirstname
        receiver_expr: &'hir HirExpression,
        arg_exprs: &'hir [HirExpression],
        ret_ty: &TermTy,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        let method_name = &method_fullname.first_name;
        let receiver_value = self.gen_expr(ctx, receiver_expr)?;
        let arg_values = arg_exprs
            .iter()
            .map(|arg_expr| self.gen_expr(ctx, arg_expr))
            .collect::<Result<Vec<_>, _>>()?;

        let block = self
            .context
            .append_basic_block(ctx.function, &format!("Invoke_{}", method_fullname));
        self.builder.build_unconditional_branch(block);
        self.builder.position_at_end(block);

        // Get llvm function from vtable
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

        let result = self.gen_llvm_function_call(func, receiver_value, arg_values);

        let block = self
            .context
            .append_basic_block(ctx.function, &format!("Invoke_{}_end", method_fullname));
        self.builder.build_unconditional_branch(block);
        self.builder.position_at_end(block);

        result
    }

    /// Generate llvm function call
    fn gen_llvm_func_call<'a>(
        &'a self,
        func_name: &str,
        receiver_value: inkwell::values::BasicValueEnum<'a>,
        arg_values: Vec<inkwell::values::BasicValueEnum<'a>>,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        let function = self.get_llvm_func(func_name);
        self.gen_llvm_function_call(function, receiver_value, arg_values)
    }

    // REFACTOR: why returns Result?
    fn gen_llvm_function_call<'a, F>(
        &'a self,
        function: F,
        receiver_value: inkwell::values::BasicValueEnum<'a>,
        mut arg_values: Vec<inkwell::values::BasicValueEnum<'a>>,
    ) -> Result<inkwell::values::BasicValueEnum, Error>
    where
        F: Into<
            either::Either<inkwell::values::FunctionValue<'a>, inkwell::values::PointerValue<'a>>,
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
            None => Ok(self.gen_const_ref(&const_name(vec!["Void".to_string()]))),
        }
    }

    /// Generate IR for HirArgRef.
    fn gen_arg_ref(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        idx: &usize,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        match ctx.function_origin {
            FunctionOrigin::Method => {
                Ok(ctx.function.get_nth_param((*idx as u32) + 1).unwrap()) // +1 for the first %self
            }
            FunctionOrigin::Lambda => {
                // Bitcast is needed because lambda params are always `%Object*`
                let obj = ctx.function.get_nth_param((*idx as u32) + 1).unwrap_or_else(|| { // +1 for the first %self
                    panic!(format!(
                        "{:?}\ngen_arg_ref: no param of idx={}",
                        &ctx.function, idx
                    ))
                });
                let llvm_type = self.llvm_type(&ctx.function_params.unwrap()[*idx].ty);
                let value = self.builder.build_bitcast(obj, llvm_type, "");
                Ok(value)
            }
            _ => panic!("[BUG] arg ref in invalid place"),
        }
    }

    fn gen_lvar_ref(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        name: &str,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        let ptr = ctx.lvars.get(name).expect("[BUG] lvar not alloca'ed");
        Ok(self.builder.build_load(*ptr, name))
    }

    fn gen_ivar_ref(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        name: &str,
        idx: &usize,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        let object = self.gen_self_expression(ctx)?;
        Ok(self.build_ivar_load(object, *idx, name))
    }

    fn gen_const_ref(&self, name: &ConstName) -> inkwell::values::BasicValueEnum {
        let fullname = name.fullname();
        let ptr = self
            .module
            .get_global(&fullname)
            .unwrap_or_else(|| panic!("[BUG] global for Constant `{}' not created", fullname));
        self.builder.build_load(ptr.as_pointer_value(), &fullname)
    }

    fn gen_lambda_expr(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        func_name: &str,
        params: &[MethodParam],
        exprs: &'hir HirExpressions,
        captures: &'hir [HirLambdaCapture],
        _lvars: &[(String, TermTy)],
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        let fn_x_type = &ty::raw(&format!("Fn{}", params.len()));
        let obj_type = ty::raw("Object");
        let mut arg_types = (1..=params.len()).map(|_| &obj_type).collect::<Vec<_>>();
        arg_types.insert(0, &fn_x_type);
        let ret_ty = &exprs.ty;
        let func_type = self.llvm_func_type(None, &arg_types, &ret_ty);
        self.module.add_function(&func_name, func_type, None);

        // eg. Fn1.new(fnptr, the_self, captures)
        let cls_name = format!("Fn{}", params.len());
        let meta = self.gen_const_ref(&const_name(vec![cls_name.clone()]));
        let fnptr = self
            .get_llvm_func(&func_name)
            .as_global_value()
            .as_basic_value_enum();
        let fnptr_i8 = self.builder.build_bitcast(fnptr, self.i8ptr_type, "");
        let sk_ptr = self.box_i8ptr(fnptr_i8.into_pointer_value());
        let the_self = self.builder.build_bitcast(
            self.gen_self_expression(ctx)?,
            self.llvm_type(&ty::raw("Object")),
            "the_self");
        let arg_values = vec![
            sk_ptr,
            the_self,
            self.gen_lambda_captures(ctx, captures)?,
        ];
        self.gen_llvm_func_call(&format!("Meta:{}#new", cls_name), meta, arg_values)
    }

    fn gen_lambda_captures(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        captures: &'hir [HirLambdaCapture],
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        let ary = self.gen_llvm_func_call(
            "Meta:Array#new",
            self.gen_const_ref(&const_name(vec!["Array".to_string()])),
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

    fn gen_self_expression(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        if ctx.function.get_name().to_str().unwrap() == "user_main" {
            Ok(self.the_main.unwrap())
        } else if ctx.function_origin == FunctionOrigin::Lambda {
            let fn_x = ctx.function.get_first_param().unwrap();
            Ok(self.build_ivar_load(fn_x, FN_X_THE_SELF_IDX, "the_main"))
        } else {
            // The first arg of llvm function is `self`
            Ok(ctx.function.get_first_param().unwrap())
        }
    }

    /// Generate code for creating an array
    fn gen_array_literal(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        exprs: &'hir [HirExpression],
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        let ary = self.gen_llvm_func_call(
            "Meta:Array#new",
            self.gen_const_ref(&const_name(vec!["Array".to_string()])),
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

    fn gen_float_literal(&self, value: f64) -> inkwell::values::BasicValueEnum {
        self.box_float(&self.f64_type.const_float(value))
    }

    fn gen_decimal_literal(&self, value: i64) -> inkwell::values::BasicValueEnum {
        self.box_int(&self.i64_type.const_int(value as u64, false))
    }

    /// Create a string object
    fn gen_string_literal(&self, idx: &usize) -> inkwell::values::BasicValueEnum {
        let func = self.get_llvm_func(&"Meta:String#new");
        let receiver_value = self.gen_const_ref(&const_name(vec!["String".to_string()]));
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

    fn gen_boolean_literal(&self, value: bool) -> inkwell::values::BasicValueEnum {
        let n = if value { 1 } else { 0 };
        let i = self.i1_type.const_int(n, false);
        self.box_bool(i)
    }

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
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        let block = self
            .context
            .append_basic_block(ctx.function, &format!("CaptureRef_{}th", idx_in_captures));
        self.builder.build_unconditional_branch(block);
        self.builder.position_at_end(block);

        let captures = self._gen_get_lambda_captures(ctx);
        let item = self.gen_llvm_func_call(
            "Array#nth",
            captures,
            vec![self.gen_decimal_literal(*idx_in_captures as i64)],
        )?;
        let ret =
            if deref {
                // `item` is a pointer
                let ptr_ty = self.llvm_type(ty).ptr_type(AddressSpace::Generic);
                let ptr = self
                    .builder
                    .build_bitcast(item, ptr_ty, "")
                    .into_pointer_value();
                self.builder.build_load(ptr, "")
            } else {
                // `item` is a value
                self.builder.build_bitcast(item, self.llvm_type(ty), "")
            };

        let block = self
            .context
            .append_basic_block(ctx.function, &format!("CaptureRef_{}th_end", idx_in_captures));
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
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        let block = self
            .context
            .append_basic_block(ctx.function, &format!("CaptureWrite_{}th", idx_in_captures));
        self.builder.build_unconditional_branch(block);
        self.builder.position_at_end(block);

        let captures = self._gen_get_lambda_captures(ctx);
        let ptr_ = self.gen_llvm_func_call(
            "Array#nth",
            captures,
            vec![self.gen_decimal_literal(*idx_in_captures as i64)],
        )?;
        let ptr_type = self.llvm_type(ty).ptr_type(AddressSpace::Generic);
        let ptr = self
            .builder
            .build_bitcast(ptr_, ptr_type, "")
            .into_pointer_value();
        let value = self.gen_expr(ctx, rhs)?;
        self.builder.build_store(ptr, value);

        let block = self
            .context
            .append_basic_block(ctx.function, &format!("CaptureWrite_{}th_end", idx_in_captures));
        self.builder.build_unconditional_branch(block);
        self.builder.position_at_end(block);
        Ok(value)
    }

    fn _gen_get_lambda_captures(&self, ctx: &mut CodeGenContext<'hir, 'run>) -> inkwell::values::BasicValueEnum {
        let fn_x = ctx.function.get_first_param().unwrap();
        self.build_ivar_load(fn_x, FN_X_CAPTURES_IDX, "captures")
    }

    fn gen_bitcast(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        expr: &'hir HirExpression,
        ty: &TermTy,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        let obj = self.gen_expr(ctx, expr)?;
        Ok(self.builder.build_bitcast(obj, self.llvm_type(ty), "as"))
    }

    fn gen_class_literal(
        &self,
        fullname: &ClassFullname,
        _str_literal_idx: &usize,
    ) -> inkwell::values::BasicValueEnum {
        let cls_obj = self.allocate_sk_obj(&fullname.meta_name(), &format!("class_{}", fullname.0));
        // Set @name #188
        //self.build_ivar_store(
        //    &cls_obj,
        //    0,
        //    self.gen_string_literal(str_literal_idx),
        //    "@name",
        //);

        cls_obj
    }
}
