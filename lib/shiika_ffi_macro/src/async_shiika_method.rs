use proc_macro::TokenStream;
use quote::quote;
use shiika_ffi_mangle::mangle_method;
use syn::parse_macro_input;

pub fn compile(args: TokenStream, input: TokenStream) -> TokenStream {
    let method_name = parse_macro_input!(args as syn::LitStr);
    let orig_function_definition = parse_macro_input!(input as syn::ItemFn);
    let orig_function_name = &orig_function_definition.sig.ident;
    let orig_function_params = &orig_function_definition.sig.inputs;

    // eg. for `fn foo(a: A, b: B)`, create `a, b`
    let forwarding_args = orig_function_params
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Typed(pat_type) => {
                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                    Some(&pat_ident.ident)
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    let when_ready = {
        let returns_unit = match &orig_function_definition.sig.output {
            syn::ReturnType::Default => true,
            syn::ReturnType::Type(_, ty) => {
                // Check if the type is () tuple
                matches!(**ty, syn::Type::Tuple(ref tuple) if tuple.elems.is_empty())
            }
        };
        if returns_unit {
            quote! { Poll::Ready(_) => Poll::Ready(0) }
        } else {
            // For Shiika values, get the inner ptr and cast to u64
            quote! { Poll::Ready(x) => Poll::Ready(x.0 as u64) }
        }
    };

    let mangled_name = mangle_method(&method_name.value());
    let gen = quote! {
        #[export_name = #mangled_name]
        //#[allow(improper_ctypes_definitions)]
        pub extern "C" fn #orig_function_name(
            env: &'static mut shiika_ffi::async_::ChiikaEnv,
            #orig_function_params,
            cont: shiika_ffi::async_::ChiikaCont,
        ) -> shiika_ffi::async_::ContFuture {
            use std::task::Poll;
            use std::future::{Future, poll_fn};

            #orig_function_definition

            env.cont = Some(cont);
            let mut future = Box::pin(
                #orig_function_name( #(#forwarding_args),* )
            );
            Box::new(poll_fn(move |ctx| match future.as_mut().poll(ctx) {
                #when_ready,
                Poll::Pending => Poll::Pending,
            }))
        }
    };
    gen.into()
}
