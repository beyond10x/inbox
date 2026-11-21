//! Adversary cases of the second pass on the conformance runner and the gate.
//!
//! The `ess_*` case holds the runner to what the ESS 0.96.0 runner decides for the same step
//! (`crates/verify/ess-conformance/src/runner.rs` of the `ess` release `ess-inputs.yaml` pins). The
//! `gate_*` cases run the gate binary on a suite and a report/2 whose `suite.digest` is that
//! suite's, whose counts agree with its lists and add up to its total, and which still is not the
//! report of a run of that suite.

use std::path::PathBuf;
use std::process::Command as Process;

use inbox::server::json::{self, Value};
use inbox_conformance::suite::{self, Scenario};
use inbox_conformance::{Command, Executed, Read, Reader, Status, Target, results, run_scenario};

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

struct Reads(Vec<Read>);

impl Target for Reads {
    fn reset(&mut self) {}

    fn execute(&mut self, _command: &Command<'_>) -> Executed {
        unreachable!("the probe sends no command")
    }

    fn query(&mut self, _view: &str, _reader: &Reader, _params: Value) -> Read {
        self.0.remove(0)
    }
}

fn rows(text: &str) -> Read {
    match json::parse_written(text).expect("JSON") {
        Value::Array(rows) => Read::Rows(rows),
        _ => unreachable!(),
    }
}

// ---- the runner against what the ESS runner decides ----------------------------------------

/// ESS `satisfies` (runner.rs): "Every row satisfies the predicate, and there is at least one";
/// an empty view answers `Unsatisfied("the view holds no rows")`. The crate doc says the runner
/// compares `satisfies` as the ESS 0.96.0 runner does. 30 steps of the suite are `satisfies`, each
/// the only expectation on its read.
#[test]
fn ess_satisfies_needs_at_least_one_row() {
    let steps = r#"[{"step": "query_view", "view": "inbox.messaging.Due"},
        {"step": "expect_view", "view": "inbox.messaging.Due", "expectation": {"expect": "satisfies", "predicate": "message_id.count == 3"}}]"#;
    let held = run_scenario(
        &scenario(steps),
        &mut Reads(vec![rows(r#"[{"message_id": "abc"}]"#)]),
    );
    assert_eq!(held.status, Status::Passed, "{held:?}");
    let empty = run_scenario(&scenario(steps), &mut Reads(vec![rows("[]")]));
    assert_eq!(empty.status, Status::Failed, "{empty:?}");
}

// ---- the suite reader -----------------------------------------------------------------------

/// The crate doc: "Nothing the suite states is silently ignored." The JSON reader keeps a member
/// stated twice (`Value::Object` is a list of pairs), and the reader takes the first: the second
/// `expectation` of this step, an unsatisfiable `counts`, is never read. ESS reads a suite with
/// serde, which refuses a field stated twice.
#[test]
fn a_key_stated_twice_in_a_step_is_not_ignored() {
    let text = format!(
        r#"{{"provenance": {{"suite_version": "ess-conformance/65", "scenario_initial_state": "empty"}},
            "coverage": {PROBE_COVERAGE},
            "events": [], "scenarios": {{"probe": {{"purpose": "p", "source": [], "steps": [
              {{"step": "query_view", "view": "inbox.messaging.Due"}},
              {{"step": "expect_view", "view": "inbox.messaging.Due",
               "expectation": {{"expect": "counts", "at_least": 0}},
               "expectation": {{"expect": "counts", "at_least": 7, "at_most": 0}}}}]}}}}}}"#
    );
    match suite::read(&text) {
        Err(refused) => assert!(refused.contains("`expectation` twice"), "{refused}"),
        Ok(read) => assert!(
            read.scenarios[0].unsupported().is_some(),
            "a step stating `expectation` twice is admitted and its second expectation ignored: {:?}",
            read.scenarios[0].steps
        ),
    }
}

// ---- the gate with the suite as input -------------------------------------------------------

const SUITE: &str = r#"{"provenance": {"suite_version": "ess-conformance/65", "scenario_initial_state": "empty"},
    "coverage": {"authored": [], "authored_sources": {},
      "counts": {"authored": 0, "generated": 2, "outside": 0, "refused": 0},
      "generated": ["a", "b"], "knowledge": "complete_inventory", "outside": [], "refused": [],
      "selection": {"filter": {"kind": "all"}, "origins": "generated_and_authored", "scope": {"kind": "system"}}},
    "scenarios": {"a": {"purpose": "p", "source": [], "steps": []},
                                "b": {"purpose": "p", "source": [], "steps": []}}}"#;

/// Runs `inbox-conformance gate` on [`SUITE`] and a report/2 with its digest and these passed ids;
/// the exit code.
fn gate(name: &str, passed: &[&str]) -> Option<i32> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("adversary2-{name}"));
    std::fs::create_dir_all(&dir).expect("a directory for the probe");
    let suite = dir.join("suite.json");
    let report = dir.join("report.json");
    let listed = dir.join("unsupported.tsv");
    std::fs::write(&suite, SUITE).expect("the suite");
    std::fs::write(&listed, "").expect("the list");
    let ids = passed
        .iter()
        .map(|id| format!("{id:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    std::fs::write(
        &report,
        format!(
            r#"{{"format": "ess-conformance-report/2", "conformance_status": "passed",
                "suite": {{"digest": "{}", "version": "ess-conformance/65"}},
                "counts": {{"total": {n}, "passed": {n}, "failed": 0, "error": 0, "skipped": 0, "unsupported": 0}},
                "outcomes": {{"passed": [{ids}], "failed": [], "error": [], "skipped": [], "unsupported": []}}}}"#,
            results::suite_digest(SUITE.as_bytes()),
            n = passed.len()
        ),
    )
    .expect("the report");
    Process::new(env!("CARGO_BIN_EXE_inbox-conformance"))
        .arg("gate")
        .arg("--suite")
        .arg(&suite)
        .arg("--report")
        .arg(&report)
        .arg("--unsupported")
        .arg(&listed)
        .status()
        .expect("the gate runs")
        .code()
}

/// The correction: the gate takes `--suite` and holds the report "to the suite it was made for".
/// A report with that suite's digest that accounts for 1 of its 2 scenarios, or for one it does
/// not have, is not the report of a run of it.
#[test]
fn gate_refuses_a_report_that_does_not_account_for_the_scenarios_of_its_suite() {
    assert_eq!(gate("whole", &["a", "b"]), Some(0), "the control passes");
    assert_eq!(
        gate("one-of-two", &["a"]),
        Some(1),
        "a report of 1 scenario passes the gate on a suite of 2"
    );
    assert_eq!(
        gate("foreign", &["a", "z"]),
        Some(1),
        "a report naming `z`, no scenario of the suite, passes the gate"
    );
}

#[test]
fn gate_refuses_a_report_that_lists_one_scenario_twice() {
    assert_eq!(
        gate("twice", &["a", "a"]),
        Some(1),
        "a report passing `a` twice and never naming `b` passes the gate"
    );
}
