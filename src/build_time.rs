use std::sync::LazyLock;

use jiff::{Timestamp, Zoned, tz::TimeZone};

pub static BUILD_TIME: LazyLock<Zoned> = LazyLock::new(|| {
    Timestamp::from_second(env!("BUILD_TIME_UNIX").parse().unwrap())
        .unwrap()
        .to_zoned(TimeZone::UTC)
});
