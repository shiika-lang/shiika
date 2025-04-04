pub mod chiika_env;
pub use chiika_env::{ChiikaEnv, ChiikaValue};
use std::future::Future;

#[allow(improper_ctypes_definitions)]
pub type ContFuture = Box<dyn Future<Output = ChiikaValue> + Unpin + Send>;

#[allow(improper_ctypes_definitions)]
pub type ChiikaCont = extern "C" fn(env: *mut ChiikaEnv, value: ChiikaValue) -> ContFuture;
