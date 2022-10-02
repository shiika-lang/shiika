use shiika_ffi_macro::shiika_method_ref;
////mod rs_zone;
mod sk_instant;
////mod sk_time;
////mod sk_zone;
use crate::builtin::time::sk_instant::SkInstant;
use crate::builtin::{SkClass, SkInt};
use chrono::Utc;
use shiika_ffi_macro::shiika_method;
////use sk_time::{SkInstant, SkTime};

shiika_method_ref!(
    "Meta:Time::Instant#new",
    fn(receiver: *const u8, sec: SkInt, nano_frac: SkInt) -> SkInstant,
    "meta_time_instant_new"
);

//#[shiika_method("Meta:Time#local")]
//pub extern "C" fn meta_time_local(_receiver: SkCls) -> SkTime {
//    let inst = meta_time_instant_now();
//    let zone = SkZone::local();
//    time_new(inst, zone)
//}

#[shiika_method("Meta:Time::Instant#now")]
pub extern "C" fn meta_time_instant_now(_receiver: SkClass) -> SkInstant {
    let t = Utc::now();
    meta_time_instant_new(
        std::ptr::null(),
        t.timestamp().into(),
        t.timestamp_nanos().into(),
    )
}

//#[shiika_method("Time#to_plain")]
//pub extern "C" fn time_to_plain(receiver: SkTime) -> SkTime {
//    let (secs, nsecs) = receiver.epoch();
//    match receiver.zone() {
//        RsZone::Utc => {
//            let t = Utc.timestamp(secs, nsecs);
//            sk_plain_date_time(t)
//        }
//        RsZone::Local => {
//        }
//        RsZone::Iana(tz) => {
//        }
//    }
//
//    let t = meta_time_instant_now();
//    let inst = time_instant_new(t.timestamp().into(), t.timestamp_nanos().into());
//
//    time_new(inst, zone)
//}
//
//fn sk_plain_date_time(t: impl Timelike) -> SkPlainDateTime {
//    let plain_date = meta_plain_date
//}
