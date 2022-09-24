//! This module provides Rust bindings for llvm functions for Shiika methods.
//!
use crate::builtin::{SkAry, SkClass, SkObj};

// Is it possible to generate this from `"Meta:Array.new"` by proc macro?
extern "C" {
    #[allow(improper_ctypes)]
    pub fn Meta_Array_new(receiver: *const u8) -> SkAry<SkObj>;
}
pub fn meta_array_new(receiver: *const u8) -> SkAry<SkObj> {
    unsafe { Meta_Array_new(receiver) }
}

extern "C" {
    #[allow(improper_ctypes)]
    fn Meta_Class_new(receiver: *const u8) -> SkClass;
}
pub fn meta_class_new(receiver: *const u8) -> SkClass {
    unsafe { Meta_Class_new(receiver) }
}
