#![feature(range_contains)]
#![feature(nll)]
mod shiika;

fn main() -> Result<(), Box<std::error::Error>> {
    let str = "putchar 72";
    let ast = shiika::parser::Parser::parse(str)?;
    let stdlib = shiika::stdlib::create_classes();
    let hir = shiika::hir::Hir::from_ast(ast, &stdlib)?;
    let code_gen = shiika::code_gen::CodeGen::new();
    code_gen.gen_program(hir, stdlib)?;
    code_gen.module.print_to_file("a.ll")?;
    Ok(())
    //println!("{:?}", shiika::parser::parse(str));
}

//use inkwell::context::Context;
//use inkwell::values::BasicValue;
//use std::error::Error;
//
//fn main() -> Result<(), Box<Error>> {
//    let context = Context::create();
//    let module = context.create_module("main");
//    let builder = context.create_builder();
//    let i32_type = context.i32_type();
//
//    // declare i32 @putchar(i32)
//    let putchar_type = i32_type.fn_type(&[i32_type.into()], false);
//    module.add_function("putchar", putchar_type, None);
//
//    // define i32 @main() {
//    let main_type = i32_type.fn_type(&[], false);
//    let function = module.add_function("main", main_type, None);
//    let basic_block = context.append_basic_block(&function, "entry");
//    builder.position_at_end(&basic_block);
//
//    // call i32 @putchar(i32 72)
//    let fun = module.get_function("putchar");
//    let n = -72;
//    let i = i32_type.const_int(n as u64, false);
//    builder.build_call(fun.unwrap(), &[i.as_basic_value_enum()], "putchar");
//    builder.build_call(fun.unwrap(), &[i32_type.const_int(105, false).into()], "putchar");
//
//    // ret i32 0
//    builder.build_return(Some(&i32_type.const_int(0, false)));
//
//    module.print_to_file("a.ll");
//
//    Ok(())
//}
//
