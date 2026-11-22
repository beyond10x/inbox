//! Adversary pass 1 on story:no-test-holds-a-wall-clock-time: `inbox serve --clock-file <path>`
//! against what `docs/design/service.md` says of it ("It fails closed", and the table of causes).
//! Each bound of 10 s here only ends a hang.

use crate::support;

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use inbox_server::format_reading;
use inbox_verify::Instant;

use support::{LOCAL, keys_only, serve_refused};

fn this_second() -> Instant {
    Instant::from_unix(
        Instant::from_system_time(SystemTime::now()).unix_seconds(),
        0,
    )
}

fn serve_args(operator: &str) -> Vec<&str> {
    vec![
        "--listen",
        "127.0.0.1:0",
        "--key-file",
        "keys.json",
        "--operator-public-key",
        operator,
        "--clock-file",
        "clock",
    ]
}

fn mkfifo(path: &Path) {
    let made = Command::new("mkfifo")
        .arg(path)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "mkfifo {}", path.display());
}

/// The exit code of `child` within 10 s, killing it when it is still running then.
fn exit_within_10_s(child: &mut std::process::Child) -> Option<i32> {
    let started = std::time::Instant::now();
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            return status.code();
        }
        if started.elapsed() >= Duration::from_secs(10) {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// service.md: a clock file that "cannot be read" is "refused before binding, exit 4". A FIFO
/// with no writer is opened with a blocking `open(2)`, so `inbox serve` neither binds nor exits:
/// it hangs before its first line.
#[test]
fn a_clock_file_that_is_a_fifo_with_no_writer_stops_serve_before_it_binds_with_exit_4() {
    let (dir, operator) = keys_only("adv-clock-file-fifo-start");
    mkfifo(&dir.join("clock"));
    let mut serve = support::command(&dir.path)
        .arg("serve")
        .args(serve_args(&operator))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the binary starts");
    let code = exit_within_10_s(&mut serve);
    assert_eq!(
        code,
        Some(LOCAL),
        "inbox serve with a FIFO as its clock file (None: still running 10 s later, killed)"
    );
}

/// service.md: while it serves, a clock file that cannot be read ends the process "at once, exit
/// 4". Replaced by a FIFO (renamed over it, as the documented writer does), the first reading
/// blocks in `open(2)` while it holds the clock's lock, and every request after it waits behind
/// that lock: the service hangs and never ends.
#[test]
fn a_clock_file_replaced_by_a_fifo_while_serving_ends_serve_with_exit_4() {
    let (dir, operator) = keys_only("adv-clock-file-fifo-serving");
    fs::write(dir.join("clock"), format_reading(this_second())).expect("the clock file");
    let mut service = support::Service::start(&dir.path, &serve_args(&operator), &[]);
    mkfifo(&dir.join("clock.new"));
    fs::rename(dir.join("clock.new"), dir.join("clock")).expect("the FIFO replaces the file");
    let mut client = support::command(&dir.path)
        .args(["open", "--secret-key-file", "bob.key"])
        .env("INBOX_URL", &service.url)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the client starts");
    let status = service.exit_within(Duration::from_secs(10));
    let _ = client.kill();
    let _ = client.wait();
    assert_eq!(
        status.and_then(|status| status.code()),
        Some(LOCAL),
        "inbox serve after its clock file became a FIFO (None: still running 10 s later): {}",
        service.stderr()
    );
}

/// service.md, the table: a file "longer than 64 bytes, or not one RFC 3339 instant" is
/// "refused, exit 4, `it does not hold one RFC 3339 instant`".
#[test]
fn a_clock_file_over_64_bytes_is_refused_with_the_cause_service_md_names() {
    let (dir, operator) = keys_only("adv-clock-file-long");
    fs::write(dir.join("clock"), "2".repeat(65)).expect("the clock file");
    let refused = serve_refused(&dir.path, &serve_args(&operator), &[]);
    assert_eq!(refused.status, LOCAL, "{refused:#?}");
    assert!(
        refused
            .stderr
            .contains("--clock-file clock: it does not hold one RFC 3339 instant"),
        "{refused:#?}"
    );
}
