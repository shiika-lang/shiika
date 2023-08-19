use crate::utils::{lambda_capture_struct_name, LlvmFuncName};
use crate::values::{I8Ptr, SkObj};
use crate::CodeGen;
use anyhow::Result;
use either::Either::*;
use inkwell::types::AnyType;
use inkwell::types::BasicType;
use inkwell::values::BasicValue;
use shiika_core::ty::*;
use skc_hir::visitor::HirVisitor;
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
        Self::get_struct_type(gen, name).ptr_type(Default::default())
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
    pub fn store(&self, gen: &CodeGen, idx: usize, value: SkObj<'run>) {
        self.store_raw(gen, idx, value.0.as_basic_value_enum());
    }

    /// Store `value` at the given index
    fn store_raw(&self, gen: &CodeGen, idx: usize, value: inkwell::values::BasicValueEnum<'run>) {
        debug_assert!(self.store_type_matches(gen, idx, value));

        gen.build_llvm_struct_set(
            &self.struct_type(gen),
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

    /// Get the (possibly indirectly) stored value.
    pub fn get_value(
        &self,
        gen: &CodeGen<'_, 'run, '_>,
        idx: usize,
        ty: &TermTy,
        deref: bool,
    ) -> SkObj<'run> {
        let v = if deref {
            let addr = gen
                .build_llvm_struct_ref_raw(
                    &self.struct_type(gen),
                    self.to_struct_ptr(),
                    gen.ptr_type.clone().as_basic_type_enum(),
                    idx,
                    "load",
                )
                .into_pointer_value();
            let pointee_ty = gen.llvm_type().as_basic_type_enum();
            gen.builder.build_load(pointee_ty, addr, "deref")
        } else {
            gen.build_llvm_struct_ref(&self.struct_type(gen), self.to_struct_ptr(), idx, "load")
        };
        SkObj::new(ty.clone(), v)
    }

    /// Given there is a pointer stored at `idx`, update its value.
    pub fn reassign(&self, gen: &CodeGen<'_, 'run, '_>, idx: usize, value: SkObj) {
        let ptr = gen
            .build_llvm_struct_ref(&self.struct_type(gen), self.to_struct_ptr(), idx, "load")
            .into_pointer_value();
        gen.builder.build_store(ptr, value.0);
    }
}

impl<'hir: 'ictx, 'run, 'ictx: 'run> CodeGen<'hir, 'run, 'ictx> {
    /// Find all lambdas in a hir and create the body of the corresponding llvm function
    /// PERF: Ideally they should be created during gen_methods but I couldn't
    /// avoid borrow checker errors.
    pub(super) fn gen_lambda_funcs(&self, hir: &'hir Hir) -> Result<()> {
        let mut v = GenLambdaFuncVisitor(&self);
        visitor::walk_hir(&mut v, hir)
    }
}
struct GenLambdaFuncVisitor<'hir: 'ictx, 'run, 'ictx: 'run>(&'run CodeGen<'hir, 'run, 'ictx>);
impl<'hir: 'ictx, 'run, 'ictx: 'run> HirVisitor<'hir> for GenLambdaFuncVisitor<'hir, 'run, 'ictx> {
    fn visit_expr(&mut self, expr: &'hir HirExpression) -> Result<()> {
        let gen = &self.0;
        match &expr.node {
            HirLambdaExpr {
                name,
                params,
                exprs,
                ret_ty,
                lvars,
                ..
            } => {
                let func_name = LlvmFuncName(name.to_string());
                gen.gen_llvm_func_body(
                    &func_name,
                    params,
                    Default::default(),
                    Right(exprs),
                    lvars,
                    ret_ty,
                    Some(name.to_string()),
                )?;
            }
            _ => (),
        }
        Ok(())
    }
}

impl<'hir: 'ictx, 'run, 'ictx: 'run> CodeGen<'hir, 'run, 'ictx> {
    /// Create LLVM structs for lambda captures.
    pub(super) fn gen_lambda_capture_structs(&self, hir: &'hir Hir) -> Result<()> {
        let mut v = LambdaCaptureStructsVisitor(&self);
        visitor::walk_hir(&mut v, hir)
    }
}
struct LambdaCaptureStructsVisitor<'hir: 'ictx, 'run, 'ictx: 'run>(
    &'run CodeGen<'hir, 'run, 'ictx>,
);
impl<'hir> HirVisitor<'hir> for LambdaCaptureStructsVisitor<'_, '_, '_> {
    fn visit_expr(&mut self, expr: &HirExpression) -> Result<()> {
        match &expr.node {
            HirLambdaExpr { name, captures, .. } => {
                let gen = &self.0;
                let struct_name = lambda_capture_struct_name(name);
                let struct_type = gen.context.opaque_struct_type(&struct_name);
                // The type of a capture may a Shiika object or a pointer to
                // a Shiika object; in both cases its llvm type is `ptr`.
                let capture_ty = gen.ptr_type.as_basic_type_enum();
                let body = captures.iter().map(|_| capture_ty).collect::<Vec<_>>();
                struct_type.set_body(&body, false);
            }
            _ => (),
        }
        Ok(())
    }
}
