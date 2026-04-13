pub mod async_;
pub mod core_class;

/// Trait for Shiika values that can be returned from async methods.
/// Converts to a raw u64 representation for the async runtime.
pub trait SkValue {
    fn as_raw_u64(self) -> u64;
}
