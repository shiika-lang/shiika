use shiika_ffi::async_::{ChiikaCont, ChiikaEnv, ContFuture};
use shiika_ffi::core_class::{SkInt, SkObject, SkString};
use shiika_ffi_macro::shiika_method;
use std::future::{poll_fn, Future};
use std::task::Poll;
use std::time::Duration;
use tokio::io::{stdout, AsyncWriteExt};

#[shiika_method("Object#print")]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn print(
    env: &'static mut ChiikaEnv,
    _receiver: SkObject,
    nn: SkInt,
    cont: ChiikaCont,
) -> ContFuture {
    async fn print(n: SkInt) {
        // Hand written part (all the rest will be macro-generated)
        let mut stdout = stdout();
        let output = format!("{}\n", n.val());
        stdout.write_all(output.as_bytes()).await.unwrap();
        stdout.flush().await.unwrap();
    }
    env.cont = Some(cont);
    let mut future = Box::pin(print(nn));
    Box::new(poll_fn(move |ctx| match future.as_mut().poll(ctx) {
        Poll::Ready(_) => Poll::Ready(0),
        Poll::Pending => Poll::Pending,
    }))
}

#[shiika_method("Object#puts")]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn puts(
    env: &'static mut ChiikaEnv,
    _receiver: SkObject,
    ss: SkString,
    cont: ChiikaCont,
) -> ContFuture {
    async fn puts(s: SkString) {
        // Hand written part (all the rest will be macro-generated)
        let mut stdout = stdout();
        stdout.write_all(s.value()).await.unwrap();
        stdout.write_all(b"\n").await.unwrap();
        stdout.flush().await.unwrap();
    }
    env.cont = Some(cont);
    let mut future = Box::pin(puts(ss));
    Box::new(poll_fn(move |ctx| match future.as_mut().poll(ctx) {
        Poll::Ready(_) => Poll::Ready(0),
        Poll::Pending => Poll::Pending,
    }))
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
