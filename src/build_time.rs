use std::sync::LazyLock;

use chrono::{DateTime, FixedOffset};

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub static BUILD_TIME: LazyLock<DateTime<FixedOffset>> =
    LazyLock::new(|| DateTime::parse_from_rfc2822(built_info::BUILT_TIME_UTC).unwrap());
