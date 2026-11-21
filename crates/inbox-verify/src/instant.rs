//! An instant on the UTC timeline, read from RFC 3339 text. The library reads no clock: every
//! check takes one of these as an argument.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use inbox::primitives::Timestamp;
use inbox::server::json::{self, Value};

/// An instant on the UTC timeline, to the nanosecond.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant {
    seconds: i64,
    nanos: u32,
}

/// Text that is not an RFC 3339 instant as the generated reader admits one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstantError(pub String);

impl fmt::Display for InstantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` is not an RFC 3339 instant", self.0)
    }
}

impl std::error::Error for InstantError {}

impl Instant {
    /// The instant this many seconds and nanoseconds after 1970-01-01T00:00:00Z (before it, for
    /// negative seconds). Nanoseconds past one second carry into the seconds.
    #[must_use]
    pub fn from_unix(seconds: i64, nanos: u32) -> Self {
        Self {
            seconds: seconds.saturating_add(i64::from(nanos / 1_000_000_000)),
            nanos: nanos % 1_000_000_000,
        }
    }

    /// The instant a `SystemTime` names. A conversion, not a clock read: the caller reads the clock.
    #[must_use]
    pub fn from_system_time(time: SystemTime) -> Self {
        match time.duration_since(UNIX_EPOCH) {
            Ok(after) => Self::from_unix(
                i64::try_from(after.as_secs()).unwrap_or(i64::MAX),
                after.subsec_nanos(),
            ),
            Err(before) => {
                let before = before.duration();
                let seconds = i64::try_from(before.as_secs()).unwrap_or(i64::MAX);
                match before.subsec_nanos() {
                    0 => Self::from_unix(-seconds, 0),
                    nanos => Self::from_unix(-seconds - 1, 1_000_000_000 - nanos),
                }
            }
        }
    }

    /// Seconds since 1970-01-01T00:00:00Z.
    #[must_use]
    pub fn unix_seconds(self) -> i64 {
        self.seconds
    }

    /// The nanoseconds within the second, below 1 000 000 000.
    #[must_use]
    pub fn subsec_nanos(self) -> u32 {
        self.nanos
    }

    /// This instant moved by a whole number of seconds.
    #[must_use]
    pub fn plus_seconds(self, seconds: i64) -> Self {
        Self {
            seconds: self.seconds.saturating_add(seconds),
            nanos: self.nanos,
        }
    }

    /// Reads RFC 3339 text as the generated reader (`timestamp_at`) admits it. The offset is
    /// applied, a fraction is truncated to nanoseconds, and a leap second (`:60`) reads as the last
    /// nanosecond of the second before it.
    ///
    /// # Errors
    ///
    /// [`InstantError`] when the generated reader refuses the text.
    pub fn parse(text: &str) -> Result<Self, InstantError> {
        let refused = || InstantError(text.to_owned());
        json::timestamp_at(&Value::Text(text.to_owned()), "", "an RFC 3339 instant")
            .map_err(|_| refused())?;
        let bytes = text.as_bytes();
        let digits = |from: usize, to: usize| -> i64 {
            bytes[from..to]
                .iter()
                .fold(0, |held, digit| held * 10 + i64::from(digit - b'0'))
        };
        let (year, month, day) = (digits(0, 4), digits(5, 7), digits(8, 10));
        let (hour, minute, mut second) = (digits(11, 13), digits(14, 16), digits(17, 19));
        let mut at = 19;
        let mut nanos: u32 = 0;
        if bytes[at] == b'.' {
            at += 1;
            let mut scale = 100_000_000;
            while let Some(digit) = bytes.get(at).filter(|byte| byte.is_ascii_digit()) {
                nanos += u32::from(digit - b'0') * scale;
                scale /= 10;
                at += 1;
            }
        }
        let offset = match bytes[at] {
            b'+' => digits(at + 1, at + 3) * 3600 + digits(at + 4, at + 6) * 60,
            b'-' => -(digits(at + 1, at + 3) * 3600 + digits(at + 4, at + 6) * 60),
            _ => 0,
        };
        if second == 60 {
            second = 59;
            nanos = 999_999_999;
        }
        let local = days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second;
        Ok(Self {
            seconds: local - offset,
            nanos,
        })
    }
}

impl TryFrom<&Timestamp> for Instant {
    type Error = InstantError;

    fn try_from(timestamp: &Timestamp) -> Result<Self, Self::Error> {
        Self::parse(&timestamp.0)
    }
}

/// Days from 1970-01-01 to this proleptic Gregorian date, counted in 400-year eras.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::Instant;

    #[test]
    fn the_epoch_and_known_instants_read() {
        assert_eq!(
            Instant::parse("1970-01-01T00:00:00Z").unwrap(),
            Instant::from_unix(0, 0)
        );
        assert_eq!(
            Instant::parse("2000-03-01T00:00:00Z").unwrap(),
            Instant::from_unix(951_868_800, 0)
        );
        assert_eq!(
            Instant::parse("2026-11-21T12:00:00Z").unwrap(),
            Instant::from_unix(1_795_262_400, 0)
        );
        assert_eq!(
            Instant::parse("1969-12-31T23:59:59Z").unwrap(),
            Instant::from_unix(-1, 0)
        );
    }

    #[test]
    fn an_offset_is_applied() {
        assert_eq!(
            Instant::parse("2026-11-21T14:30:00+02:30").unwrap(),
            Instant::parse("2026-11-21T12:00:00Z").unwrap()
        );
        assert_eq!(
            Instant::parse("2026-11-21T07:00:00-05:00").unwrap(),
            Instant::parse("2026-11-21t12:00:00z").unwrap()
        );
    }

    #[test]
    fn a_fraction_is_truncated_to_nanoseconds() {
        assert_eq!(
            Instant::parse("1970-01-01T00:00:01.1234567899Z").unwrap(),
            Instant::from_unix(1, 123_456_789)
        );
        assert_eq!(
            Instant::parse("1970-01-01T00:00:01.5Z").unwrap(),
            Instant::from_unix(1, 500_000_000)
        );
    }

    #[test]
    fn a_leap_second_reads_as_the_last_nanosecond_before_it() {
        assert_eq!(
            Instant::parse("2016-12-31T23:59:60Z").unwrap(),
            Instant::parse("2016-12-31T23:59:59.999999999Z").unwrap()
        );
    }

    #[test]
    fn text_the_generated_reader_refuses_is_refused() {
        for text in [
            "",
            "2026-11-21",
            "2026-11-21T12:00:00",
            "2026-02-30T00:00:00Z",
            "2026-11-21 12:00:00Z",
            "2026-11-21T12:00:60Z",
        ] {
            assert!(Instant::parse(text).is_err(), "{text:?} read");
        }
    }

    #[test]
    fn a_system_time_converts_on_both_sides_of_the_epoch() {
        use std::time::{Duration, UNIX_EPOCH};
        assert_eq!(
            Instant::from_system_time(UNIX_EPOCH + Duration::new(5, 7)),
            Instant::from_unix(5, 7)
        );
        assert_eq!(
            Instant::from_system_time(UNIX_EPOCH - Duration::new(1, 250_000_000)),
            Instant::from_unix(-2, 750_000_000)
        );
    }
}
