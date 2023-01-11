/// Returns default `TargetTriple`
#[cfg(feature = "mac")]
pub fn default_triple() -> inkwell::targets::TargetTriple {
    if let Ok(info) = mac_sys_info::get_mac_sys_info() {
        // #281: get_default_triple returns `darwin` but clang shows warning for it
        let arch = info.cpu_info().architecture();
        let ver = info.os_info().os_version();
        // #281: Add .0
        let n_dots = ver.chars().filter(|c| *c == '.').count();
        let zero = if n_dots >= 2 { "" } else { ".0" };
        let s = format!("{}-apple-macosx{}{}", arch, ver, zero);
        inkwell::targets::TargetTriple::create(&s)
    } else {
        inkwell::targets::TargetMachine::get_default_triple()
    }
}

#[cfg(not(feature = "mac"))]
pub fn default_triple() -> inkwell::targets::TargetTriple {
    inkwell::targets::TargetMachine::get_default_triple()
}
