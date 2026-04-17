use chrono::{DateTime, Datelike, Local, Timelike, Utc};
use shiika_ffi::core_class::time::RsZone;
use shiika_ffi::core_class::{SkClass, SkInt};
use shiika_ffi::core_class::{SkInstant, SkPlainDate, SkPlainDateTime, SkPlainTime, SkTime};
use shiika_ffi_macro::{shiika_method, shiika_method_ref};

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
    unsafe {
        meta_time_instant_new(
            shiika_const_Time_Instant.dup(),
            t.timestamp_nanos_opt()
                .expect("timestamp out of range")
                .into(),
        )
    }
}

#[shiika_method("Time#to_plain")]
pub extern "C" fn time_to_plain(receiver: SkTime) -> SkPlainDateTime {
    let nsecs = receiver.epoch();
    let utc = DateTime::from_timestamp_nanos(nsecs);
    match receiver.zone() {
        RsZone::Utc => sk_plain_date_time(utc),
        RsZone::Local => {
            let t: DateTime<Local> = utc.with_timezone(&Local);
            sk_plain_date_time(t)
        }
    }
}

/// Create a `Time::PlainDateTime` from a Timelike struct.
fn sk_plain_date_time(t: impl Timelike + Datelike) -> SkPlainDateTime {
    unsafe {
        let plain_date = meta_time_plain_date_new(
            shiika_const_Time_PlainDate.dup(),
            (t.year() as i64).into(),
            (t.month() as i64).into(),
            (t.day() as i64).into(),
        );
        let plain_time = meta_time_plain_time_new(
            shiika_const_Time_PlainTime.dup(),
            (t.hour() as i64).into(),
            (t.minute() as i64).into(),
            (t.second() as i64).into(),
            (t.nanosecond() as i64).into(),
        );
        meta_time_plain_date_time_new(
            shiika_const_Time_PlainDateTime.dup(),
            plain_date,
            plain_time,
        )
    }
}
