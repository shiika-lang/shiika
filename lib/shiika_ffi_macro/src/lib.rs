mod shiika_method_ref;
use proc_macro::TokenStream;
use quote::quote;
use shiika_ffi::mangle_method;
use shiika_method_ref::ShiikaMethodRef;
use syn::parse_macro_input;

/// Export this function as the name callable as Shiika method.
///
/// ## Example
///
/// ```rust
/// #[shiika_method("Class#_initialize_rustlib")]
/// #[allow(non_snake_case)]
/// pub extern "C" fn class__initialize_rustlib(
/// ```
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

/// Define a wrapper function to call Shiika method.
///
/// ## Example
/// ```rust
/// shiika_method_ref!(
///     "Meta:Class#new", // Shiika method name
///     fn(receiver: *const u8) -> SkAry<SkObj>, // Type of the function
///     "meta_class_new" // Name of the function
/// );
/// ```
#[proc_macro]
pub fn shiika_method_ref(input: TokenStream) -> TokenStream {
    let spec = parse_macro_input!(input as ShiikaMethodRef);
    let mangled_name = spec.mangled_name();
    let parameters = &spec.parameters;
    let return_type = &spec.ret_ty;
    let wrapper_name = spec.wrapper_name();
    let args = spec.forwaring_args();
    let gen = quote! {
        extern "C" {
            #[allow(improper_ctypes)]
            fn #mangled_name(#parameters) -> #return_type;
        }
        pub fn #wrapper_name(#parameters) -> #return_type {
            unsafe { #mangled_name(#args) }
        }
    };
    gen.into()
}
