#[derive(Clone)]
pub enum SkValue<'run> {
    SkObj(SkObj<'run>),
    Opaque(inkwell::values::BasicValueEnum<'run>),
}
#[derive(Clone)]
pub struct SkObj<'run>(pub inkwell::values::PointerValue<'run>);

impl<'run> From<SkObj<'run>> for SkValue<'run> {
    fn from(obj: SkObj<'run>) -> Self {
        SkValue::SkObj(obj)
    }
}

impl<'run> From<SkValue<'run>> for inkwell::values::BasicMetadataValueEnum<'run> {
    fn from(value: SkValue<'run>) -> Self {
        match value {
            SkValue::SkObj(obj) => obj.0.into(),
            SkValue::Opaque(value) => value.into(),
        }
    }
}

impl<'run> SkValue<'run> {
    pub fn into_basic_value_enum(self) -> inkwell::values::BasicValueEnum<'run> {
        match self {
            SkValue::SkObj(obj) => obj.0.into(),
            SkValue::Opaque(value) => value,
        }
    }

    pub fn into_function_pointer(self) -> inkwell::values::PointerValue<'run> {
        match self {
            SkValue::SkObj(_) => panic!("SkObj cannot be converted to function pointer"),
            SkValue::Opaque(value) => value.into_pointer_value(),
        }
    }

    pub fn into_int_value(self) -> inkwell::values::IntValue<'run> {
        match self {
            SkValue::SkObj(_) => panic!("SkObj cannot be converted to IntValue"),
            SkValue::Opaque(value) => value.into_int_value(),
        }
    }
}
