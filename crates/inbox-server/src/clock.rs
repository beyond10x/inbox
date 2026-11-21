//! The clock the service reads, as a parameter, and instants written as RFC 3339 text.

use std::fmt;
use std::sync::{Mutex, PoisonError};
use std::time::SystemTime;

use inbox_verify::Instant;

/// Where the service reads the time: the request window of every authentication, the instant
/// every envelope and delegation is checked at, and every sweep of the expiry task.
///
/// A parameter of the library, so a test moves time without sleeping ([`ManualClock`]).
pub trait Clock: Send + Sync + fmt::Debug {
    /// The current instant.
    fn now(&self) -> Instant;
}

/// The system clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::from_system_time(SystemTime::now())
    }
}

/// A clock that reads what it was last set to: for a test, or an embedding that keeps its own time.
#[derive(Debug)]
pub struct ManualClock {
    at: Mutex<Instant>,
}

impl ManualClock {
    /// A clock that reads `at` until it is set or moved.
    #[must_use]
    pub fn new(at: Instant) -> Self {
        Self { at: Mutex::new(at) }
    }

    /// Sets the reading.
    pub fn set(&self, at: Instant) {
        *self.at.lock().unwrap_or_else(PoisonError::into_inner) = at;
    }

    /// Moves the reading by a whole number of seconds, back for a negative number.
    pub fn advance(&self, seconds: i64) {
        let mut at = self.at.lock().unwrap_or_else(PoisonError::into_inner);
        *at = at.plus_seconds(seconds);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Instant {
        *self.at.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// `at` as RFC 3339 text in UTC, truncated to the second: `2026-11-21T12:00:00Z`.
///
/// The text the service writes into `Due`'s `now` and `Expire`'s `now` and `keep_until`, and the
/// text a client may send as its request timestamp. A year outside 0 to 9999 is written with more
/// or fewer digits and is no RFC 3339 instant; no clock this service reads is there.
#[must_use]
pub fn format_instant(at: Instant) -> String {
    let seconds = at.unix_seconds();
    let (days, of_day) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60
    )
}

/// The proleptic Gregorian date this many days after 1970-01-01, counted in 400-year eras: the
/// inverse of the day count `inbox_verify::Instant::parse` reads with.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use inbox_verify::Instant;

    use super::format_instant;

    #[test]
    fn an_instant_is_written_as_the_text_it_reads_from() {
        for text in [
            "1970-01-01T00:00:00Z",
            "1969-12-31T23:59:59Z",
            "2000-02-29T12:34:56Z",
            "2026-11-21T12:00:00Z",
            "2100-03-01T00:00:00Z",
            "2400-12-31T23:59:59Z",
        ] {
            assert_eq!(format_instant(Instant::parse(text).unwrap()), text);
        }
    }

    #[test]
    fn a_fraction_and_an_offset_are_not_written() {
        assert_eq!(
            format_instant(Instant::parse("2026-11-21T14:30:00.75+02:30").unwrap()),
            "2026-11-21T12:00:00Z"
        );
    }

    #[test]
    fn every_day_of_four_centuries_round_trips() {
        let start = Instant::parse("1900-01-01T00:00:00Z").unwrap();
        for day in 0..146_097 {
            let at = start.plus_seconds(day * 86_400 + 3_723);
            assert_eq!(Instant::parse(&format_instant(at)).unwrap(), at);
        }
    }
}
