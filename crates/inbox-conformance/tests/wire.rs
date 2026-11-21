//! The wire target: scenarios run against a running `inbox serve` over HTTP, a stand-in listener
//! that answers wrongly, the steps the wire cannot witness, a service that does not start, and
//! signatures the service refuses.
//!
//! The binary is the `inbox` of `crates/inbox-cli`, found beside this package's own binary in the
//! same target directory, where `cargo test --workspace` builds it; a missing binary fails the
//! case that needs it, naming the cause. No key material is written into this file: the target
//! makes every key at run time.

use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command as Process;
use std::sync::{Arc, Mutex};

use inbox_conformance::suite::{self, Scenario};
use inbox_conformance::{Status, WireTarget, run_scenario};

/// The `coverage` block of a coverage suite whose one scenario, `probe`, is generated.
const PROBE_COVERAGE: &str = r#"{"authored": [], "authored_sources": {},
    "counts": {"authored": 0, "generated": 1, "outside": 0, "refused": 0},
    "generated": ["probe"], "knowledge": "complete_inventory", "outside": [], "refused": [],
    "selection": {"filter": {"kind": "all"}, "origins": "generated_and_authored", "scope": {"kind": "system"}}}"#;

const EVENTS: &str = r#"["inbox.messaging.InboxClosed", "inbox.messaging.InboxOpened",
    "inbox.messaging.MessageDelivered", "inbox.messaging.MessagePosted"]"#;

fn suite_text(steps: &str) -> String {
    format!(
        r#"{{"provenance": {{"suite_version": "ess-conformance/65", "scenario_initial_state": "empty"}},
            "coverage": {PROBE_COVERAGE}, "events": {EVENTS},
            "scenarios": {{"probe": {{"purpose": "a probe", "source": [], "steps": {steps}}}}}}}"#
    )
}

fn scenario(steps: &str) -> Scenario {
    let mut read = suite::read(&suite_text(steps)).expect("the probe suite reads");
    read.scenarios.remove(0)
}

/// The built `inbox` binary, beside this package's binary.
fn inbox() -> PathBuf {
    let path = Path::new(env!("CARGO_BIN_EXE_inbox-conformance")).with_file_name("inbox");
    assert!(
        path.is_file(),
        "the inbox binary is not built at {}: build it with `cargo build -p inbox-cli` (`cargo test --workspace` builds it)",
        path.display()
    );
    path
}

/// A fresh work directory for one case.
fn work(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("wire-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the work directory is created");
    path
}

/// Whether the process `pid` still exists (a zombie included).
fn alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

fn open(who: &str) -> String {
    format!(
        r#"{{"step": "execute_command", "command": "inbox.messaging.OpenInbox", "actor": "inbox.messaging.Client",
            "caller": {{"inbox": "{who}", "principal": "{who}"}},
            "input": {{"adapter": {{"kind": "literal", "value": "claude_code"}}, "address": {{"kind": "literal", "value": "{who}"}}}}}}"#
    )
}

const WORKER: &str = "agent:box/worker";
const PLANNER: &str = "agent:ws/planner";
const MESSAGE: &str = "01JGT0DK000000000000000001";

/// A note from the planner to the worker, with the suite's placeholder signature.
fn post() -> String {
    format!(
        r#"{{"step": "execute_command", "command": "inbox.messaging.PostMessage", "actor": "inbox.messaging.Client",
        "caller": {{"inbox": "{PLANNER}", "principal": "{PLANNER}"}},
        "input": {{"body": {{"kind": "literal", "value": "Build 512 is green."}},
          "created": {{"kind": "literal", "value": "2019-12-31T00:00:00Z"}},
          "dedup_key": {{"kind": "literal", "value": "{MESSAGE}"}},
          "expires": {{"kind": "literal", "value": "2020-01-01T00:00:00Z"}},
          "from": {{"kind": "literal", "value": "{PLANNER}"}},
          "kind": {{"kind": "literal", "value": "note"}},
          "message_id": {{"kind": "literal", "value": "{MESSAGE}"}},
          "on_behalf_of": {{"kind": "literal", "value": "{PLANNER}"}},
          "signature": {{"kind": "literal", "value": {{"key_id": "signature.key_id", "value": "signature.value"}}}},
          "thread": {{"kind": "literal", "value": "{MESSAGE}"}},
          "to": {{"kind": "literal", "value": "{WORKER}"}}}}}}"#
    )
}

fn expect_outcome(command: &str, outcome: &str) -> String {
    format!(
        r#"{{"step": "expect_outcome", "outcome": {{"command": "inbox.messaging.{command}", "outcome": "{outcome}"}}}}"#
    )
}

/// Open the worker's inbox, post the planner's note, and find it in the worker's `Pending`, the
/// planner's `Outbox` and the event the post published; `outcome` is what the post should
/// answer.
fn exchange(outcome: &str) -> String {
    let posted = outcome == "posted";
    let mut steps = vec![
        open(WORKER),
        expect_outcome("OpenInbox", "opened"),
        r#"{"step": "expect_event", "event": "inbox.messaging.InboxOpened", "payload": {"address": "agent:box/worker"}}"#.to_owned(),
        r#"{"step": "capture_instance", "instance": "inbox", "entity": "inbox.messaging.Inbox", "event": "inbox.messaging.InboxOpened", "field": "address"}"#.to_owned(),
    ];
    if outcome == "signature-invalid" {
        steps.push(
            r#"{"step": "configure_external_outcome", "force": {"command": "inbox.messaging.PostMessage", "outcome": "signature-invalid"}}"#.to_owned(),
        );
    }
    steps.push(post());
    steps.push(expect_outcome("PostMessage", outcome));
    if posted {
        steps.push(format!(
            r#"{{"step": "expect_event", "event": "inbox.messaging.MessagePosted",
                "payload": {{"message_id": "{MESSAGE}", "to": "{WORKER}", "from": "{PLANNER}", "thread": "{MESSAGE}", "kind": "note"}}}}"#
        ));
    } else {
        steps.push(
            r#"{"step": "expect_no_event", "event": "inbox.messaging.MessagePosted"}"#.to_owned(),
        );
    }
    let held = if posted { "at_least" } else { "at_most" };
    let bound = if posted { 1 } else { 0 };
    steps.push(
        r#"{"step": "query_view", "view": "inbox.messaging.Pending", "params": {"inbox": {"kind": "instance", "instance": "inbox"}}}"#.to_owned(),
    );
    steps.push(format!(
        r#"{{"step": "expect_view", "view": "inbox.messaging.Pending", "expectation": {{"expect": "counts", "{held}": {bound}}}}}"#
    ));
    if posted {
        steps.push(format!(
            r#"{{"step": "expect_view", "view": "inbox.messaging.Pending", "expectation": {{"expect": "contains",
                "fields": {{"message_id": {{"kind": "literal", "value": "{MESSAGE}"}}, "from": {{"kind": "literal", "value": "{PLANNER}"}}, "state": {{"kind": "literal", "value": "Sent"}}}}}}}}"#
        ));
        steps.push(format!(
            r#"{{"step": "query_view", "view": "inbox.messaging.Outbox", "params": {{"sender": {{"kind": "literal", "value": "{PLANNER}"}}}}}}"#
        ));
        steps.push(format!(
            r#"{{"step": "expect_view", "view": "inbox.messaging.Outbox", "expectation": {{"expect": "contains",
                "fields": {{"message_id": {{"kind": "literal", "value": "{MESSAGE}"}}, "to": {{"kind": "literal", "value": "{WORKER}"}}}}}}}}"#
        ));
    }
    format!("[{}]", steps.join(","))
}

#[test]
fn a_scenario_runs_against_inbox_serve_over_http_and_passes() {
    let mut target =
        WireTarget::new(inbox(), work("passes")).expect("the target makes its operator key");
    let result = run_scenario(&scenario(&exchange("posted")), &mut target);
    assert_eq!(result.status, Status::Passed, "{result:?}");
    let pid = target.service_pid().expect("a service runs");
    assert!(alive(pid));
    drop(target);
    assert!(!alive(pid), "the service {pid} outlived its target");
}

#[test]
fn a_wrong_expectation_fails_on_the_wire() {
    let mut target = WireTarget::new(inbox(), work("wrong")).expect("the target");
    let steps = exchange("posted").replace(r#""at_least": 1"#, r#""at_least": 2"#);
    let result = run_scenario(&scenario(&steps), &mut target);
    assert_eq!(result.status, Status::Failed, "{result:?}");
    assert!(
        result
            .message
            .as_deref()
            .is_some_and(|message| message.contains("should hold at least 2 rows; it holds 1")),
        "{result:?}"
    );
}

#[test]
fn every_scenario_starts_a_fresh_service_and_stops_the_last() {
    let mut target = WireTarget::new(inbox(), work("fresh")).expect("the target");
    let first = run_scenario(&scenario(&exchange("posted")), &mut target);
    let pid = target.service_pid().expect("a service runs");
    let second = run_scenario(&scenario(&exchange("posted")), &mut target);
    assert_eq!(
        (first.status, second.status),
        (Status::Passed, Status::Passed),
        "a second run of the same scenario finds the first one's message: {second:?}"
    );
    assert!(!alive(pid), "the first scenario's service {pid} still runs");
}

#[test]
fn a_signature_the_service_refuses_is_the_outcome_signature_invalid() {
    let mut target = WireTarget::new(inbox(), work("bad-signature")).expect("the target");
    let result = run_scenario(&scenario(&exchange("signature-invalid")), &mut target);
    assert_eq!(result.status, Status::Passed, "{result:?}");
}

/// A stand-in for the service on 127.0.0.1:0 that answers every request with `status` and
/// `body`, and records each request line; answers its address and the record.
fn stand_in(status: u16, body: &'static str) -> (std::net::SocketAddr, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
    let address = listener.local_addr().expect("an address");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&seen);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
            let mut received = Vec::new();
            let mut buffer = [0_u8; 4096];
            let mut expected = None;
            loop {
                if expected.is_some_and(|total| received.len() >= total) {
                    break;
                }
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => received.extend_from_slice(&buffer[..read]),
                }
                let end = received.windows(4).position(|window| window == b"\r\n\r\n");
                if let (None, Some(end)) = (expected, end) {
                    let head = String::from_utf8_lossy(&received[..end]).to_lowercase();
                    let length: usize = head
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .and_then(|value| value.trim().parse().ok())
                        .unwrap_or(0);
                    expected = Some(end + 4 + length);
                }
            }
            let line = String::from_utf8_lossy(&received)
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned();
            record.lock().expect("the record").push(line);
            let answer = format!(
                "HTTP/1.1 {status} Stand-in\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(answer.as_bytes());
        }
    });
    (address, seen)
}

#[test]
fn a_service_that_answers_wrongly_fails_the_scenario() {
    let steps = format!(
        "[{}, {}]",
        open(WORKER),
        expect_outcome("OpenInbox", "opened")
    );
    for (status, body, said) in [
        (
            202,
            r#"{"outcome": "already-open", "published": []}"#,
            "should answer `opened`; it answered `already-open`",
        ),
        (
            401,
            r#"{"refused": "unauthenticated"}"#,
            r#"401 {"refused":"unauthenticated"}"#,
        ),
        (200, "not json", "200 not json"),
    ] {
        let (address, seen) = stand_in(status, body);
        let mut target = WireTarget::at(address, work("stand-in")).expect("the target");
        let result = run_scenario(&scenario(&steps), &mut target);
        assert_eq!(result.status, Status::Failed, "{result:?}");
        let message = result.message.unwrap_or_default();
        assert!(message.contains(said), "{message}");
        assert_eq!(
            *seen.lock().expect("the record"),
            ["POST /messaging/commands/open-inbox HTTP/1.1"]
        );
    }
    let (address, _) = stand_in(
        202,
        r#"{"outcome": "opened", "published": [{"event": "inbox.messaging.InboxOpened", "payload": {"address": "agent:box/other"}}]}"#,
    );
    let mut target = WireTarget::at(address, work("stand-in-event")).expect("the target");
    let claimed = format!(
        r#"[{}, {}, {{"step": "expect_event", "event": "inbox.messaging.InboxOpened", "payload": {{"address": "{WORKER}"}}}}]"#,
        open(WORKER),
        expect_outcome("OpenInbox", "opened")
    );
    let result = run_scenario(&scenario(&claimed), &mut target);
    assert_eq!(result.status, Status::Failed, "{result:?}");
}

#[test]
fn what_the_wire_cannot_witness_is_unsupported_and_nothing_is_sent() {
    let sweeper_expire = r#"{"step": "execute_command", "command": "inbox.messaging.Expire", "actor": "inbox.messaging.Sweeper",
        "input": {"message_id": {"kind": "literal", "value": "01JGT0DK000000000000000001"},
                  "now": {"kind": "literal", "value": "2020-01-01T00:00:00Z"},
                  "keep_until": {"kind": "literal", "value": "2020-01-02T00:00:00Z"}}}"#;
    let split_caller = open(WORKER).replacen(
        r#""inbox": "agent:box/worker""#,
        r#""inbox": "inbox-262144""#,
        1,
    );
    let signature_claim = r#"{"step": "expect_view", "view": "inbox.messaging.Pending", "expectation": {"expect": "excludes",
        "fields": {"signature": {"kind": "literal", "value": {"key_id": "signature.key_id", "value": "signature.value"}}}}}"#;
    let cases = [
        (format!("[{sweeper_expire}]"), "inbox.messaging.Sweeper"),
        (
            r#"[{"step": "read_as", "actor": "inbox.messaging.Sweeper"}]"#.to_owned(),
            "inbox.messaging.Sweeper",
        ),
        (
            r#"[{"step": "query_view", "view": "inbox.messaging.Due", "params": {"now": {"kind": "literal", "value": "2020-01-01T00:00:00Z"}}}]"#.to_owned(),
            "inbox.messaging.Sweeper",
        ),
        (
            r#"[{"step": "read_as", "actor": null}]"#.to_owned(),
            "no actor",
        ),
        (format!("[{split_caller}]"), "principal and inbox differ"),
        (
            format!(
                r#"[{}, {{"step": "query_view", "view": "inbox.messaging.Pending", "params": {{"inbox": {{"kind": "literal", "value": "{WORKER}"}}}}}}, {signature_claim}]"#,
                open(WORKER)
            ),
            "signature",
        ),
    ];
    let (address, seen) = stand_in(500, "{}");
    for (steps, named) in cases {
        let mut target = WireTarget::at(address, work("unsupported")).expect("the target");
        let result = run_scenario(&scenario(&steps), &mut target);
        assert_eq!(result.status, Status::Unsupported, "{steps}: {result:?}");
        let message = result.message.unwrap_or_default();
        assert!(message.contains(named), "{named}: {message}");
    }
    assert!(seen.lock().expect("the record").is_empty());
}

#[test]
fn a_service_that_does_not_start_is_an_error_and_leaves_no_process() {
    let steps = format!(
        "[{}, {}]",
        open(WORKER),
        expect_outcome("OpenInbox", "opened")
    );
    let missing = work("missing").join("no-such-inbox");
    let mut target = WireTarget::new(&missing, work("missing-run")).expect("the target");
    let result = run_scenario(&scenario(&steps), &mut target);
    assert_eq!(result.status, Status::Error, "{result:?}");
    assert!(
        result
            .message
            .as_deref()
            .is_some_and(|message| message.contains("did not start")),
        "{result:?}"
    );
    // A binary that starts, prints no address and exits: this package's own, which refuses
    // `serve` as wrong usage.
    let mut target = WireTarget::new(env!("CARGO_BIN_EXE_inbox-conformance"), work("no-address"))
        .expect("the target");
    let result = run_scenario(&scenario(&steps), &mut target);
    assert_eq!(result.status, Status::Error, "{result:?}");
    assert!(target.service_pid().is_none(), "a service is held");
}

#[test]
fn run_drives_the_service_and_writes_error_where_it_does_not_start() {
    let dir = work("cli");
    let suite = dir.join("suite.json");
    std::fs::write(&suite, suite_text(&exchange("posted"))).expect("the suite");
    let run = |binary: &Path, results: &Path| {
        Process::new(env!("CARGO_BIN_EXE_inbox-conformance"))
            .args(["run", "--target", "serve", "--inbox"])
            .arg(binary)
            .arg("--suite")
            .arg(&suite)
            .arg("--results")
            .arg(results)
            .output()
            .expect("the runner runs")
    };
    let passed = dir.join("passed.json");
    let output = run(&inbox(), &passed);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let written = std::fs::read_to_string(&passed).expect("the results");
    assert!(written.contains(r#""status":"passed""#), "{written}");
    assert!(
        dir.join("serve").join("keys.json").is_file(),
        "the key file is written under the results directory"
    );

    let errored = dir.join("errored.json");
    let output = run(&dir.join("no-such-inbox"), &errored);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let written = std::fs::read_to_string(&errored).expect("the results");
    assert!(written.contains(r#""status":"error""#), "{written}");

    let without = Process::new(env!("CARGO_BIN_EXE_inbox-conformance"))
        .args(["run", "--target", "serve", "--suite"])
        .arg(&suite)
        .arg("--results")
        .arg(&errored)
        .output()
        .expect("the runner runs");
    assert_eq!(without.status.code(), Some(2), "{without:?}");
}
