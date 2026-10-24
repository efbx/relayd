use std::{
    fmt::{self, Display, Formatter},
    time::{Duration, UNIX_EPOCH},
};

use bytesize::ByteSize;
use humantime::{FormattedDuration, Rfc3339Timestamp};

#[derive(Debug)]
pub struct Human;

impl Human {
    #[must_use]
    pub fn duration(milliseconds: u64) -> FormattedDuration {
        humantime::format_duration(Duration::from_millis(milliseconds))
    }

    #[must_use]
    pub fn size(bytes: u64) -> ByteSize {
        ByteSize(bytes)
    }

    #[must_use]
    pub fn time(milliseconds: u64) -> Rfc3339Timestamp {
        let moment = UNIX_EPOCH
            .checked_add(Duration::from_millis(milliseconds))
            .unwrap_or(UNIX_EPOCH);
        humantime::format_rfc3339_seconds(moment)
    }

    #[must_use]
    pub fn clock(milliseconds: u64) -> Clock {
        Clock(Duration::from_millis(milliseconds))
    }
}

#[derive(Debug)]
pub struct Clock(Duration);

impl Display for Clock {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let seconds = self.0.as_secs() % 86_400;
        write!(
            formatter,
            "{:02}:{:02}:{:02}",
            seconds / 3_600,
            seconds % 3_600 / 60,
            seconds % 60
        )
    }
}

#[derive(Debug)]
pub struct Label<T>(pub Option<T>);

impl<T: Display> Display for Label<T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(value) => value.fmt(formatter),
            None => formatter.write_str("—"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_read_like_humantime() {
        assert_eq!(Human::duration(999).to_string(), "999ms");
        assert_eq!(Human::duration(1_250).to_string(), "1s 250ms");
        assert_eq!(Human::duration(65_000).to_string(), "1m 5s");
        assert_eq!(Human::duration(3_725_000).to_string(), "1h 2m 5s");
    }

    #[test]
    fn times_are_utc_to_the_second() {
        assert_eq!(Human::time(0).to_string(), "1970-01-01T00:00:00Z");
        assert_eq!(
            Human::time(1_700_000_000_123).to_string(),
            "2023-11-14T22:13:20Z"
        );
    }

    #[test]
    fn clocks_show_the_utc_time_of_day() {
        assert_eq!(Human::clock(0).to_string(), "00:00:00");
        assert_eq!(Human::clock(1_700_000_000_123).to_string(), "22:13:20");
    }

    #[test]
    fn sizes_use_binary_units() {
        assert_eq!(Human::size(2).to_string(), "2 B");
        assert_eq!(Human::size(10_035).to_string(), "9.8 KiB");
        assert_eq!(Human::size(536_887).to_string(), "524.3 KiB");
        assert_eq!(Human::size(1_258_291).to_string(), "1.2 MiB");
    }
}
