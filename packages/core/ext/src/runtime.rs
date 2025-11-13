mod allocator;
mod wtable;

use shiika_ffi::async_::{ChiikaCont, ChiikaEnv, ChiikaValue, ContFuture};
use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::task::Poll;

#[allow(improper_ctypes_definitions)]
type ChiikaThunk = unsafe extern "C" fn(env: *mut ChiikaEnv, cont: ChiikaCont) -> ContFuture;

#[allow(improper_ctypes)]
extern "C" {
    fn chiika_start_user(env: *mut ChiikaEnv, cont: ChiikaCont) -> ContFuture;
}

#[allow(improper_ctypes_definitions)]
extern "C" fn chiika_finish(env: *mut ChiikaEnv, _v: ChiikaValue) -> ContFuture {
    unsafe {
        (*env).cont = None;
    }
    Box::new(poll_fn(move |_context| Poll::Ready(_v)))
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn chiika_spawn(f: ChiikaThunk) -> u64 {
    let poller = make_poller(f);
    tokio::spawn(poller);
    0
}

#[no_mangle]
pub extern "C" fn chiika_start_tokio() {
    let poller = make_poller(chiika_start_user);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(poller);

    // Q: Need this?
    // sleep(Duration::from_millis(50)).await;
}

fn make_poller(f: ChiikaThunk) -> impl Future<Output = ()> {
    let mut env = ChiikaEnv::new();
    poll_fn(move |context| loop {
        let future = env
            .pop_rust_frame()
            .unwrap_or_else(|| unsafe { f(&mut env, chiika_finish) });
        let mut pinned = Pin::new(future);
        let tmp = pinned.as_mut().poll(context);
        match tmp {
            Poll::Ready(value) => {
                if let Some(cont) = env.cont {
                    let new_future = cont(&mut env, value);
                    env.push_rust_frame(new_future);
                } else {
                    return Poll::Ready(());
                }
            }
            Poll::Pending => {
                env.push_rust_frame(Pin::into_inner(pinned));
                return Poll::Pending;
            }
        }
    })
}
