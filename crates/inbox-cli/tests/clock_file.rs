//! `inbox serve --clock-file <path>` (a debug build's hidden flag, `src/clock_file.rs`): the
//! service's clock reads the instant the file holds each time it asks for the time, so a test
//! moves it past an `expires` without sleeping; and it fails closed. The forms the reading
//! refuses are the unit cases of `src/clock_file.rs`; these cases hold what the process does.

use crate::support;

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use inbox_server::format_reading;
use inbox_verify::Instant;

use support::{BOB, LOCAL, REFUSED, TRANSPORT, USAGE, keys_only, run, serve_refused};

/// Text in a clock file that is no instant; no line of the service repeats it.
const MARKER: &str = "not-an-instant-7f3a";

/// The system clock's second.
fn this_second() -> Instant {
    Instant::from_unix(
        Instant::from_system_time(SystemTime::now()).unix_seconds(),
        0,
    )
}

/// Replaces the clock file whole, as a test that moves the clock does.
fn set_clock(dir: &Path, text: &str) {
    fs::write(dir.join("clock.new"), text).expect("the clock file is written");
    fs::rename(dir.join("clock.new"), dir.join("clock")).expect("the clock file is replaced");
}

fn serve_args<'a>(operator: &'a str, rest: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "--listen",
        "127.0.0.1:0",
        "--key-file",
        "keys.json",
        "--operator-public-key",
        operator,
    ];
    args.extend_from_slice(rest);
    args
}

/// A file that is missing, cannot be read, is empty or holds no RFC 3339 instant ends `inbox
/// serve` before it binds, exit 4, naming the flag and not the file's text; with
/// `--clock-start` beside it, the flag is wrong usage.
#[test]
fn a_clock_file_that_holds_no_instant_stops_serve_before_it_binds() {
    let (dir, operator) = keys_only("clock-file-start");
    let start = |what: &str| {
        let refused = serve_refused(
            &dir.path,
            &serve_args(&operator, &["--clock-file", "clock"]),
            &[],
        );
        assert_eq!(refused.status, LOCAL, "{what}: {refused:#?}");
        assert!(
            refused.stderr.contains("--clock-file clock: "),
            "{what}: {refused:#?}"
        );
        assert!(!refused.stderr.contains(MARKER), "{what}: {refused:#?}");
        refused.stderr
    };
    assert!(start("missing").contains("No such file"));
    fs::create_dir(dir.join("clock")).expect("a directory where the file goes");
    start("a directory");
    fs::remove_dir(dir.join("clock")).expect("removed");
    set_clock(&dir.path, "");
    assert!(start("empty").contains("it is empty"));
    set_clock(&dir.path, MARKER);
    assert!(start("no instant").contains("it does not hold one RFC 3339 instant"));

    set_clock(&dir.path, &format_reading(this_second()));
    let both = serve_refused(
        &dir.path,
        &serve_args(
            &operator,
            &[
                "--clock-file",
                "clock",
                "--clock-start",
                "2019-12-30T00:00:00Z",
            ],
        ),
        &[],
    );
    assert_eq!(both.status, USAGE, "{both:#?}");
}

/// The clock reads the file at each request: moved past an `expires`, the next post of an
/// envelope that expires then is refused `expired-on-arrival`, with no wait on real time. The
/// start names the file on standard error.
#[test]
fn the_service_reads_its_clock_from_the_file_at_each_request() {
    let (dir, operator) = keys_only("clock-file-moves");
    let start = this_second();
    set_clock(&dir.path, &format_reading(start));
    let service = support::Service::start(
        &dir.path,
        &serve_args(&operator, &["--clock-file", "clock"]),
        &[],
    );
    assert!(
        service.stderr().contains(&format!(
            "the service's clock reads the file clock, at {} now, not the system clock",
            format_reading(start)
        )),
        "{}",
        service.stderr()
    );
    let env = [("INBOX_URL", service.url.as_str())];
    run(&dir.path, &["open", "--secret-key-file", "bob.key"], &env).ok();
    // 30 s after the system clock just before the post: `created` is the system clock's, and the
    // service refuses an `expires` at or before it, so one process start is inside the margin.
    let expires_at = this_second().plus_seconds(30);
    let expires = format_reading(expires_at);
    let post = || {
        run(
            &dir.path,
            &[
                "post",
                "--secret-key-file",
                "alice.key",
                "--outbox",
                "alice.outbox",
                "--to",
                BOB,
                "--body",
                "on the file's clock",
                "--expires",
                &expires,
            ],
            &env,
        )
    };
    post().ok();
    set_clock(&dir.path, &format_reading(expires_at.plus_seconds(1)));
    let late = post();
    assert_eq!(late.status, REFUSED, "{late:#?}");
    assert!(late.stderr.contains("AlreadyExpired"), "{late:#?}");
}

/// While it serves, a clock file that goes back in time, or stops holding an instant, ends the
/// process at once with exit 4 and one line naming the file and the cause; the request that
/// asked for the time gets no answer. The bound of 10 s only ends a hang.
#[test]
fn a_clock_file_that_goes_bad_while_serving_ends_serve_with_exit_4() {
    let (dir, operator) = keys_only("clock-file-running");
    for (what, bad, cause) in [
        ("back in time", None, "it went back to "),
        (
            "no instant",
            Some(MARKER),
            "it does not hold one RFC 3339 instant",
        ),
    ] {
        let start = this_second();
        set_clock(&dir.path, &format_reading(start));
        let mut service = support::Service::start(
            &dir.path,
            &serve_args(&operator, &["--clock-file", "clock"]),
            &[],
        );
        let text = bad.map_or_else(|| format_reading(start.plus_seconds(-1)), str::to_owned);
        set_clock(&dir.path, &text);
        let open = run(
            &dir.path,
            &["open", "--secret-key-file", "bob.key"],
            &[("INBOX_URL", service.url.as_str())],
        );
        assert_eq!(open.status, TRANSPORT, "{what}: {open:#?}");
        let status = service.exit_within(Duration::from_secs(10));
        assert_eq!(
            status.and_then(|status| status.code()),
            Some(LOCAL),
            "{what}: {}",
            service.stderr()
        );
        let stderr = service.stderr();
        assert!(
            stderr.contains(&format!("inbox serve: the clock file clock: {cause}"))
                && stderr.trim_end().ends_with(": stopping"),
            "{what}: {stderr}"
        );
        assert!(!stderr.contains(MARKER), "{what}: {stderr}");
    }
}
