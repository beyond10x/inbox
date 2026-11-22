//! The clock the service reads, as a parameter, and instants written as RFC 3339 text.

use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

use inbox::primitives::Timestamp;
use inbox_core::CommandClock;
use inbox_verify::Instant;

/// Where the service reads the time: the request window of every authentication, the instant
/// every envelope and delegation is checked at, every sweep of the expiry task, the deadline of a
/// message a post, an answer, a refusal, a read mark or a delivery mark meets ([`CommandReading`]),
/// and the rows a wait on `Pending` holds as due.
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

/// A clock that reads `start` when it is made and then `start` plus the system clock's distance
/// from the instant it was made, to the nanosecond: the conformance run's service, whose
/// envelopes carry fixed instants (`inbox serve --clock-start`, hidden). The system clock, not
/// the monotonic one, so a suspend of the host moves it on as it moves [`SystemClock`]. It
/// follows every step of the system clock: one back that lands after the instant it was made
/// moves the reading back by the step, and one back before that instant holds it at `start`.
#[derive(Debug)]
pub struct StartedClock {
    start: Instant,
    began: SystemTime,
}

impl StartedClock {
    /// A clock that reads `start` now.
    #[must_use]
    pub fn new(start: Instant) -> Self {
        Self {
            start,
            began: SystemTime::now(),
        }
    }
}

impl Clock for StartedClock {
    fn now(&self) -> Instant {
        let elapsed = SystemTime::now()
            .duration_since(self.began)
            .unwrap_or_default();
        let nanos = u64::from(self.start.subsec_nanos()) + u64::from(elapsed.subsec_nanos());
        let seconds = i64::try_from(elapsed.as_secs())
            .unwrap_or(i64::MAX)
            .saturating_add(i64::try_from(nanos / 1_000_000_000).unwrap_or(0));
        Instant::from_unix(
            self.start.unix_seconds().saturating_add(seconds),
            u32::try_from(nanos % 1_000_000_000).unwrap_or(0),
        )
    }
}

/// The service's [`Clock`] as the core's [`CommandClock`]: a command that asks for the time reads
/// the clock then, inside the backend's section where it runs, written as [`format_reading`]
/// writes it, the text the expiry task sends as `Expire`'s `now`. So a late answer and the sweep
/// compare one `expires` with readings of one clock at its full precision, and decide alike.
#[derive(Debug, Clone)]
pub(crate) struct CommandReading(pub(crate) Arc<dyn Clock>);

impl CommandClock for CommandReading {
    fn now(&self) -> Timestamp {
        Timestamp(format_reading(self.0.now()))
    }
}

/// `at` as RFC 3339 text in UTC with its fraction of a second, the digits it needs:
/// `2026-11-21T12:00:00.7Z`, and `2026-11-21T12:00:00Z` where the fraction is 0.
///
/// The clock reading a deadline is compared with: the reading `PostMessage`, `Answer`, `Refuse`,
/// `MarkDelivered` and `MarkRead` compare `expires` with, the one a wait on `Pending` holds its rows
/// to, and `Due`'s and `Expire`'s `now` and `keep_until` of
/// the expiry task. An instant outside the years RFC 3339 writes is written as
/// [`format_instant`] holds it, without a fraction.
#[must_use]
pub fn format_reading(at: Instant) -> String {
    let whole = format_instant(at);
    let nanos = at.subsec_nanos();
    if nanos == 0 || !(FIRST_SECOND..LAST_SECOND).contains(&at.unix_seconds()) {
        return whole;
    }
    let fraction = format!("{nanos:09}");
    format!(
        "{}.{}Z",
        whole.trim_end_matches('Z'),
        fraction.trim_end_matches('0')
    )
}

/// `at` as RFC 3339 text in UTC, truncated to the second: `2026-11-21T12:00:00Z`.
///
/// The text a client may send as its request timestamp; the reading a deadline is compared with
/// is [`format_reading`]'s, which writes the second as this does. RFC 3339 writes the years 0000 to 9999 only:
/// an instant after the last second of 9999 is written as that second, one before 0000 as the
/// first, so a `keep_until` far away is still an instant `Expire` reads.
#[must_use]
pub fn format_instant(at: Instant) -> String {
    let seconds = at.unix_seconds().clamp(FIRST_SECOND, LAST_SECOND);
    let (days, of_day) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60
    )
}

/// 0000-01-01T00:00:00Z, in seconds since 1970-01-01T00:00:00Z.
const FIRST_SECOND: i64 = -62_167_219_200;
/// 9999-12-31T23:59:59Z, in seconds since 1970-01-01T00:00:00Z.
const LAST_SECOND: i64 = 253_402_300_799;

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
    fn a_started_clock_reads_its_start_when_made() {
        let start = Instant::parse("2019-12-30T00:00:00Z").unwrap();
        let clock = super::StartedClock::new(start);
        let first = super::Clock::now(&clock);
        assert!(
            first.unix_seconds() - start.unix_seconds() <= 1,
            "{first:?}"
        );
        // Within the case the host clock does not step back, so a later reading is not earlier.
        assert!(super::Clock::now(&clock) >= first);
    }

    #[test]
    fn a_fraction_and_an_offset_are_not_written() {
        assert_eq!(
            format_instant(Instant::parse("2026-11-21T14:30:00.75+02:30").unwrap()),
            "2026-11-21T12:00:00Z"
        );
    }

    #[test]
    fn an_instant_outside_the_years_rfc_3339_writes_is_held_to_them() {
        for (at, text) in [
            (Instant::from_unix(i64::MAX, 0), "9999-12-31T23:59:59Z"),
            (Instant::from_unix(i64::MIN, 0), "0000-01-01T00:00:00Z"),
            (
                Instant::parse("9999-12-31T23:59:59Z").unwrap(),
                "9999-12-31T23:59:59Z",
            ),
            (
                Instant::parse("0000-01-01T00:00:00Z").unwrap(),
                "0000-01-01T00:00:00Z",
            ),
        ] {
            assert_eq!(format_instant(at), text);
        }
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
