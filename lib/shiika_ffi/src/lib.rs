pub fn mangle_method(method_name: &str) -> String {
    let s = method_name
        // Replace '_' to use '_' as delimiter
        .replace("_", "__")
        // Replace symbols to make the function callable from Rust(skc_rustlib)
        .replace("::", "_")
        .replace("Meta:", "Meta_")
        .replace("#", "_")
        .replace("+@", "uplus_")
        .replace("-@", "uminus_")
        .replace("+", "add_")
        .replace("-", "sub_")
        .replace("*", "mul_")
        .replace("/", "div_")
        .replace("%", "mod_")
        .replace("==", "eq_")
        .replace("<=", "le_")
        .replace(">=", "ge_")
        .replace("<", "lt_")
        .replace(">", "gt_")
        .replace("[]=", "aset_")
        .replace("[]", "aref_");
    if s.ends_with('=') {
        format!("{}{}", "_set_", &s.replace("=", ""))
    } else {
        s
    }
}
