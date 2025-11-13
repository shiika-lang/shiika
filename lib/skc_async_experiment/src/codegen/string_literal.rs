use crate::codegen::CodeGen;
use crate::names::FunctionName;

/// Generates a Shiika String object from a string literal.
/// This creates the string data and calls String.new to create the proper Shiika String object.
pub fn generate<'run>(
    gen: &mut CodeGen<'run, '_>,
    s: &str,
) -> inkwell::values::BasicValueEnum<'run> {
    let args = {
        let string_data = declare_global(gen, s);
        let byte_size = gen.context.i64_type().const_int(s.len() as u64, false);
        [
            gen.compile_constref("::String").unwrap().into(),
            string_data.into(),
            byte_size.into(),
        ]
    };
    gen.builder
        .build_direct_call(
            gen.get_llvm_func(&FunctionName::method("Meta:String", "new")),
            &args,
            "string_new_result",
        )
        .try_as_basic_value()
        .left()
        .unwrap()
}

/// Defines a global i8 array for the string literal and returns a pointer to it.
fn declare_global<'run>(
    gen: &mut CodeGen<'run, '_>,
    s: &str,
) -> inkwell::values::PointerValue<'run> {
    let name = format!("shiika_str{}", gen.string_id);
    gen.string_id += 1;

    let str_type = gen.context.i8_type().array_type(s.len() as u32);
    let global = gen.module.add_global(str_type, None, &name);
    global.set_linkage(inkwell::module::Linkage::Internal);
    let content = s
        .bytes()
        .map(|byte| gen.context.i8_type().const_int(byte.into(), false))
        .collect::<Vec<_>>();
    global.set_initializer(&gen.context.i8_type().const_array(&content));

    gen.module.get_global(&name).unwrap().as_pointer_value()
}
