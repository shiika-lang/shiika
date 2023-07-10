use crate::utils::{lambda_capture_struct_name, LlvmFuncName};
use crate::values::{I8Ptr, SkObj};
use crate::CodeGen;
use anyhow::Result;
use either::Either::*;
use inkwell::types::AnyType;
use inkwell::types::BasicType;
use shiika_core::ty::*;
use skc_hir::HirExpressionBase::*;
use skc_hir::*;

/// A lambda capture
#[derive(Debug)]
pub struct LambdaCapture<'run> {
    lambda_name: String,
    /// Pointer to the struct
    raw: inkwell::values::PointerValue<'run>,
}

impl<'run> LambdaCapture<'run> {
    /// Returns LLVM struct type for a lambda
    pub fn get_struct_type<'ictx>(gen: &CodeGen, name: &str) -> inkwell::types::StructType<'ictx> {
        gen.context
            .get_struct_type(&lambda_capture_struct_name(name))
            .unwrap()
    }

    pub fn struct_ptr_type<'ictx>(gen: &CodeGen, name: &str) -> inkwell::types::PointerType<'ictx> {
        Self::get_struct_type(gen, name).ptr_type(inkwell::AddressSpace::Generic)
    }

    fn new(
        gen: &CodeGen,
        lambda_name: String,
        raw: inkwell::values::PointerValue<'run>,
    ) -> LambdaCapture<'run> {
        debug_assert!(raw.get_type() == Self::struct_ptr_type(gen, &lambda_name));
        LambdaCapture { lambda_name, raw }
    }

    pub fn from_boxed(
        gen: &CodeGen<'_, 'run, '_>,
        boxed: SkObj<'run>,
        name: &str,
    ) -> LambdaCapture<'run> {
        LambdaCapture::from_void_ptr(gen, gen.unbox_i8ptr(boxed), name)
    }

    pub fn from_void_ptr(
        gen: &CodeGen<'_, 'run, '_>,
        p: I8Ptr<'run>,
        name: &str,
    ) -> LambdaCapture<'run> {
        let t = Self::struct_ptr_type(gen, name);
        LambdaCapture::new(gen, name.to_string(), p.cast_to(gen, t))
    }

    /// Box `self` with Shiika::Internal::Ptr
    pub fn boxed(&self, gen: &CodeGen<'_, 'run, '_>) -> SkObj<'run> {
        self.to_void_ptr(gen).boxed(gen)
    }

    /// Returns the address of `self` as void pointer
    fn to_void_ptr(&self, gen: &CodeGen<'_, 'run, '_>) -> I8Ptr<'run> {
        I8Ptr::cast(gen, self.to_struct_ptr())
    }

    /// Returns the address of `self`
    fn to_struct_ptr(&self) -> inkwell::values::PointerValue<'run> {
        self.raw
    }

    fn struct_type<'ictx>(&self, gen: &CodeGen) -> inkwell::types::StructType<'ictx> {
        Self::get_struct_type(gen, &self.lambda_name)
    }

    /// Store `value` at the given index
    pub fn store(&self, gen: &CodeGen, idx: usize, value: inkwell::values::BasicValueEnum<'run>) {
        debug_assert!(self.store_type_matches(gen, idx, value));

        gen.build_llvm_struct_set(
            self.to_struct_ptr(),
            idx,
            value,
            &format!("capture_{}th", idx),
        );
    }

    /// Asserts that the value is right type
    fn store_type_matches(
        &self,
        gen: &CodeGen,
        idx: usize,
        value: inkwell::values::BasicValueEnum<'run>,
    ) -> bool {
        let value_ty = value.get_type().as_any_type_enum();
        let ptr_ty = self
            .struct_type(gen)
            .get_field_type_at_index(idx as u32)
            .unwrap()
            .into_pointer_type();

        if value_ty == ptr_ty.as_any_type_enum() {
            true
        } else {
            dbg!(&value_ty);
            dbg!(&ptr_ty);
            false
        }
    }

    /// Load the value at the given index
    pub fn load(
        &self,
        gen: &CodeGen<'_, 'run, '_>,
        idx: usize,
    ) -> inkwell::values::BasicValueEnum<'run> {
        gen.build_llvm_struct_ref(self.to_struct_ptr(), idx, "load")
    }

    /// Given there is a pointer stored at `idx`, update its value.
    pub fn reassign(&self, gen: &CodeGen<'_, 'run, '_>, idx: usize, value: SkObj) {
        // eg. `%Int**`
        let ptr_ty = self
            .struct_type(gen)
            .get_field_type_at_index(idx as u32)
            .unwrap()
            .into_pointer_type();
        // eg. `%Int*`
        let ty = ptr_ty.get_element_type().into_pointer_type();
        let upcast = gen.builder.build_bitcast(value.0, ty, "upcast");

        let ptr = self.load(gen, idx).into_pointer_value();
        gen.builder.build_store(ptr, upcast);
    }
}

impl<'hir: 'ictx, 'run, 'ictx: 'run> CodeGen<'hir, 'run, 'ictx> {
    /// Find all lambdas in a hir and create the body of the corresponding llvm function
    /// PERF: Ideally they should be created during gen_methods but I couldn't
    /// avoid borrow checker errors.
    pub(super) fn gen_lambda_funcs(&self, hir: &'hir Hir) -> Result<()> {
        for methods in hir.sk_methods.values() {
            for method in methods {
                if let SkMethodBody::Normal { exprs } = &method.body {
                    self.gen_lambda_funcs_in_exprs(&exprs.exprs)?;
                }
            }
        }

        for expr in &hir.const_inits {
            self.gen_lambda_funcs_in_expr(expr)?;
        }

        self.gen_lambda_funcs_in_exprs(&hir.main_exprs.exprs)?;
        Ok(())
    }

    fn gen_lambda_funcs_in_exprs(&self, exprs: &'hir [HirExpression]) -> Result<()> {
        for expr in exprs {
            self.gen_lambda_funcs_in_expr(expr)?;
        }
        Ok(())
    }

    fn gen_lambda_funcs_in_expr(&self, expr: &'hir HirExpression) -> Result<()> {
        match &expr.node {
            HirLogicalNot { expr } => self.gen_lambda_funcs_in_expr(expr)?,
            HirLogicalAnd { left, right } => {
                self.gen_lambda_funcs_in_expr(left)?;
                self.gen_lambda_funcs_in_expr(right)?;
            }
            HirLogicalOr { left, right } => {
                self.gen_lambda_funcs_in_expr(left)?;
                self.gen_lambda_funcs_in_expr(right)?;
            }
            HirIfExpression {
                cond_expr,
                then_exprs,
                else_exprs,
                ..
            } => {
                self.gen_lambda_funcs_in_expr(cond_expr)?;
                self.gen_lambda_funcs_in_exprs(&then_exprs.exprs)?;
                self.gen_lambda_funcs_in_exprs(&else_exprs.exprs)?;
            }
            HirMatchExpression {
                cond_assign_expr,
                clauses,
            } => {
                self.gen_lambda_funcs_in_expr(cond_assign_expr)?;
                for clause in clauses {
                    self.gen_lambda_funcs_in_exprs(&clause.body_hir.exprs)?;
                }
            }
            HirWhileExpression {
                cond_expr,
                body_exprs,
                ..
            } => {
                self.gen_lambda_funcs_in_expr(cond_expr)?;
                self.gen_lambda_funcs_in_exprs(&body_exprs.exprs)?;
            }
            HirBreakExpression { .. } => (),
            HirReturnExpression { arg, .. } => self.gen_lambda_funcs_in_expr(arg)?,
            HirLVarAssign { rhs, .. } => self.gen_lambda_funcs_in_expr(rhs)?,
            HirIVarAssign { rhs, .. } => self.gen_lambda_funcs_in_expr(rhs)?,
            HirConstAssign { rhs, .. } => self.gen_lambda_funcs_in_expr(rhs)?,
            HirMethodCall {
                receiver_expr,
                arg_exprs,
                ..
            } => {
                self.gen_lambda_funcs_in_expr(receiver_expr)?;
                for expr in arg_exprs {
                    self.gen_lambda_funcs_in_expr(expr)?;
                }
            }
            HirModuleMethodCall {
                receiver_expr,
                arg_exprs,
                ..
            } => {
                self.gen_lambda_funcs_in_expr(receiver_expr)?;
                for expr in arg_exprs {
                    self.gen_lambda_funcs_in_expr(expr)?;
                }
            }
            HirLambdaInvocation {
                lambda_expr,
                arg_exprs,
            } => {
                self.gen_lambda_funcs_in_expr(lambda_expr)?;
                for expr in arg_exprs {
                    self.gen_lambda_funcs_in_expr(expr)?;
                }
            }
            HirArgRef { .. } => (),
            HirLVarRef { .. } => (),
            HirIVarRef { .. } => (),
            HirTVarRef { .. } => (),
            HirConstRef { .. } => (),
            HirLambdaExpr {
                name,
                params,
                exprs,
                ret_ty,
                lvars,
                ..
            } => {
                self.gen_lambda_func(&name, params, exprs, ret_ty, lvars)?;
                self.gen_lambda_funcs_in_exprs(&exprs.exprs)?;
            }
            HirSelfExpression => (),
            HirFloatLiteral { .. } => (),
            HirDecimalLiteral { .. } => (),
            HirStringLiteral { .. } => (),
            HirBooleanLiteral { .. } => (),

            HirLambdaCaptureRef { .. } => (),
            HirLambdaCaptureWrite { rhs, .. } => self.gen_lambda_funcs_in_expr(rhs)?,
            HirBitCast { expr } => self.gen_lambda_funcs_in_expr(expr)?,
            HirClassLiteral { .. } => (),
            HirParenthesizedExpr { exprs } => self.gen_lambda_funcs_in_exprs(&exprs.exprs)?,
            HirDefaultExpr { .. } => (),
            HirIsOmittedValue { expr, .. } => self.gen_lambda_funcs_in_expr(&expr)?,
        }
        Ok(())
    }

    fn gen_lambda_func(
        &self,
        name: &str,
        params: &'hir [MethodParam],
        exprs: &'hir HirExpressions,
        ret_ty: &TermTy,
        lvars: &HirLVars,
    ) -> Result<()> {
        let func_name = LlvmFuncName(name.to_string());
        self.gen_llvm_func_body(
            &func_name,
            params,
            Default::default(),
            Right(exprs),
            lvars,
            ret_ty,
            Some(name.to_string()),
        )
    }

    /// TODO: refactor (preprocess in MIR)
    pub(super) fn gen_lambda_capture_structs(&self, hir: &'hir Hir) -> Result<()> {
        for methods in hir.sk_methods.values() {
            for method in methods {
                if let SkMethodBody::Normal { exprs } = &method.body {
                    self.gen_lambda_capture_structs_in_exprs(&exprs.exprs)?;
                }
            }
        }

        for expr in &hir.const_inits {
            self.gen_lambda_capture_structs_in_expr(expr)?;
        }

        self.gen_lambda_capture_structs_in_exprs(&hir.main_exprs.exprs)?;
        Ok(())
    }

    fn gen_lambda_capture_structs_in_exprs(&self, exprs: &'hir [HirExpression]) -> Result<()> {
        for expr in exprs {
            self.gen_lambda_capture_structs_in_expr(expr)?;
        }
        Ok(())
    }

    fn gen_lambda_capture_structs_in_expr(&self, expr: &'hir HirExpression) -> Result<()> {
        match &expr.node {
            HirLogicalNot { expr } => self.gen_lambda_capture_structs_in_expr(expr)?,
            HirLogicalAnd { left, right } => {
                self.gen_lambda_capture_structs_in_expr(left)?;
                self.gen_lambda_capture_structs_in_expr(right)?;
            }
            HirLogicalOr { left, right } => {
                self.gen_lambda_capture_structs_in_expr(left)?;
                self.gen_lambda_capture_structs_in_expr(right)?;
            }
            HirIfExpression {
                cond_expr,
                then_exprs,
                else_exprs,
                ..
            } => {
                self.gen_lambda_capture_structs_in_expr(cond_expr)?;
                self.gen_lambda_capture_structs_in_exprs(&then_exprs.exprs)?;
                self.gen_lambda_capture_structs_in_exprs(&else_exprs.exprs)?;
            }
            HirMatchExpression {
                cond_assign_expr,
                clauses,
            } => {
                self.gen_lambda_capture_structs_in_expr(cond_assign_expr)?;
                for clause in clauses {
                    self.gen_lambda_capture_structs_in_exprs(&clause.body_hir.exprs)?;
                }
            }
            HirWhileExpression {
                cond_expr,
                body_exprs,
                ..
            } => {
                self.gen_lambda_capture_structs_in_expr(cond_expr)?;
                self.gen_lambda_capture_structs_in_exprs(&body_exprs.exprs)?;
            }
            HirBreakExpression { .. } => (),
            HirReturnExpression { arg, .. } => self.gen_lambda_capture_structs_in_expr(arg)?,
            HirLVarAssign { rhs, .. } => self.gen_lambda_capture_structs_in_expr(rhs)?,
            HirIVarAssign { rhs, .. } => self.gen_lambda_capture_structs_in_expr(rhs)?,
            HirConstAssign { rhs, .. } => self.gen_lambda_capture_structs_in_expr(rhs)?,
            HirMethodCall {
                receiver_expr,
                arg_exprs,
                ..
            } => {
                self.gen_lambda_capture_structs_in_expr(receiver_expr)?;
                for expr in arg_exprs {
                    self.gen_lambda_capture_structs_in_expr(expr)?;
                }
            }
            HirModuleMethodCall {
                receiver_expr,
                arg_exprs,
                ..
            } => {
                self.gen_lambda_capture_structs_in_expr(receiver_expr)?;
                for expr in arg_exprs {
                    self.gen_lambda_capture_structs_in_expr(expr)?;
                }
            }
            HirLambdaInvocation {
                lambda_expr,
                arg_exprs,
            } => {
                self.gen_lambda_capture_structs_in_expr(lambda_expr)?;
                for expr in arg_exprs {
                    self.gen_lambda_capture_structs_in_expr(expr)?;
                }
            }
            HirArgRef { .. } => (),
            HirLVarRef { .. } => (),
            HirIVarRef { .. } => (),
            HirTVarRef { .. } => (),
            HirConstRef { .. } => (),
            HirLambdaExpr {
                name,
                exprs,
                captures,
                ..
            } => {
                self.gen_lambda_capture_struct(name, captures)?;
                self.gen_lambda_capture_structs_in_exprs(&exprs.exprs)?;
            }
            HirSelfExpression => (),
            HirFloatLiteral { .. } => (),
            HirDecimalLiteral { .. } => (),
            HirStringLiteral { .. } => (),
            HirBooleanLiteral { .. } => (),

            HirLambdaCaptureRef { .. } => (),
            HirLambdaCaptureWrite { rhs, .. } => self.gen_lambda_capture_structs_in_expr(rhs)?,
            HirBitCast { expr } => self.gen_lambda_capture_structs_in_expr(expr)?,
            HirClassLiteral { .. } => (),
            HirParenthesizedExpr { exprs } => {
                self.gen_lambda_capture_structs_in_exprs(&exprs.exprs)?
            }
            HirDefaultExpr { .. } => (),
            HirIsOmittedValue { expr, .. } => self.gen_lambda_capture_structs_in_expr(&expr)?,
        }
        Ok(())
    }

    fn gen_lambda_capture_struct(&self, name: &str, captures: &[HirLambdaCapture]) -> Result<()> {
        let struct_name = lambda_capture_struct_name(name);
        let struct_type = self.context.opaque_struct_type(&struct_name);
        let body = captures
            .iter()
            .map(|cap| self.capture_ty(cap))
            .collect::<Vec<_>>();
        struct_type.set_body(&body, false);
        Ok(()) // REFACTOR: not needed to be a Result
    }

    fn capture_ty(&self, cap: &HirLambdaCapture) -> inkwell::types::BasicTypeEnum {
        if cap.readonly {
            self.llvm_type(&cap.ty)
        } else {
            // The (local) variable is captured by reference.
            // PERF: not needed to be by-ref when the variable is declared with
            // `var` but not reassigned from closure.
            self.llvm_type(&cap.ty)
                .ptr_type(inkwell::AddressSpace::Generic)
                .as_basic_type_enum()
        }
    }
}
