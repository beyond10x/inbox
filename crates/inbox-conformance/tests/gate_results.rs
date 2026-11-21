//! The gate binary holds the results document `--results` names to the report written from it:
//! results that do not say what the report says fail the gate (exit 1).

use std::path::PathBuf;
use std::process::Command as Process;

use inbox_conformance::{ScenarioResult, Status, results};

const SUITE: &str = r#"{"provenance": {"suite_version": "ess-conformance/65", "scenario_initial_state": "empty"},
    "coverage": {"authored": [], "authored_sources": {},
      "counts": {"authored": 0, "generated": 2, "outside": 0, "refused": 0},
      "generated": ["a", "b"], "knowledge": "complete_inventory", "outside": [], "refused": [],
      "selection": {"filter": {"kind": "all"}, "origins": "generated_and_authored", "scope": {"kind": "system"}}},
    "scenarios": {"a": {"purpose": "p", "source": [], "steps": []},
                  "b": {"purpose": "p", "source": [], "steps": []}}}"#;

/// Runs `inbox-conformance gate` on [`SUITE`], a report/2 that passed `a` and `b` at the instant
/// 1700, and results with these statuses; the exit code.
fn gate(name: &str, statuses: &[(&str, Status)]) -> Option<i32> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("gate-results-{name}"));
    std::fs::create_dir_all(&dir).expect("a directory");
    let suite = dir.join("suite.json");
    let report = dir.join("report.json");
    let results_path = dir.join("results.json");
    let listed = dir.join("unsupported.tsv");
    std::fs::write(&suite, SUITE).expect("the suite");
    std::fs::write(&listed, "").expect("the list");
    let digest = results::suite_digest(SUITE.as_bytes());
    std::fs::write(
        &report,
        format!(
            r#"{{"format": "ess-conformance-report/2", "conformance_status": "passed", "completed_at": 1700,
                "suite": {{"digest": "{digest}", "version": "ess-conformance/65"}},
                "counts": {{"total": 2, "passed": 2, "failed": 0, "error": 0, "skipped": 0, "unsupported": 0}},
                "outcomes": {{"passed": ["a", "b"], "failed": [], "error": [], "skipped": [], "unsupported": []}}}}"#
        ),
    )
    .expect("the report");
    let outcomes: Vec<ScenarioResult> = statuses
        .iter()
        .map(|(id, status)| ScenarioResult {
            id: (*id).to_owned(),
            status: *status,
            message: (*status == Status::Unsupported).then(|| "why".to_owned()),
        })
        .collect();
    std::fs::write(&results_path, results::document(&digest, 1700, &outcomes))
        .expect("the results");
    Process::new(env!("CARGO_BIN_EXE_inbox-conformance"))
        .arg("gate")
        .arg("--suite")
        .arg(&suite)
        .arg("--report")
        .arg(&report)
        .arg("--results")
        .arg(&results_path)
        .arg("--unsupported")
        .arg(&listed)
        .status()
        .expect("the gate runs")
        .code()
}

#[test]
fn results_that_do_not_say_what_the_report_says_fail_the_gate() {
    assert_eq!(
        gate("own", &[("a", Status::Passed), ("b", Status::Passed)]),
        Some(0),
        "the control passes"
    );
    assert_eq!(
        gate(
            "all-failed",
            &[("a", Status::Failed), ("b", Status::Failed)]
        ),
        Some(1),
        "results whose passed scenarios are all `failed` pass the gate"
    );
    assert_eq!(
        gate(
            "one-unsupported",
            &[("a", Status::Passed), ("b", Status::Unsupported)]
        ),
        Some(1),
        "results with a passed scenario marked `unsupported` pass the gate"
    );
}
