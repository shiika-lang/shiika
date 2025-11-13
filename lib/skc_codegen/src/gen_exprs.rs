use crate::code_gen_context::*;
use crate::lambda::LambdaCapture;
use crate::utils::*;
use crate::values::*;
use crate::vtable::VTableRef;
use crate::wtable;
use crate::CodeGen;
use anyhow::Result;
use inkwell::types::*;
use inkwell::values::*;
use shiika_core::{names::*, ty, ty::*};
use skc_corelib::fn_x;
use skc_hir::pattern_match;
use skc_hir::HirExpressionBase::*;
use skc_hir::*;
use std::rc::Rc;

/// Fn::EXIT_BREAK
const EXIT_BREAK: u64 = 1;

impl<'hir, 'run, 'ictx> CodeGen<'hir, 'run, 'ictx> {
    pub fn gen_expr(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        expr: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>> {
        // Debug helper: print the expr under processing
        //let msg = format!("{:?}", expr);
        //println!(
        //    "{}",
        //    skc_error::build_report("-".to_string(), &expr.locs, |r, locs_span| {
        //        r.with_label(skc_error::Label::new(locs_span).with_message(msg))
        //    })
        //);
        match &expr.node {
            HirLogicalNot { expr } => self.gen_logical_not(ctx, expr),
            HirLogicalAnd { left, right } => self.gen_logical_and(ctx, left, right),
            HirLogicalOr { left, right } => self.gen_logical_or(ctx, left, right),
            HirIfExpression {
                cond_expr,
                then_exprs,
                else_exprs,
                ..
            } => self.gen_if_expr(ctx, &expr.ty, cond_expr, then_exprs, else_exprs),
            HirMatchExpression {
                cond_assign_expr,
                clauses,
            } => self.gen_match_expr(ctx, &expr.ty, cond_assign_expr, clauses),
            HirWhileExpression {
                cond_expr,
                body_exprs,
                ..
            } => self.gen_while_expr(ctx, cond_expr, body_exprs),
            HirBreakExpression { from } => self.gen_break_expr(ctx, from),
            HirReturnExpression { arg, .. } => self.gen_return_expr(ctx, arg),
            HirLVarAssign { name, rhs } => self.gen_lvar_assign(ctx, name, rhs),
            HirIVarAssign {
                name,
                rhs,
                self_ty,
                idx,
                ..
            } => self.gen_ivar_assign(ctx, idx, name, rhs, self_ty),
            HirConstAssign { fullname, rhs } => self.gen_const_assign(ctx, fullname, rhs),
            HirMethodCall {
                receiver_expr,
                method_fullname,
                arg_exprs,
                tyarg_exprs,
                ..
            } => self.gen_method_call(
                ctx,
                method_fullname,
                receiver_expr,
                arg_exprs,
                tyarg_exprs,
                &expr.ty,
            ),
            HirModuleMethodCall {
                receiver_expr,
                module_fullname,
                method_name,
                method_idx,
                arg_exprs,
                tyarg_exprs,
            } => self.gen_module_method_call(
                ctx,
                module_fullname,
                method_name,
                method_idx,
                receiver_expr,
                arg_exprs,
                tyarg_exprs,
                &expr.ty,
            ),
            HirLambdaInvocation {
                lambda_expr,
                arg_exprs,
            } => self.gen_lambda_invocation(ctx, lambda_expr, arg_exprs, &expr.ty),
            HirArgRef { idx } => Ok(Some(self.gen_arg_ref(ctx, idx))),
            HirLVarRef { name } => Ok(Some(self.gen_lvar_ref(ctx, &expr.ty, name))),
            HirIVarRef {
                name, self_ty, idx, ..
            } => Ok(Some(self.gen_ivar_ref(ctx, idx, name, self_ty, &expr.ty))),
            HirClassTVarRef {
                typaram_ref,
                self_ty,
            } => Ok(Some(self.gen_class_tvar_ref(
                ctx,
                typaram_ref,
                self_ty,
                &expr.ty,
            ))),
            HirMethodTVarRef {
                typaram_ref,
                n_params,
            } => Ok(Some(self.gen_method_tvar_ref(ctx, typaram_ref, n_params))),
            HirConstRef { fullname } => Ok(Some(self.gen_const_ref(fullname, &expr.ty))),
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
            HirFloatLiteral { value } => Ok(Some(self.gen_float_literal(*value))),
            HirDecimalLiteral { value } => Ok(Some(self.gen_decimal_literal(*value))),
            HirStringLiteral { idx } => Ok(Some(self.gen_string_literal(idx))),
            HirBooleanLiteral { value } => Ok(Some(self.gen_boolean_literal(*value))),

            HirLambdaCaptureRef { idx, readonly } => Ok(Some(
                self.gen_lambda_capture_ref(ctx, idx, !readonly, &expr.ty),
            )),
            HirLambdaCaptureWrite { cidx, rhs } => self.gen_lambda_capture_write(ctx, cidx, rhs),
            HirBitCast { expr: target } => self.gen_bitcast(ctx, target),
            HirClassLiteral {
                fullname,
                str_literal_idx,
                includes_modules,
                initializer,
            } => Ok(Some(self.gen_class_literal(
                fullname,
                &expr.ty,
                str_literal_idx,
                includes_modules,
                initializer,
            ))),
            HirParenthesizedExpr { exprs } => self.gen_parenthesized_expr(ctx, exprs),
            HirDefaultExpr => Ok(Some(self.gen_default_expr(&expr.ty))),
            HirIsOmittedValue { expr } => self.gen_is_omitted_value(ctx, expr),
        }
    }

    fn gen_logical_not(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        expr: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>> {
        if let Some(b) = self.gen_expr(ctx, expr)? {
            let i = self.unbox_bool(b);
            let b2 = self.builder.build_not(i, "b2")?;
            Ok(Some(self.box_bool(b2)))
        } else {
            Ok(None)
        }
    }

    fn gen_logical_and(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        left: &'hir HirExpression,
        right: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>> {
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

        let phi_node = self.builder.build_phi(self.llvm_type(), "AndResult")?;
        phi_node.add_incoming(&[
            (&left_value.0, begin_block_end),
            (&right_value.0, more_block_end),
        ]);
        Ok(Some(SkObj::new(ty::raw("Bool"), phi_node.as_basic_value())))
    }

    fn gen_logical_or(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        left: &'hir HirExpression,
        right: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>> {
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

        let phi_node = self.builder.build_phi(self.llvm_type(), "OrResult")?;
        phi_node.add_incoming(&[
            (&left_value.0, begin_block_end),
            (&right_value.0, else_block_end),
        ]);
        Ok(Some(SkObj::new(ty::raw("Bool"), phi_node.as_basic_value())))
    }

    fn gen_if_expr(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        ty: &TermTy,
        cond_expr: &'hir HirExpression,
        then_exprs: &'hir HirExpression,
        else_exprs: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>> {
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
        let then_value = self.gen_expr(ctx, then_exprs)?;
        if then_value.is_some() {
            self.builder.build_unconditional_branch(merge_block);
        }
        let then_block_end = self.builder.get_insert_block().unwrap();
        // IfElse:
        self.builder.position_at_end(else_block);
        let else_value = self.gen_expr(ctx, else_exprs)?;
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
                let phi_node = self.builder.build_phi(self.llvm_type(), "ifResult")?;
                phi_node
                    .add_incoming(&[(&then_val.0, then_block_end), (&else_val.0, else_block_end)]);
                Ok(Some(SkObj::new(ty.clone(), phi_node.as_basic_value())))
            }
        }
    }

    fn gen_match_expr(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        result_ty: &TermTy,
        cond_assign_expr: &'hir HirExpression,
        clauses: &'hir [pattern_match::MatchClause],
    ) -> Result<Option<SkObj<'run>>> {
        let n_clauses = clauses.len();
        let begin_block = self.context.append_basic_block(ctx.function, "MatchBegin");
        let clause_blocks = (1..=n_clauses)
            .map(|i| {
                self.context
                    .append_basic_block(ctx.function, &format!("MatchClause{}_", i))
            })
            .collect::<Vec<_>>();
        let merge_block = self.context.append_basic_block(ctx.function, "MatchEnd");
        // MatchBegin:
        self.builder.build_unconditional_branch(begin_block);
        self.builder.position_at_end(begin_block);
        self.gen_expr(ctx, cond_assign_expr)?;
        self.builder.build_unconditional_branch(clause_blocks[0]);

        // MatchClauseX:
        let mut incoming_values = vec![];
        let mut incoming_blocks = vec![];
        for (i, clause) in clauses.iter().enumerate() {
            let clause_block = clause_blocks[i];
            let next_block = if (i + 1) < n_clauses {
                clause_blocks[i + 1]
            } else {
                merge_block
            };
            self.builder.position_at_end(clause_block);
            let opt_val = self.gen_match_clause(ctx, clause, next_block, result_ty)?;
            if let Some(val) = opt_val {
                let last_block = self.builder.get_insert_block().unwrap();
                incoming_values.push(val.0);
                incoming_blocks.push(last_block);
                self.builder.build_unconditional_branch(merge_block);
            }
        }

        if incoming_blocks.is_empty() {
            // All the clauses ends with a jump; no merge block needed
            self.builder.position_at_end(merge_block);
            self.builder.build_unreachable();
            Ok(None)
        } else {
            // MatchEnd:
            self.builder.position_at_end(merge_block);
            let phi_node = self.builder.build_phi(self.llvm_type(), "matchResult")?;
            phi_node.add_incoming(
                incoming_values
                    .iter()
                    .zip(incoming_blocks.into_iter())
                    .map(|(v, b)| (v as &dyn BasicValue, b))
                    .collect::<Vec<_>>()
                    .as_slice(),
            );
            Ok(Some(SkObj::new(
                result_ty.clone(),
                phi_node.as_basic_value(),
            )))
        }
    }

    fn gen_match_clause(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        clause: &'hir pattern_match::MatchClause,
        skip_block: inkwell::basic_block::BasicBlock,
        result_ty: &TermTy,
    ) -> Result<Option<SkObj<'run>>> {
        let lvar_ptrs = self.gen_alloca_lvars(ctx.function, &clause.lvars);
        let orig_lvars = ctx.inject_lvars(lvar_ptrs);
        for component in &clause.components {
            match component {
                pattern_match::Component::Test(expr) => {
                    let v = self.gen_expr(ctx, expr)?.unwrap();
                    let cont_block = self.context.append_basic_block(ctx.function, "Matching");
                    self.gen_conditional_branch(v, cont_block, skip_block);
                    // Continue processing this clause
                    self.builder.position_at_end(cont_block);
                }
                pattern_match::Component::Bind(name, expr) => {
                    self.gen_lvar_assign(ctx, name, expr)?;
                }
            }
        }
        let result = self
            .gen_expr(ctx, &clause.body_hir)?
            .map(|v| self.bitcast(v, result_ty, "as"));
        ctx.lvars = orig_lvars;
        Ok(result)
    }

    fn gen_while_expr(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        cond_expr: &'hir HirExpression,
        body_exprs: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>> {
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
        let orig_loop_end = ctx.current_loop_end.as_ref().map(Rc::clone);
        ctx.current_loop_end = Some(rc1);
        self.gen_expr(ctx, body_exprs)?;
        ctx.current_loop_end = orig_loop_end;
        self.builder.build_unconditional_branch(begin_block);

        // WhileEnd:
        self.builder.position_at_end(*rc2);
        Ok(Some(
            self.gen_const_ref(&toplevel_const("Void"), &ty::raw("Void")),
        ))
    }

    fn gen_break_expr(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        from: &HirBreakFrom,
    ) -> Result<Option<SkObj<'run>>> {
        match from {
            HirBreakFrom::While => match &ctx.current_loop_end {
                Some(b) => {
                    self.builder.build_unconditional_branch(*Rc::clone(b));
                    Ok(None)
                }
                None => panic!("[BUG] break outside of a loop"),
            },
            HirBreakFrom::Block => {
                debug_assert!(matches!(ctx.function_origin, FunctionOrigin::Lambda { .. }));
                // Set @exit_status
                let fn_x = self.get_nth_param(ty::raw("Fn"), &ctx.function, 0);
                let i = self.box_int(&self.i64_type.const_int(EXIT_BREAK, false));
                self.build_ivar_store(fn_x, fn_x::IVAR_EXIT_STATUS_IDX, "@exit_status", i);

                // Jump to the end of the llvm func
                self.builder
                    .build_unconditional_branch(*Rc::clone(&ctx.current_func_end));
                Ok(None)
            }
        }
    }

    fn gen_return_expr(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        arg: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>> {
        let value = self.gen_expr(ctx, arg)?.unwrap();
        // Jump to the end of the llvm func
        self.builder
            .build_unconditional_branch(*Rc::clone(&ctx.current_func_end));
        let block_end = self.builder.get_insert_block().unwrap();
        ctx.returns.push((value, block_end));
        Ok(None)
    }

    fn gen_lvar_assign(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        name: &str,
        rhs: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>> {
        let value = self.gen_expr(ctx, rhs)?.unwrap();
        let ptr = ctx
            .lvars
            .get(name)
            .unwrap_or_else(|| panic!("[BUG] lvar `{}' not found in ctx.lvars", name));
        self.builder.build_store(*ptr, value.0);
        Ok(Some(value))
    }

    fn gen_ivar_assign(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        idx: &usize,
        name: &str,
        rhs: &'hir HirExpression,
        self_ty: &TermTy,
    ) -> Result<Option<SkObj<'run>>> {
        let object = self.gen_self_expression(ctx, self_ty);
        let value = self.gen_expr(ctx, rhs)?.unwrap();
        self.build_ivar_store(object, *idx, name, value.clone());
        Ok(Some(value))
    }

    fn gen_const_assign(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        fullname: &ConstFullname,
        rhs: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>> {
        let value = self.gen_expr(ctx, rhs)?.unwrap();
        let name = llvm_const_name(fullname);
        let ptr = self
            .module
            .get_global(&name)
            .unwrap_or_else(|| panic!("[BUG] global for Constant `{}' not created", fullname.0))
            .as_pointer_value();
        self.builder.build_store(ptr, value.0);
        Ok(Some(value))
    }

    /// Generate method call
    fn gen_method_call(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        method_fullname: &MethodFullname,
        receiver_expr: &'hir HirExpression,
        arg_exprs: &'hir [HirExpression],
        tyarg_exprs: &'hir [HirExpression],
        ret_ty: &TermTy,
    ) -> Result<Option<SkObj<'run>>> {
        // Prepare arguments
        let receiver_value = self.gen_expr(ctx, receiver_expr)?.unwrap();
        let mut arg_tys = vec![];
        let mut arg_values = vec![];
        for expr in arg_exprs {
            arg_tys.push(&expr.ty);
            arg_values.push(self.gen_expr(ctx, expr)?.unwrap());
        }
        for expr in tyarg_exprs {
            arg_tys.push(&expr.ty);
            arg_values.push(self.gen_expr(ctx, expr)?.unwrap());
        }

        // Create basic block
        let start_block = self
            .context
            .append_basic_block(ctx.function, &format!("Invoke_{}", method_fullname));
        self.builder.build_unconditional_branch(start_block);
        self.builder.position_at_end(start_block);

        // Get the llvm function from vtable of the class of the object
        let func_type = self.llvm_func_type(Some(&receiver_expr.ty), &arg_tys, ret_ty);
        let func = self._get_method_func(
            &method_fullname.first_name,
            &receiver_expr.ty,
            receiver_value.clone(),
            func_type,
        );

        let result = self.indirect_method_function_call(
            ret_ty.clone(),
            func,
            func_type,
            receiver_value,
            &arg_values,
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
        &'run self,
        method_name: &MethodFirstname,
        receiver_ty: &TermTy,
        receiver_value: SkObj<'run>,
        func_type: inkwell::types::FunctionType<'ictx>,
    ) -> inkwell::values::PointerValue<'run> {
        let (idx, size) = self.__lookup_vtable(receiver_ty, method_name);
        let vtable = VTableRef::of_sk_obj(self, receiver_value, size);
        let func_raw = vtable.get_func(self, *idx);
        self.builder
            .build_bit_cast(func_raw, self.context.ptr_type(Default::default()), "func")
            .unwrap()
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

    /// Generate method call via wtable
    #[allow(clippy::too_many_arguments)]
    fn gen_module_method_call(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        module_fullname: &ModuleFullname,
        method_name: &MethodFirstname,
        method_idx: &usize,
        receiver_expr: &'hir HirExpression,
        arg_exprs: &'hir [HirExpression],
        tyarg_exprs: &'hir [HirExpression],
        ret_ty: &TermTy,
    ) -> Result<Option<SkObj<'run>>> {
        // Prepare arguments
        let receiver_value = self.gen_expr(ctx, receiver_expr)?.unwrap();
        let mut arg_tys = vec![];
        let mut arg_values = vec![];
        for expr in arg_exprs {
            arg_tys.push(&expr.ty);
            arg_values.push(self.gen_expr(ctx, expr)?.unwrap());
        }
        for expr in tyarg_exprs {
            arg_tys.push(&expr.ty);
            arg_values.push(self.gen_expr(ctx, expr)?.unwrap());
        }

        // Create basic block
        let start_block = self
            .context
            .append_basic_block(ctx.function, &format!("Invoke_{}", method_name));
        self.builder.build_unconditional_branch(start_block);
        self.builder.position_at_end(start_block);

        // Get the llvm function via wtable
        let key = self.get_const_addr_int(&module_fullname.to_const_fullname());
        let idx = self.i64_type.const_int(*method_idx as u64, false);
        let args = &[
            receiver_value.clone().into_i8ptr(self).into(),
            key.as_basic_value_enum().into(),
            idx.as_basic_value_enum().into(),
        ];
        let func_ptr = self
            .call_llvm_func(&llvm_func_name("shiika_lookup_wtable"), args, "method")
            .into_pointer_value();
        let func_type = self.llvm_func_type(Some(&receiver_expr.ty), &arg_tys, ret_ty);
        let func = self
            .builder
            .build_bit_cast(func_ptr, self.context.ptr_type(Default::default()), "as")
            .unwrap()
            .into_pointer_value();

        let result = self.indirect_method_function_call(
            ret_ty.clone(),
            func,
            func_type,
            receiver_value,
            &arg_values,
        );
        if ret_ty.is_never_type() {
            self.builder.build_unreachable();
            Ok(None)
        } else {
            let end_block = self
                .context
                .append_basic_block(ctx.function, &format!("Invoke_{}_end", method_name));
            self.builder.build_unconditional_branch(end_block);
            self.builder.position_at_end(end_block);
            Ok(Some(result))
        }
    }

    /// Get the address of a Shiika constant and returns it as an integer
    pub fn get_const_addr_int(&self, fullname: &ConstFullname) -> inkwell::values::IntValue<'run> {
        let name = llvm_const_name(fullname);
        let ptr = self
            .module
            .get_global(&name)
            .unwrap_or_else(|| panic!("[BUG] global for Constant `{}' not created", fullname))
            .as_pointer_value();
        ptr.const_to_int(self.i64_type)
    }

    /// Generate invocation of a lambda
    fn gen_lambda_invocation(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        lambda_expr: &'hir HirExpression,
        arg_exprs: &'hir [HirExpression],
        ret_ty: &TermTy,
    ) -> Result<Option<SkObj<'run>>> {
        let lambda_obj = self.gen_expr(ctx, lambda_expr)?.unwrap();

        // Prepare arguments
        let mut args = vec![lambda_obj.clone()];
        for e in arg_exprs {
            args.push(self.gen_expr(ctx, e)?.unwrap());
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
        let n = arg_exprs.len() + 1; // +1 for self
        let arg_types = vec![self.llvm_type().into(); n];
        let fntype = self.llvm_type().fn_type(&arg_types, false);
        let fnptype = self.ptr_type;

        // Cast `fnptr` to that type
        let fnptr = self.unbox_i8ptr(self.build_ivar_load(
            lambda_obj.clone(),
            ty::raw("Shiika::Internal::Ptr"),
            skc_corelib::fn_x::IVAR_FUNC_IDX,
            "@func",
        ));
        let func = self
            .builder
            .build_bit_cast(fnptr.0, fnptype, "")
            .unwrap()
            .into_pointer_value();

        // Generate function call
        let result = self.indirect_function_call(ret_ty.clone(), func, fntype, args);

        // Check `break` in block
        if ret_ty.is_void_type() {
            let broke_block = self.context.append_basic_block(ctx.function, "Broke");
            let exit_status = self.build_ivar_load(
                lambda_obj,
                ty::raw("Int"),
                skc_corelib::fn_x::IVAR_EXIT_STATUS_IDX,
                "@exit_status",
            );
            let eq = self.call_method_func(
                &method_fullname_raw("Int", "=="),
                exit_status,
                &vec![self.box_int(&self.i64_type.const_int(EXIT_BREAK, false))],
                ty::raw("Bool"),
                "eq",
            );
            self.gen_conditional_branch(eq, broke_block, end_block);

            // If `break` happened...
            self.builder.position_at_end(broke_block);
            if matches!(ctx.function_origin, FunctionOrigin::Lambda { .. }) {
                // Set @exit_status
                let fn_x = self.get_nth_param(ty::raw("Fn"), &ctx.function, 0);
                let i = self.box_int(&self.i64_type.const_int(EXIT_BREAK, false));
                self.build_ivar_store(fn_x, fn_x::IVAR_EXIT_STATUS_IDX, "@exit_status", i);
            }
            self.builder
                .build_unconditional_branch(*ctx.current_func_end);
        } else {
            // No check is needed because `break` is not allowed if the lambda has a return type.
            self.builder.build_unconditional_branch(end_block);
        }
        self.builder.position_at_end(end_block);
        Ok(Some(result))
    }

    // Call a method llvm function via function pointer.
    fn indirect_method_function_call(
        &'run self,
        result_ty: TermTy,
        function: PointerValue<'run>,
        func_type: FunctionType<'ictx>,
        receiver_value: SkObj<'run>,
        arg_values: &[SkObj<'run>],
    ) -> SkObj<'run> {
        let mut args = arg_values.to_vec();
        args.insert(0, receiver_value);
        self.indirect_function_call(result_ty, function, func_type, args)
    }

    // Call a llvm function via function pointer.
    fn indirect_function_call(
        &'run self,
        result_ty: TermTy,
        function: PointerValue<'run>,
        func_type: FunctionType<'ictx>,
        arg_values: Vec<SkObj<'run>>,
    ) -> SkObj<'run> {
        let llvm_args = arg_values.iter().map(|x| x.0.into()).collect::<Vec<_>>();
        let call_result = self
            .builder
            .build_indirect_call(func_type, function, &llvm_args, "result")
            .unwrap();
        if call_result.try_as_basic_value().is_basic() {
            let result_value: BasicValueEnum = call_result.as_any_value_enum().try_into().unwrap();
            SkObj::new(result_ty, result_value)
        } else {
            self.gen_const_ref(&toplevel_const("Void"), &ty::raw("Void"))
        }
    }

    /// Generate IR for HirArgRef.
    fn gen_arg_ref(&'run self, ctx: &mut CodeGenContext<'hir, 'run>, idx: &usize) -> SkObj<'run> {
        self._llvm_function_arg_ref(ctx, idx, false)
    }

    fn gen_tyarg_ref(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        n_params: &usize,
        idx: &usize,
    ) -> SkObj<'run> {
        self._llvm_function_arg_ref(ctx, &(n_params + idx), true)
    }

    fn _llvm_function_arg_ref(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        idx: &usize,
        is_tyarg: bool,
    ) -> SkObj<'run> {
        match ctx.function_origin {
            FunctionOrigin::Method { params } => {
                let ty = if is_tyarg {
                    ty::raw("Class")
                } else {
                    params[*idx].ty.clone()
                };
                SkObj::new(ty, ctx.function.get_nth_param((*idx as u32) + 1).unwrap())
                // +1 for the first %self
            }
            FunctionOrigin::Lambda { params, .. } => {
                let ty = if is_tyarg {
                    ty::raw("Class")
                } else {
                    params[*idx].ty.clone()
                };
                // +1 for the first %self
                let obj = self.get_nth_param(ty.clone(), &ctx.function, *idx + 1);
                // Bitcast is needed because lambda params are always `%Object*`
                self.bitcast(obj, &ty, "value")
            }
            _ => panic!("[BUG] arg ref in invalid place"),
        }
    }

    fn gen_lvar_ref(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        ty: &TermTy,
        name: &str,
    ) -> SkObj<'run> {
        let ptr = ctx
            .lvars
            .get(name)
            .unwrap_or_else(|| panic!("[BUG] lvar `{}' not found in ctx.lvars", name));
        SkObj::new(
            ty.clone(),
            self.builder
                .build_load(self.llvm_type(), *ptr, name)
                .unwrap(),
        )
    }

    fn gen_ivar_ref(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        idx: &usize,
        name: &str,
        self_ty: &TermTy,
        ty: &TermTy,
    ) -> SkObj<'run> {
        let object = self.gen_self_expression(ctx, self_ty);
        self.build_ivar_load(object, ty.clone(), *idx, name)
    }

    fn gen_class_tvar_ref(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        typaram_ref: &TyParamRef,
        self_ty: &TermTy,
        expr_ty: &TermTy,
    ) -> SkObj<'run> {
        debug_assert!(typaram_ref.kind == TyParamKind::Class);
        let self_obj = self.gen_self_expression(ctx, self_ty);
        self.bitcast(
            self._get_nth_tyarg_of_self(self_obj, typaram_ref.idx),
            expr_ty,
            "as",
        )
    }

    fn _get_nth_tyarg_of_self(&'run self, self_obj: SkObj<'run>, idx: usize) -> SkObj<'run> {
        let cls_obj = self.get_class_of_obj(self_obj);
        self.call_method_func(
            &method_fullname_raw("Class", "_type_argument"),
            self.bitcast(cls_obj.as_sk_obj(), &ty::raw("Class"), "as"),
            &vec![self.gen_decimal_literal(idx as i64)],
            ty::raw("Class"),
            "tyarg",
        )
    }

    fn gen_method_tvar_ref(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        typaram_ref: &TyParamRef,
        n_params: &usize,
    ) -> SkObj<'run> {
        debug_assert!(typaram_ref.kind == TyParamKind::Method);
        let idx = 1 + // %self
            *n_params +
            typaram_ref.idx;
        SkObj::new(
            typaram_ref.to_term_ty(),
            ctx.function.get_nth_param(idx as u32).unwrap(),
        )
    }

    pub fn gen_const_ref(&'run self, fullname: &ConstFullname, ty: &TermTy) -> SkObj<'run> {
        let name = llvm_const_name(fullname);
        let llvm_type = self.llvm_type();
        let ptr = self
            .module
            .get_global(&name)
            .unwrap_or_else(|| panic!("[BUG] global for Constant `{}' not created", fullname));
        SkObj::new(
            ty.clone(),
            self.builder
                .build_load(llvm_type, ptr.as_pointer_value(), &name)
                .unwrap(),
        )
    }

    fn gen_lambda_expr(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        name: &str,
        params: &[MethodParam],
        captures: &'hir [HirLambdaCapture],
        ret_ty: &TermTy,
    ) -> SkObj<'run> {
        let func_name = LlvmFuncName(name.to_string());
        let fn_x_type = &ty::raw(&format!("Fn{}", params.len()));
        let obj_type = ty::raw("Object");
        let mut arg_types = (1..=params.len()).map(|_| &obj_type).collect::<Vec<_>>();
        arg_types.insert(0, fn_x_type);
        let func_type = self.llvm_func_type(None, &arg_types, ret_ty);
        self.module.add_function(&func_name.0, func_type, None);

        // eg. Fn1.new(fnptr, the_self, captures)
        let cls_name = format!("Fn{}", params.len());
        let meta = self.gen_const_ref(&toplevel_const(&cls_name), &ty::meta(&cls_name));
        let fnptr = self
            .get_llvm_func(&func_name)
            .as_global_value()
            .as_basic_value_enum();
        let fnptr_i8 = self
            .builder
            .build_bit_cast(fnptr, self.ptr_type, "")
            .unwrap();
        let sk_ptr = self.box_i8ptr(fnptr_i8);
        let the_self = self.gen_self_expression(ctx, &ty::raw("Object"));
        let captured = self._gen_lambda_captures(ctx, name, captures);
        let arg_values = vec![sk_ptr, the_self, captured.boxed(self)];
        self.call_method_func(
            &method_fullname(metaclass_fullname(cls_name).into(), "new"),
            meta,
            &arg_values,
            fn_x_type.clone(),
            "lambda",
        )
    }

    fn _gen_lambda_captures(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        name: &str,
        captures: &'hir [HirLambdaCapture],
    ) -> LambdaCapture<'run> {
        let struct_type = LambdaCapture::get_struct_type(self, name);
        let mem = self.allocate_mem(&struct_type.as_basic_type_enum());
        let lambda_capture = LambdaCapture::from_void_ptr(self, mem, name);

        for (i, cap) in captures.iter().enumerate() {
            let mut item = match &cap.detail {
                HirLambdaCaptureDetail::CaptureLVar { name } => {
                    if cap.readonly {
                        self.gen_lvar_ref(ctx, &cap.ty, name)
                    } else {
                        // Captured by pointer to be reassigned
                        SkObj::new(cap.ty.clone(), *ctx.lvars.get(name).unwrap())
                    }
                }
                HirLambdaCaptureDetail::CaptureArg { idx } => {
                    // Args are captured by value
                    self.gen_arg_ref(ctx, idx)
                }
                HirLambdaCaptureDetail::CaptureFwd { cidx, .. } => {
                    let deref = false;
                    self.gen_lambda_capture_ref(ctx, cidx, deref, &cap.ty)
                }
                HirLambdaCaptureDetail::CaptureMethodTyArg { idx, n_params } => {
                    // Method-wise type arguments are passed as llvm function parameter.
                    self.gen_tyarg_ref(ctx, n_params, idx)
                }
            };
            if cap.upcast_needed {
                item = self.bitcast(item, &cap.ty, "upcast_needed");
            }
            lambda_capture.store(self, i, item);
        }
        lambda_capture
    }

    /// Get the object referred by `self`
    /// `ty` is needed for bitcast (because the type information is lost
    /// in a lambda)
    fn gen_self_expression(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        ty: &TermTy,
    ) -> SkObj<'run> {
        let the_self = if ctx.function.get_name().to_str().unwrap() == "user_main" {
            self.the_main.clone().unwrap()
        } else if matches!(ctx.function_origin, FunctionOrigin::Lambda { .. }) {
            let fn_x = self.get_nth_param(ty::raw("Fn"), &ctx.function, 0);
            self.build_ivar_load(
                fn_x,
                ty::raw("Object"),
                fn_x::IVAR_THE_SELF_IDX,
                "@the_self",
            )
        } else {
            self.get_nth_param(ty.clone(), &ctx.function, 0)
        };
        self.bitcast(the_self, ty, "the_main")
    }

    fn gen_float_literal(&self, value: f64) -> SkObj<'run> {
        self.box_float(&self.f64_type.const_float(value))
    }

    fn gen_decimal_literal(&self, value: i64) -> SkObj<'run> {
        self.box_int(&self.i64_type.const_int(value as u64, false))
    }

    /// Create a string object
    fn gen_string_literal(&'run self, idx: &usize) -> SkObj<'run> {
        let byte_ary = self
            .module
            .get_global(&format!("str_{}", idx))
            .unwrap_or_else(|| panic!("[BUG] global for str_{} not created", idx))
            .as_pointer_value();
        let i8ptr = self
            .builder
            .build_bit_cast(byte_ary, self.ptr_type, "i8ptr")
            .unwrap();
        let bytesize = self
            .i64_type
            .const_int(self.str_literals[*idx].len() as u64, false);
        SkObj::new(
            ty::raw("String"),
            self.call_llvm_func(
                &llvm_func_name("gen_literal_string"),
                &[i8ptr.into(), bytesize.into()],
                "sk_str",
            ),
        )
    }

    fn gen_boolean_literal(&'run self, value: bool) -> SkObj<'run> {
        let n = if value { 1 } else { 0 };
        let i = self.i1_type.const_int(n, false);
        self.box_bool(i)
    }

    /// Generate conditional branch by Shiika Bool
    fn gen_conditional_branch(
        &'run self,
        cond: SkObj,
        then_block: inkwell::basic_block::BasicBlock,
        else_block: inkwell::basic_block::BasicBlock,
    ) {
        let i = self.unbox_bool(cond);
        let one = self.i1_type.const_int(1, false);
        let istrue = self
            .builder
            .build_int_compare(inkwell::IntPredicate::EQ, i, one, "istrue")
            .unwrap();
        self.builder
            .build_conditional_branch(istrue, then_block, else_block);
    }

    /// Get an object from `captures`
    fn gen_lambda_capture_ref(
        &'run self,
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
        let ret = captures.get_value(self, *idx_in_captures, ty, deref);
        let block = self.context.append_basic_block(
            ctx.function,
            &format!("CaptureRef_{}th_end", idx_in_captures),
        );
        self.builder.build_unconditional_branch(block);
        self.builder.position_at_end(block);
        ret
    }

    fn gen_lambda_capture_write(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        idx_in_captures: &usize,
        rhs: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>> {
        let block = self
            .context
            .append_basic_block(ctx.function, &format!("CaptureWrite_{}th", idx_in_captures));
        self.builder.build_unconditional_branch(block);
        self.builder.position_at_end(block);

        let captures = self._gen_get_lambda_captures(ctx);
        let value = self.gen_expr(ctx, rhs)?.unwrap();
        captures.reassign(self, *idx_in_captures, value.clone());

        let block = self.context.append_basic_block(
            ctx.function,
            &format!("CaptureWrite_{}th_end", idx_in_captures),
        );
        self.builder.build_unconditional_branch(block);
        self.builder.position_at_end(block);
        Ok(Some(value))
    }

    fn _gen_get_lambda_captures(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
    ) -> LambdaCapture<'run> {
        let fn_x = self.get_nth_param(ty::raw("Fn"), &ctx.function, 0);
        let boxed = self.build_ivar_load(
            fn_x,
            ty::raw("Shiika::Internal::Ptr"),
            fn_x::IVAR_CAPTURES_IDX,
            "@captures",
        );
        LambdaCapture::from_boxed(self, boxed, ctx.lambda_name().unwrap())
    }

    fn gen_bitcast(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        expr: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>> {
        // Just compile the expr (nothing to do more in the runtime)
        self.gen_expr(ctx, expr)
    }

    /// Create a class object
    /// ("class literal" is a special Hir that does not appear directly
    /// on a source text.)
    fn gen_class_literal(
        &'run self,
        fullname: &TypeFullname,
        clsobj_ty: &TermTy,
        str_literal_idx: &usize,
        includes_modules: &bool,
        initializer: &Option<MethodSignature>,
    ) -> SkObj<'run> {
        debug_assert!(!fullname.is_meta());
        if fullname.0 == "Metaclass" {
            self.gen_the_metaclass(str_literal_idx)
        } else {
            // Create metaclass object (eg. `#<metaclass Int>`) with `Metaclass.new`
            let the_metaclass =
                self.gen_const_ref(&toplevel_const("Metaclass"), &ty::raw("Metaclass"));
            let receiver = self.null_ptr(&ty::meta("Metaclass"));
            let vtable = self
                .get_vtable_of_class(&class_fullname("Metaclass"))
                .as_object_ptr(self);
            let wtable = SkObj::nullptr(self);
            let metacls_obj = self.call_method_func(
                &method_fullname_raw("Metaclass", "_new"),
                receiver,
                &vec![
                    self.gen_string_literal(str_literal_idx),
                    vtable,
                    wtable,
                    self.bitcast(the_metaclass, &ty::raw("Metaclass"), "as"),
                    self.null_ptr(&ty::raw("Class")),
                ],
                ty::raw("Metaclass"),
                "meta",
            );

            // Create the class object (eg. `#<class Int>`, which is the value of `::Int`)
            let receiver = self.null_ptr(&ty::meta("Class"));
            let vtable = self
                .get_vtable_of_class(&fullname.meta_name())
                .as_object_ptr(self);
            let wtable = SkObj::nullptr(self);
            let cls = self.call_method_func(
                &method_fullname(metaclass_fullname("Class").into(), "_new"),
                receiver,
                &vec![
                    self.gen_string_literal(str_literal_idx),
                    vtable,
                    wtable,
                    self.bitcast(metacls_obj, &ty::raw("Metaclass"), "as"),
                    self.null_ptr(&ty::raw("Class")),
                ],
                ty::raw("Class"),
                "cls",
            );
            if *includes_modules {
                let fname = wtable::insert_wtable_func_name(&fullname.clone().to_class_fullname());
                self.call_void_llvm_func(&llvm_func_name(fname), &[cls.0.into()], "_");
            }
            self.call_class_level_initialize(cls.clone(), initializer);

            self.bitcast(cls, clsobj_ty, "as")
        }
    }

    fn call_class_level_initialize(
        &'run self,
        receiver: SkObj,
        initializer: &Option<MethodSignature>,
    ) {
        if let Some(initialize_sig) = initializer {
            let init_cls_name = initialize_sig.fullname.type_name.to_class_fullname();
            let addr = self.bitcast(receiver, &init_cls_name.to_ty(), "obj_as_super");
            let args = vec![addr.0.into()];
            let initialize = self.get_llvm_func(&method_func_name(&initialize_sig.fullname));
            self.builder.build_direct_call(initialize, &args, "");
        }
    }

    /// Create the metaclass object `Metaclass`
    fn gen_the_metaclass(&'run self, str_literal_idx: &usize) -> SkObj<'run> {
        // We need a trick here to achieve `Metaclass.class == Metaclass`.
        let null = SkClassObj::nullptr(self);
        let cls_obj = self._allocate_sk_obj(&class_fullname("Metaclass"), null);
        self.build_ivar_store(
            cls_obj.clone(),
            skc_corelib::class::IVAR_NAME_IDX,
            "@name",
            self.gen_string_literal(str_literal_idx),
        );
        self.set_class_of_obj(&cls_obj, SkClassObj(cls_obj.0));
        cls_obj
    }

    /// Compile successive expressions. The last evaluated value is returned.
    /// Returns `None` if terminated with a `Never` type (`return`, `panic`, etc.)
    fn gen_parenthesized_expr(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        exprs: &'hir [HirExpression],
    ) -> Result<Option<SkObj<'run>>> {
        debug_assert!(!exprs.is_empty());
        let mut last_value = None;
        for expr in exprs {
            let value = self.gen_expr(ctx, expr)?;
            if value.is_none() {
                // Found `return`, `panic` or something. The rest of `exprs`
                // will never be executed
                return Ok(None);
            } else {
                last_value = Some(value);
            }
        }
        Ok(last_value.unwrap())
    }

    /// Returns a special value (currently a nullptr) that denotes using the default argument value.
    fn gen_default_expr(&'run self, ty: &TermTy) -> SkObj<'run> {
        self.null_ptr(ty)
    }

    /// Returns true if `expr` evaluates to a special value (currently a nullptr) that denotes using the default argument value.
    fn gen_is_omitted_value(
        &'run self,
        ctx: &mut CodeGenContext<'hir, 'run>,
        expr: &'hir HirExpression,
    ) -> Result<Option<SkObj<'run>>> {
        let v = self.gen_expr(ctx, expr)?.unwrap();
        let i1 = self.builder.build_is_null(v.0, "omitted").unwrap();
        Ok(Some(self.box_bool(i1)))
    }
}
