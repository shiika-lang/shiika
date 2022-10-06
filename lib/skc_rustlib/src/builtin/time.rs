use shiika_ffi_macro::shiika_method_ref;
mod rs_zone;
mod sk_instant;
mod sk_plain;
mod sk_time;
mod sk_zone;
use crate::builtin::time::rs_zone::RsZone;
use crate::builtin::time::sk_instant::SkInstant;
use crate::builtin::time::sk_plain::{SkPlainDate, SkPlainDateTime, SkPlainTime};
use crate::builtin::time::sk_time::SkTime;
use crate::builtin::time::sk_zone::SkZone;
use crate::builtin::{SkClass, SkInt};
use chrono::{Datelike, Local, TimeZone, Timelike, Utc};
use shiika_ffi_macro::shiika_method;

extern "C" {
    #[allow(improper_ctypes)]
    static shiika_const_Time_Instant: SkClass;
    #[allow(improper_ctypes)]
    static shiika_const_Time_PlainDateTime: SkClass;
    #[allow(improper_ctypes)]
    static shiika_const_Time_PlainDate: SkClass;
    #[allow(improper_ctypes)]
    static shiika_const_Time_PlainTime: SkClass;
}

shiika_method_ref!(
    "Meta:Time::Instant#new",
    fn(receiver: SkClass, nano_timestamp: SkInt) -> SkInstant,
    "meta_time_instant_new"
);
shiika_method_ref!(
    "Meta:Time::PlainDateTime#new",
    fn(receiver: SkClass, d: SkPlainDate, t: SkPlainTime) -> SkPlainDateTime,
    "meta_time_plain_date_time_new"
);
shiika_method_ref!(
    "Meta:Time::PlainDate#new",
    fn(receiver: SkClass, y: SkInt, m: SkInt, d: SkInt) -> SkPlainDate,
    "meta_time_plain_date_new"
);
shiika_method_ref!(
    "Meta:Time::PlainTime#new",
    fn(receiver: SkClass, h: SkInt, m: SkInt, s: SkInt, n: SkInt) -> SkPlainTime,
    "meta_time_plain_time_new"
);

#[shiika_method("Meta:Time::Instant#now")]
pub extern "C" fn meta_time_instant_now(_receiver: SkClass) -> SkInstant {
    let t = Utc::now();
    unsafe { meta_time_instant_new(shiika_const_Time_Instant.dup(), t.timestamp_nanos().into()) }
}

#[shiika_method("Time#to_plain")]
pub extern "C" fn time_to_plain(receiver: SkTime) -> SkPlainDateTime {
    let nsecs = receiver.epoch();
    match receiver.zone() {
        RsZone::Utc => {
            let t = Utc.timestamp_nanos(nsecs);
            sk_plain_date_time(t)
        }
        RsZone::Local => {
            let t = Local.timestamp_nanos(nsecs);
            sk_plain_date_time(t)
        } //        RsZone::Iana(tz) => {
          //            todo!();
          //        }
    }
}

/// Create a `Time::PlainDateTime` from a Timelike struct.
fn sk_plain_date_time(t: impl Timelike + Datelike) -> SkPlainDateTime {
    unsafe {
        let plain_date = meta_time_plain_date_new(
            shiika_const_Time_PlainDate.dup(),
            t.year().into(),
            t.month().into(),
            t.day().into(),
        );
        let plain_time = meta_time_plain_time_new(
            shiika_const_Time_PlainTime.dup(),
            t.hour().into(),
            t.minute().into(),
            t.second().into(),
            t.nanosecond().into(),
        );
        meta_time_plain_date_time_new(
            shiika_const_Time_PlainDateTime.dup(),
            plain_date,
            plain_time,
        )
    }
}
