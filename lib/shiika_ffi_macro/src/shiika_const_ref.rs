use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use shiika_ffi_mangle::mangle_const;
use syn::parse::{Parse, ParseStream};
use syn::parse_macro_input;
use syn::{Result, Token};

/// Define a reference to a Shiika constant.
///
/// ## Example
/// ```rust
/// shiika_const_ref!(
///     "::Time::Zone", // Shiika const name
///     SkClass,        // Type of the constant
///     "sk_Time_Zone", // Wrapper name
/// );
/// ...
/// dbg!(&sk_Time_Zone());
/// ```
pub fn compile(input: TokenStream) -> TokenStream {
    let spec = parse_macro_input!(input as ShiikaConstRef);
    let mangled_name = spec.mangled_name();
    let const_type = &spec.const_ty;
    let wrapper_name = spec.wrapper_name();
    // Example:
    //   extern "C" {
    //       #[allow(improper_ctypes)]
    //       static shiika_const_xx: SkClass;
    //   }
    //   pub fn sk_Time_Zone() -> SkClass {
    //       unsafe { SkClass( shiika_const_xx.dup() ) }
    //   }
    let gen = quote! {
        extern "C" {
            #[allow(improper_ctypes)]
            static #mangled_name: #const_type;
        }
        pub fn #wrapper_name() -> #const_type {
            unsafe { #mangled_name.dup() }
        }
    };
    gen.into()
}

/// Helper struct for `shiika_const_ref` macro
pub struct ShiikaConstRef {
    pub const_name: syn::LitStr,
    pub const_ty: syn::Type,
    pub rust_func_name: syn::LitStr,
}

impl Parse for ShiikaConstRef {
    fn parse(input: ParseStream) -> Result<Self> {
        let const_name = input.parse()?;
        let _: Token![,] = input.parse()?;
        let const_ty = input.parse()?;
        let _: Token![,] = input.parse()?;
        let rust_func_name = input.parse()?;
        Ok(ShiikaConstRef {
            const_name,
            const_ty,
            rust_func_name,
        })
    }
}

impl ShiikaConstRef {
    /// Returns mangled llvm constant name.
    pub fn mangled_name(&self) -> Ident {
        Ident::new(&mangle_const(&self.const_name.value()), Span::call_site())
    }

    /// Returns user-specified wrapper function name.
    pub fn wrapper_name(&self) -> Ident {
        Ident::new(&self.rust_func_name.value(), Span::call_site())
    }
}
