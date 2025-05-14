use crate::codegen::constants;
use crate::codegen::CodeGen;
use shiika_core::names::ClassFullname;

#[derive(Clone)]
pub struct SkObj<'run>(pub inkwell::values::PointerValue<'run>);

impl<'run> From<SkObj<'run>> for inkwell::values::BasicValueEnum<'run> {
    fn from(obj: SkObj<'run>) -> Self {
        obj.0.into()
    }
}

impl<'run> SkObj<'run> {
    pub fn from_basic_value_enum(value: inkwell::values::BasicValueEnum<'run>) -> Self {
        SkObj(value.into_pointer_value())
    }
}

/// Shiika class object (eg. `Meta:Int*`, `Meta:String*`)
#[derive(Debug)]
pub struct SkClassObj<'run>(pub inkwell::values::PointerValue<'run>);

impl<'run> SkClassObj<'run> {
    pub fn load(gen: &mut CodeGen<'run, '_>, name: &ClassFullname) -> Self {
        SkClassObj(constants::load(gen, &name.to_const_fullname()).0)
    }
}
