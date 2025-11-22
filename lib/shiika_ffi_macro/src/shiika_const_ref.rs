use proc_macro2::{Ident, Span};
use shiika_ffi_mangle::mangle_const;
use syn::parse::{Parse, ParseStream};
use syn::{Result, Token};

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
