//! This module provides Rust bindings for llvm functions for Shiika methods.
//!
use crate::builtin::{SkAry, SkObj};

// Is it possible to generate this from `"Meta:Array.new"` by proc macro?
extern "C" {
    #[allow(improper_ctypes)]
    pub fn Meta_Array_new(receiver: *const u8) -> SkAry<SkObj>;
}
pub fn meta_array_new(receiver: *const u8) -> SkAry<SkObj> {
    unsafe { Meta_Array_new(receiver) }
}
