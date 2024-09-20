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
