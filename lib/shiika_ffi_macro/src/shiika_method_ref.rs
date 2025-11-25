use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use shiika_ffi_mangle::mangle_method;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parenthesized, parse_macro_input, Field, Result, Token};

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
pub fn compile(input: TokenStream) -> TokenStream {
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

/// Helper struct for `shiika_method_ref` macro
pub struct ShiikaMethodRef {
    pub method_name: syn::LitStr,
    pub parameters: Punctuated<Field, Token![,]>,
    pub ret_ty: syn::Type,
    pub rust_func_name: syn::LitStr,
}

impl Parse for ShiikaMethodRef {
    fn parse(input: ParseStream) -> Result<Self> {
        let method_name = input.parse()?;
        let _: Token![,] = input.parse()?;
        let _: Token![fn] = input.parse()?;
        let content;
        let _: syn::token::Paren = parenthesized!(content in input);
        let parameters = content.parse_terminated(Field::parse_named)?;
        let _: Token![->] = input.parse()?;
        let ret_ty = input.parse()?;
        let _: Token![,] = input.parse()?;
        let rust_func_name = input.parse()?;
        Ok(ShiikaMethodRef {
            method_name,
            parameters,
            ret_ty,
            rust_func_name,
        })
    }
}

impl ShiikaMethodRef {
    /// Returns mangled llvm func name (eg. `Meta_Class_new`)
    pub fn mangled_name(&self) -> Ident {
        Ident::new(&mangle_method(&self.method_name.value()), Span::call_site())
    }

    /// Returns user-specified func name. (eg. `meta_class_new`)
    pub fn wrapper_name(&self) -> Ident {
        Ident::new(&self.rust_func_name.value(), Span::call_site())
    }

    /// Returns list of parameters for forwarding function call (eg. `a, b, c`)
    pub fn forwaring_args(&self) -> Punctuated<Ident, Token![,]> {
        self.parameters
            .iter()
            .map(|field| field.ident.clone().expect("Field::ident is None. why?"))
            .collect()
    }
}
