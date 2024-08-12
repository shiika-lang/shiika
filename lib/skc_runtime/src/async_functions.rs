use crate::chiika_env::ChiikaEnv;
use crate::{ChiikaCont, ContFuture};
use std::future::{poll_fn, Future};
use std::task::Poll;
use std::time::Duration;

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn sleep_sec(env: &'static mut ChiikaEnv, n: i64, cont: ChiikaCont) -> ContFuture {
    async fn sleep_sec(n: i64) -> u64 {
        // Hand written part (all the rest will be macro-generated)
        tokio::time::sleep(Duration::from_secs(n as u64)).await;
        0
    }
    env.cont = Some(cont);
    let mut future = Box::pin(sleep_sec(n));
    Box::new(poll_fn(move |ctx| match future.as_mut().poll(ctx) {
        Poll::Ready(v) => Poll::Ready(v),
        Poll::Pending => Poll::Pending,
    }))
}
