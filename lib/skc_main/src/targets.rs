use os_info::{Type, Version};

/// Returns default `TargetTriple`
pub fn default_triple() -> inkwell::targets::TargetTriple {
    let info = os_info::get();
    if info.os_type() == Type::Macos {
        // #281: calculate target triple to avoid clang's warning
        if let Some(arch) = info.architecture() {
            if let Version::Semantic(major, minor, patch) = info.version() {
                let s = format!("{}-apple-macosx{}.{}.{}", arch, major, minor, patch);
                return inkwell::targets::TargetTriple::create(&s);
            }
        }
    }
    inkwell::targets::TargetMachine::get_default_triple()
}
