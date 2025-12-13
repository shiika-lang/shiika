use shiika_ffi::core_class::{SkClass, SkInt, SkObject, SkString};
use shiika_ffi_macro::async_shiika_method;
use std::time::Duration;
use tokio::io::{stdout, AsyncWriteExt};

#[async_shiika_method("Object#class")]
async fn object_class(receiver: SkObject) -> SkClass {
    receiver.class()
}
#[async_shiika_method("Object#print")]
async fn object_print(_receiver: SkObject, n: SkInt) {
    let mut stdout = stdout();
    let output = format!("{}\n", n.val());
    stdout.write_all(output.as_bytes()).await.unwrap();
    stdout.flush().await.unwrap();
}

#[async_shiika_method("Object#puts")]
async fn object_puts(_receiver: SkObject, s: SkString) {
    let mut stdout = stdout();
    stdout.write_all(s.value()).await.unwrap();
    stdout.write_all(b"\n").await.unwrap();
    stdout.flush().await.unwrap();
}

#[async_shiika_method("Object#sleep_sec")]
async fn object_sleep_sec(_receiver: SkObject, n: SkInt) {
    let sec = n.val() as u64;
    tokio::time::sleep(Duration::from_secs(sec)).await;
}
