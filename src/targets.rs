/// Returns default `TargetTriple`
pub fn default_triple() -> inkwell::targets::TargetTriple {
    if let Some(info) = mac_sys_info::get_mac_sys_info().ok() {
        // #281: get_default_triple returns `darwin` but clang shows warning for it
        let arch = info.cpu_info().architecture();
        let ver = info.os_info().os_version();
        let s = format!("{}-apple-macosx{}", arch, ver);
        inkwell::targets::TargetTriple::create(&s)
    } else {
        inkwell::targets::TargetMachine::get_default_triple()
    }
}
