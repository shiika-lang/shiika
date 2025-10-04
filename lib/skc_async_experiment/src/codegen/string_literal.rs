use crate::codegen::CodeGen;

/// Defines a global i8 array for the string literal and returns a pointer to it.
pub fn declare<'run>(gen: &mut CodeGen<'run, '_>, s: &str) -> inkwell::values::PointerValue<'run> {
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
