use proc_macro::TokenStream;
use quote::quote;
use shiika_ffi::mangle_method;
use syn::parse_macro_input;

// Export this function as the name callable as Shiika method.
//
// ## Example
//
// ```rust
// #[shiika_method("Class#_initialize_rustlib")]
// #[allow(non_snake_case)]
// pub extern "C" fn class__initialize_rustlib(
// ```
#[proc_macro_attribute]
pub fn shiika_method(args: TokenStream, input: TokenStream) -> TokenStream {
    let method_name = parse_macro_input!(args as syn::LitStr);
    let function_definition = parse_macro_input!(input as syn::ItemFn);

    let mangled_name = mangle_method(&method_name.value());
    let gen = quote! {
        #[export_name = #mangled_name]
        #function_definition
    };
    gen.into()
}
