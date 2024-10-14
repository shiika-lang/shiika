use crate::chiika_env::ChiikaEnv;
use crate::{ChiikaCont, ContFuture};
use shiika_ffi::core_class::SkInt;
use std::future::{poll_fn, Future};
use std::task::Poll;
use std::time::Duration;

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn sleep_sec(
    env: &'static mut ChiikaEnv,
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
