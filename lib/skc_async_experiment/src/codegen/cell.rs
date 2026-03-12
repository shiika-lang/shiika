//! Helper functions for Cell operations (used for lambda captures)
use crate::codegen::CodeGen;
use crate::mir;
use inkwell::values::BasicValueEnum;

/// Allocate a new cell with the given value.
/// Automatically casts the value to Any (i64).
pub fn cell_new<'run, 'ictx: 'run>(
    gen: &mut CodeGen<'run, 'ictx>,
    value: BasicValueEnum<'run>,
    value_ty: &mir::Ty,
) -> BasicValueEnum<'run> {
    let value_as_any = gen.cast_to_any(value, value_ty);
    gen.call_llvm_func("shiika_cell_new", &[value_as_any.into()], "cell")
        .expect("shiika_cell_new should return a value")
}

/// Get the value from a cell.
/// Automatically casts the result from Any to the target type.
pub fn cell_get<'run, 'ictx: 'run>(
    gen: &mut CodeGen<'run, 'ictx>,
    cell: BasicValueEnum<'run>,
    result_ty: &mir::Ty,
) -> BasicValueEnum<'run> {
    let any_value = gen
        .call_llvm_func("shiika_cell_get", &[cell.into()], "cell_value")
        .expect("shiika_cell_get should return a value");
    gen.cast_from_any(any_value, result_ty)
}

/// Set the value in a cell.
/// Automatically casts the value to Any (i64).
pub fn cell_set<'run, 'ictx: 'run>(
    gen: &mut CodeGen<'run, 'ictx>,
    cell: BasicValueEnum<'run>,
    value: BasicValueEnum<'run>,
    value_ty: &mir::Ty,
) {
    let value_as_any = gen.cast_to_any(value, value_ty);
    gen.call_llvm_func("shiika_cell_set", &[cell.into(), value_as_any.into()], "");
}
