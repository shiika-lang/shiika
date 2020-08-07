use crate::code_gen::code_gen_context::*;
use crate::code_gen::*;
use crate::error;
use crate::error::Error;
use crate::hir::HirExpressionBase::*;
use crate::hir::*;
use crate::names::*;
use crate::ty;
use crate::ty::*;
use inkwell::types::*;
use inkwell::values::*;
use inkwell::AddressSpace;
use std::rc::Rc;

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
            } => self.gen_method_call(ctx, method_fullname, receiver_expr, arg_exprs),
            HirArgRef { idx } => self.gen_arg_ref(ctx, idx),
            HirLVarRef { name } => self.gen_lvar_ref(ctx, name),
            HirIVarRef { name, idx } => self.gen_ivar_ref(ctx, name, idx),
            HirConstRef { fullname } => Ok(self.gen_const_ref(fullname)),
            HirLambda { params, exprs } => self.gen_lambda(ctx, params, exprs),
            HirSelfExpression => self.gen_self_expression(ctx),
            HirArrayLiteral { exprs } => self.gen_array_literal(ctx, exprs),
            HirFloatLiteral { value } => Ok(self.gen_float_literal(*value)),
            HirDecimalLiteral { value } => Ok(self.gen_decimal_literal(*value)),
            HirStringLiteral { idx } => Ok(self.gen_string_literal(idx)),
            HirBooleanLiteral { value } => Ok(self.gen_boolean_literal(*value)),
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
        let value = self.gen_expr(ctx, expr)?;
        Ok(self.invert_sk_bool(value).as_basic_value_enum())
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
        let left_value = self.gen_expr(ctx, left)?.into_int_value();
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
        let left_value = self.gen_expr(ctx, left)?.into_int_value();
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
        opt_else_exprs: &'hir Option<HirExpressions>,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        match opt_else_exprs {
            Some(else_exprs) => {
                let begin_block = self.context.append_basic_block(ctx.function, "IfBegin");
                let then_block = self.context.append_basic_block(ctx.function, "IfThen");
                let else_block = self.context.append_basic_block(ctx.function, "IfElse");
                let merge_block = self.context.append_basic_block(ctx.function, "IfEnd");
                // IfBegin:
                self.builder.build_unconditional_branch(begin_block);
                self.builder.position_at_end(begin_block);
                let cond_value = self.gen_expr(ctx, cond_expr)?.into_int_value();
                self.gen_conditional_branch(cond_value, then_block, else_block);
                // IfThen:
                self.builder.position_at_end(then_block);
                let then_value: &dyn inkwell::values::BasicValue =
                    &self.gen_exprs(ctx, then_exprs)?;
                self.builder.build_unconditional_branch(merge_block);
                let then_block_end = self.builder.get_insert_block().unwrap();
                // IfElse:
                self.builder.position_at_end(else_block);
                let else_value = self.gen_exprs(ctx, else_exprs)?;
                self.builder.build_unconditional_branch(merge_block);
                let else_block_end = self.builder.get_insert_block().unwrap();
                // IfEnd:
                self.builder.position_at_end(merge_block);

                let phi_node = self.builder.build_phi(self.llvm_type(ty), "ifResult");
                phi_node
                    .add_incoming(&[(then_value, then_block_end), (&else_value, else_block_end)]);
                Ok(phi_node.as_basic_value())
            }
            None => {
                let cond_value = self.gen_expr(ctx, cond_expr)?.into_int_value();
                let then_block = self.context.append_basic_block(ctx.function, "IfThen");
                let merge_block = self.context.append_basic_block(ctx.function, "IfEnd");
                self.gen_conditional_branch(cond_value, then_block, merge_block);
                // IfThen:
                self.builder.position_at_end(then_block);
                self.gen_exprs(ctx, then_exprs)?;
                self.builder.build_unconditional_branch(merge_block);
                // IfEnd:
                self.builder.position_at_end(merge_block);
                Ok(self.i1_type.const_int(0, false).as_basic_value_enum()) // dummy value
            }
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
        let cond_value = self.gen_expr(ctx, cond_expr)?.into_int_value();
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
        match ctx.lvars.get(name) {
            Some(ptr) => {
                // Reassigning; Just store to it
                self.builder.build_store(*ptr, value);
            }
            None => {
                let ptr = self.builder.build_alloca(self.llvm_type(&rhs.ty), name);
                self.builder.build_store(ptr, value);
                ctx.lvars.insert(name.to_string(), ptr);
            }
        }
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
        method_fullname: &MethodFullname,
        receiver_expr: &'hir HirExpression,
        arg_exprs: &'hir [HirExpression],
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        let receiver_value = self.gen_expr(ctx, receiver_expr)?;
        let arg_values = arg_exprs
            .iter()
            .map(|arg_expr| self.gen_expr(ctx, arg_expr))
            .collect::<Result<Vec<_>, _>>()?;
        self.gen_llvm_func_call(&method_fullname.full_name, receiver_value, arg_values)
    }

    /// Generate llvm function call
    fn gen_llvm_func_call<'a>(
        &'a self,
        func_name: &str,
        receiver_value: inkwell::values::BasicValueEnum<'a>,
        mut arg_values: Vec<inkwell::values::BasicValueEnum<'a>>,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        let function = self.get_llvm_func(func_name);
        let mut llvm_args = vec![receiver_value];
        llvm_args.append(&mut arg_values);
        match self
            .builder
            .build_call(function, &llvm_args, "result")
            .try_as_basic_value()
            .left()
        {
            Some(result_value) => Ok(result_value),
            None => Ok(self.gen_const_ref(&const_fullname("::void"))),
        }
    }

    fn gen_arg_ref(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        idx: &usize,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        let plus = match ctx.function_origin {
            FunctionOrigin::Method => 1, // +1 for the first %self
            _ => 0,
        };
        Ok(ctx.function.get_nth_param((*idx as u32) + plus).unwrap())
    }

    fn gen_lvar_ref(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        name: &str,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        let ptr = ctx.lvars.get(name).expect("[BUG] lvar not declared");
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

    fn gen_const_ref(&self, fullname: &ConstFullname) -> inkwell::values::BasicValueEnum {
        let ptr = self
            .module
            .get_global(&fullname.0)
            .unwrap_or_else(|| panic!("[BUG] global for Constant `{}' not created", fullname.0));
        self.builder.build_load(ptr.as_pointer_value(), &fullname.0)
    }

    fn gen_lambda(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        params: &'hir [MethodParam],
        exprs: &'hir HirExpressions,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        let func_name = ctx.new_lambda_name();
        ctx.push_lambda(func_name.clone(), params, exprs);

        let ret_ty = &exprs.ty;
        let func_type = self.llvm_func_type(None, &params, &ret_ty);
        self.module.add_function(&func_name, func_type, None);

        // Fn1.new(fnptr, freevars)
        let meta = self.gen_const_ref(&const_fullname("::Fn1"));
        let fnptr = self
            .get_llvm_func(&func_name)
            .as_global_value()
            .as_basic_value_enum();
        let fnptr_i8 = self.builder.build_bitcast(fnptr, self.i8ptr_type, "");
        let arg_values = vec![fnptr_i8];
        self.gen_llvm_func_call("Meta:Fn1#new", meta, arg_values)
    }

    fn gen_self_expression(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        if ctx.function.get_name().to_str().unwrap() == "user_main" {
            Ok(self.the_main.expect("[BUG] self.the_main is None"))
        } else {
            // The first arg of llvm function is `self`
            Ok(ctx
                .function
                .get_first_param()
                .expect("[BUG] get_first_param() is None"))
        }
    }

    /// Generate code for creating an array
    fn gen_array_literal(
        &self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        exprs: &'hir HirExpressions,
    ) -> Result<inkwell::values::BasicValueEnum, Error> {
        self.gen_exprs(ctx, exprs)
    }

    fn gen_float_literal(&self, value: f64) -> inkwell::values::BasicValueEnum {
        self.box_float(&self.f64_type.const_float(value))
    }

    fn gen_decimal_literal(&self, value: i32) -> inkwell::values::BasicValueEnum {
        self.box_int(&self.i32_type.const_int(value as u64, false))
    }

    fn gen_string_literal(&self, idx: &usize) -> inkwell::values::BasicValueEnum {
        // REFACTOR: Just call `new` to do this

        let sk_str = self.allocate_sk_obj(&class_fullname("String"), "str");

        // Store ptr
        let global = self
            .module
            .get_global(&format!("str_{}", idx))
            .unwrap_or_else(|| panic!("[BUG] global for str_{} not created", idx))
            .as_pointer_value();
        let glob_i8 = self.builder.build_bitcast(global, self.i8ptr_type, "");
        self.build_ivar_store(&sk_str, 0, glob_i8, "@ptr");

        // Store bytesize
        let bytesize = self
            .i32_type
            .const_int(self.str_literals[*idx].len() as u64, false);
        let sk_int = self.box_int(&bytesize);
        self.build_ivar_store(&sk_str, 1, sk_int, "@bytesize");

        sk_str
    }

    fn gen_boolean_literal(&self, value: bool) -> inkwell::values::BasicValueEnum {
        let i = if value { SK_TRUE } else { SK_FALSE };
        self.i64_type.const_int(i, false).as_basic_value_enum()
    }

    fn gen_conditional_branch(
        &self,
        cond: inkwell::values::IntValue,
        then_block: inkwell::basic_block::BasicBlock,
        else_block: inkwell::basic_block::BasicBlock,
    ) {
        let t = self.gen_boolean_literal(true);
        let istrue = self.builder.build_int_compare(
            inkwell::IntPredicate::EQ,
            cond,
            t.into_int_value(),
            "istrue",
        );
        self.builder
            .build_conditional_branch(istrue, then_block, else_block);
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
        str_literal_idx: &usize,
    ) -> inkwell::values::BasicValueEnum {
        let cls_obj = self.allocate_sk_obj(&fullname.meta_name(), &format!("class_{}", fullname.0));
        // Set @name
        self.build_ivar_store(
            &cls_obj,
            0,
            self.gen_string_literal(str_literal_idx),
            "@name",
        );

        cls_obj
    }

    // Generate call of GC_malloc and returns a ptr to Shiika object
    pub fn allocate_sk_obj(
        &self,
        class_fullname: &ClassFullname,
        reg_name: &str,
    ) -> inkwell::values::BasicValueEnum<'ictx> {
        let object_type = self.llvm_struct_types.get(&class_fullname).unwrap();
        let obj_ptr_type = object_type.ptr_type(AddressSpace::Generic);
        let size = object_type
            .size_of()
            .expect("[BUG] object_type has no size");

        // %mem = call i8* @GC_malloc(i64 %size)",
        let func = self.get_llvm_func("GC_malloc");
        let raw_addr = self
            .builder
            .build_call(func, &[size.as_basic_value_enum()], "mem")
            .try_as_basic_value()
            .left()
            .unwrap();

        // %foo = bitcast i8* %mem to %#{t}*",
        self.builder.build_bitcast(raw_addr, obj_ptr_type, reg_name)
    }

    pub fn llvm_type(&self, ty: &TermTy) -> inkwell::types::BasicTypeEnum<'ictx> {
        match ty.body {
            TyBody::TyRaw => match ty.fullname.0.as_str() {
                "Bool" => self.i64_type.as_basic_type_enum(),
                "Shiika::Internal::Ptr" => self.i8ptr_type.as_basic_type_enum(),
                _ => self.sk_obj_llvm_type(ty),
            },
            _ => self.sk_obj_llvm_type(ty),
        }
    }

    /// Return zero value in LLVM. None if it is a pointer
    pub(super) fn llvm_zero_value(&self, ty: &TermTy) -> Option<inkwell::values::BasicValueEnum> {
        match ty.body {
            TyBody::TyRaw => match ty.fullname.0.as_str() {
                "Bool" => Some(self.i1_type.const_int(0, false).as_basic_value_enum()),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn sk_obj_llvm_type(&self, ty: &TermTy) -> inkwell::types::BasicTypeEnum<'ictx> {
        let s = match &ty.body {
            TyBody::TySpe { base_name, .. } => &base_name,
            TyBody::TyParamRef { .. } => "Object", // its upper bound
            _ => &ty.fullname.0,
        };
        let struct_type = self
            .llvm_struct_types
            .get(&class_fullname(s))
            .unwrap_or_else(|| panic!("[BUG] struct_type not found: {:?}", ty.fullname));
        struct_type
            .ptr_type(AddressSpace::Generic)
            .as_basic_type_enum()
    }
}
