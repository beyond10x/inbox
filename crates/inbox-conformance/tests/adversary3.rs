//! Adversary cases of story:service-acceptance on the wire target, the gate's list and the
//! reading of `ess-conformance/65`. No key material is written into this file.

use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use inbox_conformance::gate::{self, Report};
use inbox_conformance::suite::{self, Scenario};
use inbox_conformance::{Status, WireTarget, run_scenario};

const PROBE_COVERAGE: &str = r#"{"authored": [], "authored_sources": {},
    "counts": {"authored": 0, "generated": 1, "outside": 0, "refused": 0},
    "generated": ["probe"], "knowledge": "complete_inventory", "outside": [], "refused": [],
    "selection": {"filter": {"kind": "all"}, "origins": "generated_and_authored", "scope": {"kind": "system"}}}"#;

const WORKER: &str = "agent:box/worker";

fn suite_text(steps: &str) -> String {
    format!(
        r#"{{"provenance": {{"suite_version": "ess-conformance/65", "scenario_initial_state": "empty"}},
            "coverage": {PROBE_COVERAGE},
            "scenarios": {{"probe": {{"purpose": "a probe", "source": [], "steps": {steps}}}}}}}"#
    )
}

fn scenario(steps: &str) -> Scenario {
    let mut read = suite::read(&suite_text(steps)).expect("the probe suite reads");
    read.scenarios.remove(0)
}

fn work(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("adversary3-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the work directory is created");
    path
}

fn open_steps() -> String {
    format!(
        r#"[{{"step": "execute_command", "command": "inbox.messaging.OpenInbox", "actor": "inbox.messaging.Client",
            "caller": {{"inbox": "{WORKER}", "principal": "{WORKER}"}},
            "input": {{"adapter": {{"kind": "literal", "value": "claude_code"}}, "address": {{"kind": "literal", "value": "{WORKER}"}}}}}},
           {{"step": "expect_outcome", "outcome": {{"command": "inbox.messaging.OpenInbox", "outcome": "opened"}}}},
           {{"step": "expect_event", "event": "inbox.messaging.InboxOpened", "payload": {{"address": "{WORKER}"}}}}]"#
    )
}

/// A listener on 127.0.0.1:0 that answers every request with `status` and `body`.
fn stand_in(status: u16, body: &'static str) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
    let address = listener.local_addr().expect("an address");
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
            let answer = format!(
                "HTTP/1.1 {status} Stand-in\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(answer.as_bytes());
        }
    });
    address
}

/// `generated/openapi` gives each declared outcome its status (`opened` is a `2xx`), and the
/// `inbox` client reads the status first: a `500` is "an answer that is neither success nor
/// refusal", exit 3 (AGENTS.md, the client's exit statuses). The wire target reads the outcome
/// from the body alone, so a service that answers `500 Internal Server Error` with a body naming
/// the expected outcome passes the scenario a client of the same service sees fail.
#[test]
fn a_declared_outcome_under_a_status_the_contract_does_not_give_it_fails_the_scenario() {
    let body = r#"{"outcome": "opened", "published": [{"event": "inbox.messaging.InboxOpened", "payload": {"address": "agent:box/worker"}}]}"#;
    for status in [500_u16, 409] {
        let mut target = WireTarget::at(stand_in(status, body), work(&format!("status-{status}")))
            .expect("the target");
        let result = run_scenario(&scenario(&open_steps()), &mut target);
        assert_eq!(
            result.status,
            Status::Failed,
            "an `opened` answered with HTTP {status} is recorded as {result:?}"
        );
    }
}

/// `unsupported.tsv` is `<id>\t<reason>` per line, "each with every reason of its scenario",
/// and the reason column is the only account of why the serve lane may report 77 scenarios it
/// did not run. The gate holds the ids alone: a list whose every reason is false passes.
#[test]
fn the_gate_refuses_a_listed_reason_that_is_not_the_reason_of_the_run() {
    let listed = include_str!("../unsupported.tsv");
    let ids: Vec<String> = listed
        .lines()
        .map(|line| line.split_once('\t').expect("a tab").0.to_owned())
        .collect();
    let falsified: String = ids
        .iter()
        .map(|id| format!("{id}\tthe service was not running\n"))
        .collect();
    let report = Report {
        total: 99,
        passed: 22,
        unsupported: ids.clone(),
        ..Report::default()
    };
    let verdict = gate::verdict(
        &report,
        &gate::read_listed(&falsified).expect("the list reads"),
    );
    assert!(
        !verdict.passes(),
        "the gate passes {} listed scenarios whose every reason is `the service was not running`",
        ids.len()
    );
}

/// The crate says a dotted row path is compared as the ESS interpreted target compares it.
/// Measured on ESS 0.96.0 (`ess verify conform run --target interpreted`, the suite of `task
/// coverage` with one claim added to an authored scenario): `{"signature..key_id": null}` and
/// `{"signature.": null}` pass there (the path reaches nothing, which holds `null`). The runner
/// makes the scenario unsupported instead.
#[test]
fn ess_reads_a_row_path_with_an_empty_member_and_holds_null_there() {
    let steps = r#"[{"step": "query_view", "view": "inbox.messaging.Inboxes"},
            {"step": "expect_view", "view": "inbox.messaging.Inboxes", "expectation": {"expect": "excludes",
              "fields": {"signature..key_id": {"kind": "literal", "value": "x"}}}}]"#;
    let read = scenario(steps);
    assert!(
        read.steps.is_ok(),
        "the runner does not interpret what ESS interprets: {:?}",
        read.steps.err()
    );
}

/// The crate says the runner reads `coverage` as ESS's own reader does on 0.96.0. Measured on
/// ESS 0.96.0: the suite of `task coverage` with one generated and one authored id exchanged
/// between `authored` and `generated` (each still named once, the counts unchanged) is refused,
/// "selected origin lists must be sorted and distinct"; the runner reads it and runs all 93.
#[test]
fn ess_refuses_coverage_lists_out_of_order() {
    let text = suite_text("[]").replace(
        r#""generated": ["probe"]"#,
        r#""generated": ["probe", "a-probe"]"#,
    );
    let text = text
        .replace(r#""generated": 1,"#, r#""generated": 2,"#)
        .replace(
            r#""probe": {"purpose": "a probe", "source": [], "steps": []}"#,
            r#""probe": {"purpose": "a probe", "source": [], "steps": []}, "a-probe": {"purpose": "a probe", "source": [], "steps": []}"#,
        );
    assert!(
        suite::read(&text).is_err(),
        "the runner reads a coverage block ESS refuses"
    );
}
