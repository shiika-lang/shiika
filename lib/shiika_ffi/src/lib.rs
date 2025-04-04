pub mod async_;
pub mod core_class;

/// Returns the C-level name of a Shiika method
/// (eg: `Int#+`, `Meta:Class#new`)
pub fn mangle_method(method_name: &str) -> String {
    let s = method_name
        // Replace '_' to use '_' as delimiter
        .replace('_', "__")
        // Replace symbols to make the function callable from Rust(skc_rustlib)
        .replace("::", "_")
        .replace("Meta:", "Meta_")
        .replace('#', "_")
        .replace("+@", "uplus_")
        .replace("-@", "uminus_")
        .replace('+', "add_")
        .replace('-', "sub_")
        .replace('*', "mul_")
        .replace('/', "div_")
        .replace('%', "mod_")
        .replace("==", "eq_")
        .replace("<=", "le_")
        .replace(">=", "ge_")
        .replace('<', "lt_")
        .replace('>', "gt_")
        .replace("[]=", "aset_")
        .replace("[]", "aref_");
    if s.ends_with('=') {
        format!("{}{}", "_set_", &s.replace('=', ""))
    } else {
        s
    }
}

/// Returns the C-level name of a Shiika constant.
pub fn mangle_const(const_name: &str) -> String {
    let s = const_name
        // Replace '_' to use '_' as delimiter
        .replace('_', "__")
        // Trim the first "::"
        .trim_start_matches("::")
        // Replace symbols to make the global variable accesible from Rust
        .replace("::", "_");
    format!("shiika_const_{}", &s)
}
