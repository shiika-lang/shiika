use shiika_ffi::async_::{ChiikaCont, ChiikaEnv, ContFuture};
use shiika_ffi::core_class::{SkInt, SkObject};
use shiika_ffi_macro::shiika_method;
use std::future::{poll_fn, Future};
use std::task::Poll;
use std::time::Duration;

#[shiika_method("Object#initialize")]
pub extern "C" fn object_initialize(_receiver: SkObject) {}

#[shiika_method("Object#print")]
pub extern "C" fn print(_receiver: SkInt, n: SkInt) {
    println!("{}", n.val());
}

#[shiika_method("Object#sleep_sec")]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn sleep_sec(
    env: &'static mut ChiikaEnv,
    _receiver: SkInt,
    nn: SkInt,
    cont: ChiikaCont,
) -> ContFuture {
    async fn sleep_sec(n: SkInt) {
        // Hand written part (all the rest will be macro-generated)
        let sec = n.val() as u64;
        tokio::time::sleep(Duration::from_secs(sec)).await;
    }
    env.cont = Some(cont);
    let mut future = Box::pin(sleep_sec(nn));
    Box::new(poll_fn(move |ctx| match future.as_mut().poll(ctx) {
        Poll::Ready(_) => Poll::Ready(0),
        Poll::Pending => Poll::Pending,
    }))
}
