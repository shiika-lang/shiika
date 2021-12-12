pub fn mangle_method(method_name: &str) -> String {
    method_name
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
        .replace("<", "lt_")
        .replace(">", "gt_")
        .replace("<=", "le_")
        .replace(">=", "ge_")
        .replace("[]", "aref_")
        .replace("[]=", "aset_")
}
