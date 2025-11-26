mod async_shiika_method;
mod shiika_const_ref;
mod shiika_method;
mod shiika_method_ref;
use proc_macro::TokenStream;

/// See `shiika_method::compile`.
#[proc_macro_attribute]
pub fn shiika_method(args: TokenStream, input: TokenStream) -> TokenStream {
    shiika_method::compile(args, input)
}

/// See `async_shiika_method::compile`.
#[proc_macro_attribute]
pub fn async_shiika_method(args: TokenStream, input: TokenStream) -> TokenStream {
    async_shiika_method::compile(args, input)
}

/// See `shiika_method_ref::compile`.
#[proc_macro]
pub fn shiika_method_ref(input: TokenStream) -> TokenStream {
    shiika_method_ref::compile(input)
}

/// See `shiika_const_ref::compile`.
#[proc_macro]
pub fn shiika_const_ref(input: TokenStream) -> TokenStream {
    shiika_const_ref::compile(input)
}
