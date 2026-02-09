use crate::codegen::{
    cell, codegen_context::CodeGenContext, constants, instance, intrinsics, item, llvm_struct,
    string_literal, type_object, value::SkObj, wtable, CodeGen,
};
use crate::mir;
use crate::names::FunctionName;
use anyhow::Result;
use inkwell::types::BasicType;
use inkwell::values::{AnyValue, BasicValue, BasicValueEnum};
use shiika_core::ty::TermTy;

impl<'run, 'ictx: 'run> CodeGen<'run, 'ictx> {
    pub fn compile_extern_funcs(&mut self, externs: Vec<mir::Extern>) {
        for e in externs {
            self.compile_extern(e);
        }
    }

    pub fn compile_program(&mut self, funcs: Vec<mir::Function>) -> item::MethodFuncs {
        for f in &funcs {
            self.declare_func(f);
        }
        for f in funcs {
            self.compile_func(f);
        }
        item::MethodFuncs()
    }

    fn compile_extern(&self, ext: mir::Extern) {
        let func_type = self.llvm_function_type(&ext.fun_ty);
        self.module
            .add_function(&ext.name.mangle(), func_type, None);
    }

    fn declare_func(&self, f: &mir::Function) {
        let func_type = self.llvm_function_type(&f.fun_ty());
        self.module.add_function(&f.name.mangle(), func_type, None);
    }

    fn compile_func(&mut self, f: mir::Function) {
        log::info!("Compiling function {:?}", f.name);
        let function = self.get_llvm_func(&f.name);

        // Set param names
        for (i, param) in function.get_param_iter().enumerate() {
            let name = f.params[i].name.as_str();
            inkwell_set_name(param, name);
        }

        let basic_block = self.context.append_basic_block(function, "");
        self.builder.position_at_end(basic_block);

        let mut ctx = CodeGenContext {
            function,
            lvars: Default::default(),
        };

        let _ = self.compile_expr(&mut ctx, &f.body_stmts).unwrap();
    }

    fn compile_value_expr(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        texpr: &mir::TypedExpr,
    ) -> inkwell::values::BasicValueEnum<'run> {
        match self.compile_expr(ctx, texpr).unwrap() {
            Some(v) => v,
            None => panic!("this expression does not have value"),
        }
    }

    pub fn compile_expr(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        texpr: &mir::TypedExpr,
    ) -> Result<Option<inkwell::values::BasicValueEnum<'run>>> {
        Ok(match &texpr.0 {
            mir::Expr::Number(n) => self.compile_number(*n),
            mir::Expr::StringLiteral(s) => self.compile_string_literal(s),
            mir::Expr::CreateNativeArray(elems) => self.compile_create_native_array(ctx, elems)?,
            mir::Expr::NativeArrayRef(arr, idx) => self.compile_native_array_ref(ctx, arr, *idx)?,
            mir::Expr::PseudoVar(pvar) => Some(self.compile_pseudo_var(pvar)),
            mir::Expr::LVarRef(name) => self.compile_lvarref(ctx, name, &texpr.1),
            mir::Expr::IVarRef(obj_expr, idx, name) => {
                self.compile_ivarref(ctx, obj_expr, *idx, name)
            }
            mir::Expr::ArgRef(idx, _) => self.compile_argref(ctx, idx),
            mir::Expr::EnvRef(_, _) | mir::Expr::EnvSet(_, _, _) => {
                panic!("should be lowered before codegen.rs")
            }
            mir::Expr::ConstRef(name) => self.compile_constref(name),
            mir::Expr::FuncRef(name) => self.compile_funcref(name),
            mir::Expr::FunCall(fexpr, arg_exprs) => self.compile_funcall(ctx, fexpr, arg_exprs),
            mir::Expr::GetVTable(receiver) => self.compile_get_vtable(ctx, receiver),
            mir::Expr::VTableRef(_, _, _) => {
                panic!("VTableRef should be lowered before codegen")
            }
            mir::Expr::WTableKey(modname) => self.compile_wtable_key(modname),
            mir::Expr::WTableRow(classname, modname) => self.compile_wtable_row(classname, modname),
            mir::Expr::WTableRef(receiver, module, idx, _debug_name) => {
                self.compile_wtable_ref(ctx, receiver, module, *idx)
            }
            mir::Expr::If(cond, then, els) => self.compile_if(ctx, cond, then, els)?,
            mir::Expr::While(cond, exprs) => self.compile_while(ctx, cond, exprs)?,
            mir::Expr::Spawn(_) => todo!(),
            mir::Expr::Alloc(name, ty) => self.compile_alloc(ctx, name, ty),
            mir::Expr::LVarDecl(name, rhs, _) | mir::Expr::LVarSet(name, rhs) => {
                self.compile_lvar_set(ctx, name, rhs)?
            }
            mir::Expr::IVarSet(obj_expr, idx, rhs, name) => {
                self.compile_ivar_set(ctx, obj_expr, *idx, rhs, name)?
            }
            mir::Expr::ConstSet(name, rhs) => self.compile_const_set(ctx, name, rhs)?,
            mir::Expr::Return(val_expr) => {
                self.compile_return(ctx, val_expr.as_ref().map(|v| &**v))?
            }
            mir::Expr::Exprs(exprs) => self.compile_exprs(ctx, exprs)?,
            mir::Expr::Cast(cast_type, expr) => self.compile_cast(ctx, cast_type, expr),
            mir::Expr::CreateObject(type_name) => self.compile_create_object(type_name)?,
            mir::Expr::CreateTypeObject(the_ty) => self.compile_create_type_object(ctx, the_ty)?,
            mir::Expr::CellNew(value_expr) => self.compile_cell_new(ctx, value_expr),
            mir::Expr::CellGet(cell_expr) => self.compile_cell_get(ctx, cell_expr, &texpr.1),
            mir::Expr::CellSet(cell_expr, value_expr) => {
                self.compile_cell_set(ctx, cell_expr, value_expr)
            }
            mir::Expr::Unbox(expr) => self.compile_unbox(ctx, expr),
            mir::Expr::RawI64(n) => self.compile_raw_i64(*n),
            mir::Expr::Nop => None,
        })
    }

    fn compile_number(&mut self, n: i64) -> Option<inkwell::values::BasicValueEnum<'run>> {
        Some(intrinsics::box_int(self, n).into())
    }

    fn compile_string_literal(&mut self, s: &str) -> Option<inkwell::values::BasicValueEnum<'run>> {
        Some(string_literal::generate(self, s))
    }

    fn compile_create_native_array(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        elem_exprs: &[mir::TypedExpr],
    ) -> Result<Option<inkwell::values::BasicValueEnum<'run>>> {
        // Allocate memory for llvm array
        let element_count = elem_exprs.len();
        let array_ptr = instance::allocate_llvm_array(
            self,
            self.ptr_type().as_basic_type_enum(),
            element_count as u32,
        );

        // Insert each element
        for (i, elem_expr) in elem_exprs.iter().enumerate() {
            let compiled_elem = self.compile_value_expr(ctx, elem_expr);
            let idx = self.context.i64_type().const_int(i as u64, false);
            let elem_ptr = unsafe {
                self.builder
                    .build_gep(
                        self.ptr_type(),
                        array_ptr,
                        &[idx],
                        &format!("elem_ptr_{}", i),
                    )
                    .unwrap()
            };
            self.builder.build_store(elem_ptr, compiled_elem).unwrap();
        }

        Ok(Some(array_ptr.as_basic_value_enum()))
    }

    fn compile_native_array_ref(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        arr_expr: &mir::TypedExpr,
        idx: usize,
    ) -> Result<Option<inkwell::values::BasicValueEnum<'run>>> {
        let array_ptr = self.compile_value_expr(ctx, arr_expr);
        let idx_val = self.context.i64_type().const_int(idx as u64, false);
        let elem_ptr = unsafe {
            self.builder
                .build_gep(
                    self.ptr_type(),
                    array_ptr.into_pointer_value(),
                    &[idx_val],
                    &format!("elem_ptr_{}", idx),
                )
                .unwrap()
        };
        let elem_val = self
            .builder
            .build_load(self.ptr_type(), elem_ptr, &format!("elem_{}", idx))
            .unwrap();
        Ok(Some(elem_val))
    }

    fn compile_argref(
        &self,
        ctx: &mut CodeGenContext<'run>,
        idx: &usize,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let v = ctx.function.get_nth_param(*idx as u32).unwrap_or_else(|| {
            panic!(
                "argument at index {} not found in function {:?}",
                idx, ctx.function
            )
        });
        Some(v)
    }

    pub fn compile_constref(
        &self,
        name: &shiika_core::names::ConstFullname,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        Some(constants::load(self, name))
    }

    fn compile_funcref(
        &self,
        name: &FunctionName,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let f = self
            .get_llvm_func(&name)
            .as_global_value()
            .as_pointer_value();
        Some(f.into())
    }

    fn compile_pseudo_var(
        &mut self,
        pseudo_var: &mir::PseudoVar,
    ) -> inkwell::values::BasicValueEnum<'run> {
        match pseudo_var {
            mir::PseudoVar::True => intrinsics::box_bool(self, true).into(),
            mir::PseudoVar::False => intrinsics::box_bool(self, false).into(),
            mir::PseudoVar::Void => return self.compile_void(),
        }
    }

    fn compile_void(&mut self) -> inkwell::values::BasicValueEnum<'run> {
        // TODO: should be instance of Void
        intrinsics::box_bool(self, false).into()
    }

    fn compile_lvarref(
        &self,
        ctx: &mut CodeGenContext<'run>,
        name: &str,
        ty: &mir::Ty,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let v = ctx.lvars.get(name).unwrap();
        let t = self.llvm_type(ty);
        let v = self.builder.build_load(t, *v, name).unwrap();
        Some(v)
    }

    fn compile_ivarref(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        obj_expr: &mir::TypedExpr,
        idx: usize,
        name: &str,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let struct_ty = llvm_struct::of_ty(self, &obj_expr.1);
        let obj = self.compile_value_expr(ctx, obj_expr);
        Some(instance::build_ivar_load_raw(
            self,
            SkObj::from_basic_value_enum(obj),
            struct_ty,
            self.ptr_type().into(),
            idx,
            name,
        ))
    }

    fn compile_funcall(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        fexpr: &mir::TypedExpr,
        arg_exprs: &[mir::TypedExpr],
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let func = self.compile_value_expr(ctx, fexpr);
        let func_type = self.llvm_function_type(fexpr.1.as_fun_ty());
        let args = arg_exprs
            .iter()
            .map(|x| self.compile_value_expr(ctx, x).into())
            .collect::<Vec<_>>();
        let call_result = self
            .builder
            .build_indirect_call(func_type, func.into_pointer_value(), &args, "result")
            .unwrap();
        call_result.set_tail_call(true);

        call_result.as_any_value_enum().try_into().ok()
    }

    fn compile_get_vtable(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        receiver: &mir::TypedExpr,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let obj = self.compile_value_expr(ctx, receiver);
        let vtable = instance::get_vtable(self, &SkObj::from_basic_value_enum(obj));
        Some(vtable.ptr.into())
    }

    fn compile_wtable_ref(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        receiver: &mir::TypedExpr,
        module: &shiika_core::names::ModuleFullname,
        idx: usize,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let lookup_func = self
            .module
            .get_function("shiika_lookup_wtable")
            .unwrap_or_else(|| panic!("shiika_lookup_wtable function not found"));
        let args = {
            let receiver_obj = self.compile_value_expr(ctx, receiver);
            let key = wtable::get_module_key(self, module);
            let idx_val = self.context.i64_type().const_int(idx as u64, false);
            &[receiver_obj.into(), key.into(), idx_val.into()]
        };
        let call_result = self
            .builder
            .build_direct_call(lookup_func, args, "wtable_method")
            .unwrap();
        call_result.set_tail_call(true);
        Some(call_result.as_any_value_enum().try_into().unwrap())
    }

    fn compile_wtable_key(
        &mut self,
        modname: &shiika_core::names::ModuleFullname,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let key = wtable::get_module_key(self, modname);
        Some(key.into())
    }

    fn compile_wtable_row(
        &mut self,
        classname: &shiika_core::names::ClassFullname,
        modname: &shiika_core::names::ModuleFullname,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let funcs = wtable::load_wtable_const(self, classname, modname);
        Some(funcs.into())
    }

    /// Compile a sync if
    fn compile_if(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        cond_expr: &mir::TypedExpr,
        then_exprs: &mir::TypedExpr,
        else_exprs: &mir::TypedExpr,
    ) -> Result<Option<inkwell::values::BasicValueEnum<'run>>> {
        let begin_block = self.context.append_basic_block(ctx.function, "IfBegin");
        let then_block = self.context.append_basic_block(ctx.function, "IfThen");
        let else_block = self.context.append_basic_block(ctx.function, "IfElse");
        let merge_block = self.context.append_basic_block(ctx.function, "IfEnd");

        // IfBegin:
        self.builder.build_unconditional_branch(begin_block)?;
        self.builder.position_at_end(begin_block);
        let cond_value = self.compile_value_expr(ctx, cond_expr);
        self.gen_conditional_branch(cond_value, then_block, else_block);
        // IfThen:
        self.builder.position_at_end(then_block);
        let then_value = self.compile_expr(ctx, then_exprs)?;
        if then_value.is_some() {
            self.builder.build_unconditional_branch(merge_block)?;
        }
        let then_block_end = self.builder.get_insert_block().unwrap();
        // IfElse:
        self.builder.position_at_end(else_block);
        let else_value = self.compile_expr(ctx, else_exprs)?;
        if else_value.is_some() {
            self.builder.build_unconditional_branch(merge_block)?;
        }
        let else_block_end = self.builder.get_insert_block().unwrap();

        // IfEnd:
        self.builder.position_at_end(merge_block);
        Ok(match (then_value, else_value) {
            (None, None) => {
                self.builder.build_unreachable()?;
                None
            }
            (None, else_value) => else_value,
            (then_value, None) => then_value,
            (Some(then_val), Some(else_val)) => {
                let phi_node = self.builder.build_phi(self.ptr_type(), "ifResult").unwrap();
                phi_node.add_incoming(&[(&then_val, then_block_end), (&else_val, else_block_end)]);
                Some(phi_node.as_basic_value())
            }
        })
    }

    /// Compile a sync while
    fn compile_while(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        cond_expr: &mir::TypedExpr,
        body_expr: &mir::TypedExpr,
    ) -> Result<Option<inkwell::values::BasicValueEnum<'run>>> {
        let cond_block = self.context.append_basic_block(ctx.function, "WhileCond");
        let body_block = self.context.append_basic_block(ctx.function, "WhileBody");
        let end_block = self.context.append_basic_block(ctx.function, "WhileEnd");

        // WhileCond:
        self.builder.build_unconditional_branch(cond_block)?;
        self.builder.position_at_end(cond_block);
        let cond_value = self.compile_value_expr(ctx, cond_expr);
        self.gen_conditional_branch(cond_value, body_block, end_block);

        // WhileBody:
        self.builder.position_at_end(body_block);
        let _ = self.compile_expr(ctx, body_expr)?;
        self.builder.build_unconditional_branch(cond_block)?;

        // WhileEnd:
        self.builder.position_at_end(end_block);
        Ok(Some(self.compile_void()))
    }

    fn compile_alloc(
        &self,
        ctx: &mut CodeGenContext<'run>,
        name: &str,
        ty: &mir::Ty,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let v = self.builder.build_alloca(self.llvm_type(ty), name).unwrap();
        ctx.lvars.insert(name.to_string(), v);
        None
    }

    fn compile_lvar_set(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        name: &str,
        rhs: &mir::TypedExpr,
    ) -> Result<Option<inkwell::values::BasicValueEnum<'run>>> {
        let v = self.compile_value_expr(ctx, rhs);
        let ptr = ctx.lvars.get(name).unwrap();
        self.builder.build_store(ptr.clone(), v.clone())?;
        Ok(Some(v))
    }

    fn compile_ivar_set(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        obj_expr: &mir::TypedExpr,
        idx: usize,
        rhs: &mir::TypedExpr,
        name: &str,
    ) -> Result<Option<inkwell::values::BasicValueEnum<'run>>> {
        let obj = self.compile_value_expr(ctx, obj_expr);
        let value = self.compile_value_expr(ctx, rhs);
        instance::build_ivar_store_raw(
            self,
            SkObj::from_basic_value_enum(obj),
            &llvm_struct::of_ty(self, &obj_expr.1),
            idx,
            value,
            name,
        )?;
        Ok(None)
    }

    fn compile_const_set(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        name: &shiika_core::names::ConstFullname,
        rhs: &mir::TypedExpr,
    ) -> Result<Option<inkwell::values::BasicValueEnum<'run>>> {
        let v = self.compile_value_expr(ctx, rhs);
        let g = constants::get_global(self, name);
        self.builder.build_store(g.as_pointer_value(), v)?;
        Ok(Some(v))
    }

    fn compile_return(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        val_expr: Option<&mir::TypedExpr>,
    ) -> Result<Option<inkwell::values::BasicValueEnum<'run>>> {
        let return_val = val_expr.map(|expr| self.compile_value_expr(ctx, expr));
        self.builder.build_return(
            return_val
                .as_ref()
                .map(|v| v as &dyn inkwell::values::BasicValue),
        )?;
        Ok(None)
    }

    fn compile_exprs(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        exprs: &[mir::TypedExpr],
    ) -> Result<Option<inkwell::values::BasicValueEnum<'run>>> {
        let mut last_val = None;
        for e in exprs {
            last_val = self.compile_expr(ctx, e)?;
        }
        Ok(last_val)
    }

    fn compile_cast<'a>(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        cast_type: &mir::CastType,
        expr: &mir::TypedExpr,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let v1 = self.compile_value_expr(ctx, expr);
        let v2 = match cast_type {
            mir::CastType::Force(_) => v1,
            mir::CastType::Upcast(_) => v1,
            mir::CastType::ToAny => self.cast_to_any(v1, &expr.1),
            mir::CastType::Recover(ty) => self.cast_from_any(v1, ty),
        };
        Some(v2)
    }

    fn compile_create_object(
        &mut self,
        type_name: &str,
    ) -> Result<Option<inkwell::values::BasicValueEnum<'run>>> {
        let obj = instance::allocate_sk_obj(self, type_name)?;
        Ok(Some(obj.0.into()))
    }

    fn compile_create_type_object(
        &mut self,
        _ctx: &mut CodeGenContext<'run>,
        the_ty: &TermTy,
    ) -> Result<Option<inkwell::values::BasicValueEnum<'run>>> {
        let type_obj = type_object::create(self, the_ty)?;
        Ok(Some(type_obj.0.into()))
    }

    fn compile_unbox(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        expr: &mir::TypedExpr,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let e = self.compile_value_expr(ctx, expr);
        let sk_int = SkObj::from_basic_value_enum(e);
        Some(intrinsics::unbox_int(self, sk_int).into())
    }

    fn compile_raw_i64(&mut self, n: i64) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let llvm_n = self.context.i64_type().const_int(n as u64, false);
        Some(llvm_n.into())
    }

    fn compile_cell_new(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        value_expr: &mir::TypedExpr,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let value = self.compile_value_expr(ctx, value_expr);
        Some(cell::cell_new(self, value, &value_expr.1))
    }

    fn compile_cell_get(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        cell_expr: &mir::TypedExpr,
        result_ty: &mir::Ty,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let cell = self.compile_value_expr(ctx, cell_expr);
        Some(cell::cell_get(self, cell, result_ty))
    }

    fn compile_cell_set(
        &mut self,
        ctx: &mut CodeGenContext<'run>,
        cell_expr: &mir::TypedExpr,
        value_expr: &mir::TypedExpr,
    ) -> Option<inkwell::values::BasicValueEnum<'run>> {
        let cell = self.compile_value_expr(ctx, cell_expr);
        let value = self.compile_value_expr(ctx, value_expr);
        cell::cell_set(self, cell, value, &value_expr.1);
        None
    }

    /// Cast a value to Any (i64 representation).
    pub fn cast_to_any(
        &mut self,
        value: BasicValueEnum<'run>,
        from_ty: &mir::Ty,
    ) -> BasicValueEnum<'run> {
        match from_ty {
            mir::Ty::I1 => todo!("cast_to_any for I1"),
            mir::Ty::Int64 => value,
            _ => self
                .builder
                .build_ptr_to_int(
                    value.into_pointer_value(),
                    self.context.i64_type(),
                    "ptr_as_i64",
                )
                .unwrap()
                .into(),
        }
    }

    /// Cast a value from Any (i64) to the target type.
    pub fn cast_from_any(
        &mut self,
        value: BasicValueEnum<'run>,
        to_ty: &mir::Ty,
    ) -> BasicValueEnum<'run> {
        match to_ty {
            mir::Ty::I1 => todo!("cast_from_any for I1"),
            mir::Ty::Int64 => value,
            _ => self
                .builder
                .build_int_to_ptr(
                    value.into_int_value(),
                    self.ptr_type(),
                    "recover_i64_to_ptr",
                )
                .unwrap()
                .into(),
        }
    }

    /// Generate conditional branch by Shiika Bool
    fn gen_conditional_branch(
        &mut self,
        cond: inkwell::values::BasicValueEnum<'run>,
        then_block: inkwell::basic_block::BasicBlock,
        else_block: inkwell::basic_block::BasicBlock,
    ) {
        let i = intrinsics::unbox_bool(self, SkObj::from_basic_value_enum(cond));
        let one = self.context.bool_type().const_int(1, false);
        let istrue = self
            .builder
            .build_int_compare(inkwell::IntPredicate::EQ, i, one, "istrue")
            .unwrap();
        let _ = self
            .builder
            .build_conditional_branch(istrue, then_block, else_block);
    }

    fn llvm_function_type(&self, fun_ty: &mir::FunTy) -> inkwell::types::FunctionType<'ictx> {
        let param_tys = self.llvm_types(&fun_ty.param_tys);
        if *fun_ty.ret_ty == mir::Ty::CVoid {
            self.context.void_type().fn_type(&param_tys, false)
        } else {
            let ret_ty = self.llvm_type(&fun_ty.ret_ty);
            ret_ty.fn_type(&param_tys, false)
        }
    }

    fn llvm_types(&self, tys: &[mir::Ty]) -> Vec<inkwell::types::BasicMetadataTypeEnum<'ictx>> {
        tys.iter().map(|x| self.llvm_type(x).into()).collect()
    }
}

// Question: is there a better way to do this?
fn inkwell_set_name(val: BasicValueEnum, name: &str) {
    match val {
        BasicValueEnum::ArrayValue(v) => v.set_name(name),
        BasicValueEnum::IntValue(v) => v.set_name(name),
        BasicValueEnum::FloatValue(v) => v.set_name(name),
        BasicValueEnum::PointerValue(v) => v.set_name(name),
        BasicValueEnum::StructValue(v) => v.set_name(name),
        BasicValueEnum::VectorValue(v) => v.set_name(name),
        BasicValueEnum::ScalableVectorValue(v) => v.set_name(name),
    }
}
