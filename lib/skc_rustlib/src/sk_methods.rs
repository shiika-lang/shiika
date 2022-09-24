//! This module provides Rust bindings for llvm functions for Shiika methods.
//!
use crate::builtin::{SkAry, SkClass, SkObj};
use shiika_ffi_macro::shiika_method_ref;

// This macro call expands into:
//
//    extern "C" {
//        #[allow(improper_ctypes)]
//        fn Meta_Array_new(receiver: *const u8) -> SkAry<SkObj>;
//    }
//    pub fn meta_array_new(receiver: *const u8) -> SkAry<SkObj> {
//        unsafe { Meta_Array_new(receiver) }
//    }
shiika_method_ref!(
    "Meta:Array#new",
    fn(receiver: *const u8) -> SkAry<SkObj>,
    "meta_array_new"
);

shiika_method_ref!(
    "Meta:Class#new",
    fn(receiver: *const u8) -> SkClass,
    "meta_class_new"
);
