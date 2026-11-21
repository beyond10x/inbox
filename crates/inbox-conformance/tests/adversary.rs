//! Adversary cases for the conformance runner and the gate.
//!
//! The `ess_*` cases hold the runner to what the ESS 0.96.0 runner decides for the same step
//! (`crates/verify/ess-conformance/src/runner.rs` and `value_invariant.rs` of the `ess` release
//! `ess-inputs.yaml` pins); the `catches_*` cases are one per comparison, each failing when that
//! comparison is made to always hold.

use inbox::server::json::{self, Value};
use inbox_conformance::run_scenario;
use inbox_conformance::suite::{self, Scenario};
use inbox_conformance::{Answer, Command, Event, Executed, Read, Reader, Status, Target, gate};

/// The `coverage` block of a coverage suite whose one scenario, `probe`, is generated.
const PROBE_COVERAGE: &str = r#"{"authored": [], "authored_sources": {},
    "counts": {"authored": 0, "generated": 1, "outside": 0, "refused": 0},
    "generated": ["probe"], "knowledge": "complete_inventory", "outside": [], "refused": [],
    "selection": {"filter": {"kind": "all"}, "origins": "generated_and_authored", "scope": {"kind": "system"}}}"#;

fn scenario(steps: &str) -> Scenario {
    let text = format!(
        r#"{{"provenance": {{"suite_version": "ess-conformance/65", "scenario_initial_state": "empty"}},
            "coverage": {PROBE_COVERAGE},
            "scenarios": {{"probe": {{"purpose": "a probe", "source": [], "steps": {steps}}}}}}}"#
    );
    let mut read = suite::read(&text).expect("the probe suite reads");
    read.scenarios.remove(0)
}

fn parse(text: &str) -> Value {
    json::parse_written(text).expect("JSON")
}

fn rows(text: &str) -> Read {
    match parse(text) {
        Value::Array(rows) => Read::Rows(rows),
        _ => unreachable!(),
    }
}

#[derive(Default)]
struct Scripted {
    answers: Vec<Executed>,
    reads: Vec<Read>,
}

impl Target for Scripted {
    fn reset(&mut self) {}

    fn execute(&mut self, _command: &Command<'_>) -> Executed {
        self.answers.remove(0)
    }

    fn query(&mut self, _view: &str, _reader: &Reader, _params: Value) -> Read {
        self.reads.remove(0)
    }
}

fn executed(answer: Answer, events: &[(&str, &str)]) -> Executed {
    Executed {
        answer,
        events: events
            .iter()
            .map(|(name, payload)| Event {
                name: (*name).to_owned(),
                payload: parse(payload),
            })
            .collect(),
    }
}

fn outcome(name: &str, error: Option<&str>, events: &[(&str, &str)]) -> Executed {
    executed(
        Answer::Outcome {
            outcome: name.to_owned(),
            error: error.map(str::to_owned),
        },
        events,
    )
}

fn run(steps: &str, answers: Vec<Executed>, reads: Vec<Read>) -> (Status, Option<String>) {
    let mut target = Scripted { answers, reads };
    let result = run_scenario(&scenario(steps), &mut target);
    (result.status, result.message)
}

const OPEN: &str = r#"{"step": "execute_command", "command": "inbox.messaging.OpenInbox",
    "actor": "inbox.messaging.Client", "caller": {"inbox": "agent:box/worker", "principal": "agent:box/worker"},
    "input": {"adapter": {"kind": "literal", "value": "claude_code"}, "address": {"kind": "literal", "value": "agent:box/worker"}}}"#;

const QUERY: &str = r#"{"step": "query_view", "view": "inbox.messaging.Inboxes"}"#;

fn expect_view(expectation: &str) -> String {
    format!(
        r#"{{"step": "expect_view", "view": "inbox.messaging.Inboxes", "expectation": {expectation}}}"#
    )
}

// ---- the runner against what the ESS runner decides ----------------------------------------

/// ESS `snapshot_subject`: "exactly one subject row"; a snapshot that selects no row fails.
#[test]
fn ess_a_snapshot_that_finds_no_row_does_not_hold_the_subject_unchanged() {
    let steps = format!(
        r#"[{QUERY},
            {{"step": "snapshot_subject", "view": "inbox.messaging.Inboxes", "subject": {{"address": {{"kind": "literal", "value": "agent:box/worker"}}}}}},
            {QUERY},
            {{"step": "expect_subject_unchanged", "view": "inbox.messaging.Inboxes"}}]"#
    );
    let (status, message) = run(&steps, Vec::new(), vec![rows("[]"), rows("[]")]);
    assert_eq!(status, Status::Failed, "{message:?}");
}

/// ESS `fields_match`: a plain top-level field name keeps exact comparison, so `contains` with
/// `null` needs the row to carry the member as `null`; only a dotted leaf path reads absent as
/// `null`.
#[test]
#[ignore = "declined, beyond10x/ess#799"]
fn ess_contains_null_at_the_top_level_needs_the_member_published() {
    let steps = format!(
        r#"[{QUERY}, {}]"#,
        expect_view(
            r#"{"expect": "contains", "fields": {"address": {"kind": "literal", "value": "a"}, "reply": {"kind": "literal", "value": null}}}"#
        )
    );
    let (status, message) = run(&steps, Vec::new(), vec![rows(r#"[{"address": "a"}]"#)]);
    assert_eq!(status, Status::Failed, "{message:?}");
}

// ---- the suite reader -----------------------------------------------------------------------

/// "A key or construct the runner does not know makes the scenario unsupported, never passed":
/// the top level of the suite is read by name and anything else on it is ignored.
#[test]
fn an_unknown_top_level_key_of_the_suite_is_not_ignored() {
    let text = format!(
        r#"{{"provenance": {{"suite_version": "ess-conformance/65", "scenario_initial_state": "empty"}},
            "coverage": {PROBE_COVERAGE},
            "events": [], "fixtures": [{{"entity": "inbox.messaging.Inbox", "rows": []}}],
            "scenarios": {{"probe": {{"purpose": "p", "source": [], "steps": []}}}}}}"#
    );
    match suite::read(&text) {
        Err(refused) => assert!(refused.contains("`fixtures`"), "{refused}"),
        Ok(read) => assert!(
            read.scenarios
                .iter()
                .all(|scenario| scenario.unsupported().is_some()),
            "a suite with the unknown top-level key `fixtures` is admitted and its scenarios run"
        ),
    }
}

#[test]
fn an_unknown_provenance_key_is_not_ignored() {
    let text = format!(
        r#"{{"provenance": {{"suite_version": "ess-conformance/65", "scenario_initial_state": "empty",
            "scenario_clock": "held"}},
            "coverage": {PROBE_COVERAGE},
            "events": [], "scenarios": {{"probe": {{"purpose": "p", "source": [], "steps": []}}}}}}"#
    );
    match suite::read(&text) {
        Err(refused) => assert!(refused.contains("`scenario_clock`"), "{refused}"),
        Ok(read) => assert!(
            read.scenarios
                .iter()
                .all(|scenario| scenario.unsupported().is_some()),
            "a suite with the unknown provenance key `scenario_clock` is admitted and its scenarios run"
        ),
    }
}

// ---- the gate -------------------------------------------------------------------------------

/// The unit: the gate fails "on counts that disagree with the id lists".
#[test]
fn the_report_passed_count_must_agree_with_the_passed_list() {
    let text = r#"{"format": "ess-conformance-report/2",
        "counts": {"total": 3, "passed": 3, "failed": 0, "error": 0, "skipped": 0, "unsupported": 0},
        "outcomes": {"passed": ["a"], "failed": [], "error": [], "skipped": [], "unsupported": []}}"#;
    assert!(
        gate::read_report(text).is_err(),
        "counts.passed 3 against outcomes.passed of 1 is read"
    );
}

#[test]
fn the_report_total_must_be_the_sum_of_its_outcomes() {
    let text = r#"{"format": "ess-conformance-report/2",
        "counts": {"total": 92, "passed": 1, "failed": 0, "error": 0, "skipped": 0, "unsupported": 0},
        "outcomes": {"passed": ["a"], "failed": [], "error": [], "skipped": [], "unsupported": []}}"#;
    let refused = match gate::read_report(text) {
        Err(_) => true,
        Ok(report) => !gate::verdict(&report, &gate::Listed::new()).passes(),
    };
    assert!(
        refused,
        "a report of 92 scenarios with 1 accounted for passes the gate"
    );
}

fn report(passed: u64, skipped: &[&str], unsupported: &[&str]) -> gate::Report {
    let ids = |ids: &[&str]| ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>();
    gate::Report {
        total: passed + (skipped.len() + unsupported.len()) as u64,
        passed,
        failed: Vec::new(),
        error: Vec::new(),
        skipped: ids(skipped),
        unsupported: ids(unsupported),
        reasons: unsupported
            .iter()
            .map(|id| ((*id).to_owned(), "the step kind `deliver_event`".to_owned()))
            .collect(),
    }
}

#[test]
fn catches_a_skipped_scenario() {
    assert!(!gate::verdict(&report(3, &["s"], &[]), &gate::Listed::new()).passes());
}

#[test]
fn catches_a_run_that_passed_nothing_where_every_unsupported_one_is_listed() {
    let listed = gate::read_listed("u\tthe step kind `deliver_event`\n").unwrap();
    assert!(gate::verdict(&report(1, &[], &["u"]), &listed).passes());
    assert!(!gate::verdict(&report(0, &[], &["u"]), &listed).passes());
}

// ---- one case per comparison ----------------------------------------------------------------

#[test]
fn catches_a_wrong_error_name() {
    let steps =
        format!(r#"[{OPEN}, {{"step": "expect_error", "error": "inbox.messaging.Duplicate"}}]"#);
    let (status, _) = run(
        &steps,
        vec![outcome("x", Some("inbox.messaging.UnknownInbox"), &[])],
        Vec::new(),
    );
    assert_eq!(status, Status::Failed);
    let (status, _) = run(&steps, vec![outcome("x", None, &[])], Vec::new());
    assert_eq!(status, Status::Failed);
}

#[test]
fn catches_an_event_of_another_name() {
    let steps = format!(
        r#"[{OPEN}, {{"step": "expect_event", "event": "inbox.messaging.InboxOpened", "payload": {{"address": "a"}}}}]"#
    );
    let (status, _) = run(
        &steps,
        vec![outcome(
            "opened",
            None,
            &[("inbox.messaging.InboxClosed", r#"{"address": "a"}"#)],
        )],
        Vec::new(),
    );
    assert_eq!(status, Status::Failed);
}

#[test]
fn catches_a_payload_outside_its_shape() {
    let steps = format!(
        r#"[{OPEN}, {{"step": "expect_event", "event": "inbox.messaging.InboxOpened", "payload": {{"kind": "loud"}},
            "shape": {{"kind": {{"holds": "enum", "variants": ["note", "request"]}}}}}}]"#
    );
    let (status, message) = run(
        &steps,
        vec![outcome(
            "opened",
            None,
            &[("inbox.messaging.InboxOpened", r#"{"kind": "loud"}"#)],
        )],
        Vec::new(),
    );
    assert_eq!(status, Status::Failed, "{message:?}");
}

#[test]
fn catches_an_excluded_row() {
    let steps = format!(
        r#"[{QUERY}, {}]"#,
        expect_view(
            r#"{"expect": "excludes", "fields": {"address": {"kind": "literal", "value": "a"}}}"#
        )
    );
    let (status, _) = run(&steps, Vec::new(), vec![rows(r#"[{"address": "a"}]"#)]);
    assert_eq!(status, Status::Failed);
    let (status, _) = run(&steps, Vec::new(), vec![rows(r#"[{"address": "b"}]"#)]);
    assert_eq!(status, Status::Passed);
}

#[test]
fn catches_a_row_count_outside_its_bounds() {
    let steps = format!(
        r#"[{QUERY}, {}]"#,
        expect_view(r#"{"expect": "counts", "at_least": 1, "at_most": 1}"#)
    );
    for (view_rows, want) in [
        ("[]", Status::Failed),
        (r#"[{"a": 1}, {"a": 2}]"#, Status::Failed),
        (r#"[{"a": 1}]"#, Status::Passed),
    ] {
        let (status, message) = run(&steps, Vec::new(), vec![rows(view_rows)]);
        assert_eq!(status, want, "{view_rows}: {message:?}");
    }
}

#[test]
fn catches_a_subject_row_that_changed() {
    let steps = format!(
        r#"[{QUERY},
            {{"step": "snapshot_subject", "view": "inbox.messaging.Inboxes", "subject": {{"address": {{"kind": "literal", "value": "a"}}}}}},
            {QUERY},
            {{"step": "expect_subject_unchanged", "view": "inbox.messaging.Inboxes"}}]"#
    );
    let before = r#"[{"address": "a", "state": "Open"}]"#;
    let (status, _) = run(
        &steps,
        Vec::new(),
        vec![
            rows(before),
            rows(r#"[{"address": "a", "state": "Closed"}]"#),
        ],
    );
    assert_eq!(status, Status::Failed);
    let (status, _) = run(&steps, Vec::new(), vec![rows(before), rows("[]")]);
    assert_eq!(status, Status::Failed);
}

#[test]
fn catches_a_complete_subject_row_that_changed() {
    let steps = format!(
        r#"[{QUERY},
            {{"step": "snapshot_complete_subject", "view": "inbox.messaging.Inboxes", "subject": {{"address": {{"kind": "literal", "value": "a"}}}},
              "shape": {{"identity_field": "address", "fields": [{{"name": "address", "type": "inbox.messaging.Address"}}, {{"name": "state", "type": "inbox.messaging.InboxState"}}], "declarations": {{}}}}}},
            {QUERY},
            {{"step": "expect_complete_subject_unchanged", "view": "inbox.messaging.Inboxes"}}]"#
    );
    let (status, message) = run(
        &steps,
        Vec::new(),
        vec![
            rows(r#"[{"address": "a", "state": "Open"}]"#),
            rows(r#"[{"address": "a", "state": "Closed"}]"#),
        ],
    );
    assert_eq!(status, Status::Failed, "{message:?}");
}

#[test]
fn catches_a_command_that_ran_where_it_should_be_refused_as_not_granted() {
    let steps =
        format!(r#"[{OPEN}, {{"step": "expect_not_granted", "actor": "inbox.messaging.Client"}}]"#);
    let (status, _) = run(&steps, vec![outcome("opened", None, &[])], Vec::new());
    assert_eq!(status, Status::Failed);
    let (status, _) = run(
        &steps,
        vec![executed(
            Answer::NotGranted(Some("inbox.messaging.Sweeper".to_owned())),
            &[],
        )],
        Vec::new(),
    );
    assert_eq!(status, Status::Failed);
    let (status, _) = run(
        &steps,
        vec![executed(
            Answer::NotGranted(Some("inbox.messaging.Client".to_owned())),
            &[],
        )],
        Vec::new(),
    );
    assert_eq!(status, Status::Passed);
}

#[test]
fn catches_a_read_served_where_it_should_be_refused() {
    let steps = r#"[{"step": "read_as", "actor": "inbox.messaging.Client"},
        {"step": "query_view", "view": "inbox.messaging.Due"},
        {"step": "expect_not_granted", "actor": "inbox.messaging.Client"}]"#;
    let (status, _) = run(steps, Vec::new(), vec![rows("[]")]);
    assert_eq!(status, Status::Failed);
}

#[test]
fn catches_events_where_none_are_expected() {
    let steps = format!(r#"[{OPEN}, {{"step": "expect_no_events"}}]"#);
    let (status, _) = run(
        &steps,
        vec![outcome(
            "opened",
            None,
            &[("inbox.messaging.InboxOpened", r#"{"address": "a"}"#)],
        )],
        Vec::new(),
    );
    assert_eq!(status, Status::Failed);
}

#[test]
fn catches_an_event_that_should_be_absent() {
    let steps = format!(
        r#"[{OPEN}, {{"step": "expect_no_event", "event": "inbox.messaging.InboxOpened"}}]"#
    );
    let (status, _) = run(
        &steps,
        vec![outcome(
            "opened",
            None,
            &[("inbox.messaging.InboxOpened", r#"{"address": "a"}"#)],
        )],
        Vec::new(),
    );
    assert_eq!(status, Status::Failed);
}
