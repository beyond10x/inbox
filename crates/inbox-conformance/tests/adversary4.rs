//! Adversary cases of the second pass on story:service-acceptance: suites ESS 0.96.0 reads that
//! the runner refuses. Each was measured with `ess verify conform run --target interpreted` on
//! the suite `task coverage` writes, altered the same way: ESS ran all 93 scenarios, exit 0.

use inbox_conformance::suite;

/// A coverage suite of one authored scenario, `probe`, with one refusal; `refused` is the entry.
fn suite_with_refusal(refused: &str, events: &str) -> String {
    format!(
        r#"{{"provenance": {{"suite_version": "ess-conformance/65", "scenario_initial_state": "empty"}},
            "coverage": {{"authored": ["probe"],
              "authored_sources": {{"scenarios/probe.yaml": {{"digest": "sha256:00", "disposition": "accepted", "scenario": "probe"}}}},
              "counts": {{"authored": 1, "generated": 0, "outside": 0, "refused": 1}},
              "generated": [], "knowledge": "complete_inventory", "outside": [],
              "refused": [{refused}],
              "selection": {{"filter": {{"kind": "all"}}, "origins": "generated_and_authored", "scope": {{"kind": "system"}}}}}},
            {events}
            "scenarios": {{"probe": {{"purpose": "a probe", "source": [], "steps": []}}}}}}"#
    )
}

const ENTRY_WITHOUT_COVERED_BY: &str = r#"{"code": "ESS-SYNTH-004", "effect": "candidate_not_emitted",
    "message": "m", "needs": [], "origin": "generated", "retained": null,
    "scenario": "x/refused", "scope": "in_scope", "source": null, "subject": {}}"#;

/// `suite::Coverage` says the runner refuses an entry of `refused` that "lacks one of the 11
/// members ESS 0.96.0 writes there", and that ESS refuses each altered suite the rules stand for.
/// Measured: ESS refuses a missing `code`, `effect`, `message`, `needs`, `origin`, `retained`,
/// `scenario`, `scope`, `source` or `subject` ("MissingField: required field absent"), and reads
/// an entry without `covered_by`, in one entry or in all 89. The runner refuses it.
#[test]
fn ess_reads_a_refusal_without_covered_by() {
    let text = suite_with_refusal(ENTRY_WITHOUT_COVERED_BY, "");
    let read = suite::read(&text);
    assert!(
        read.is_ok(),
        "the runner refuses a suite ESS reads: {:?}",
        read.err()
    );
}

/// ESS writes the `events` catalog "only where a scenario refers to it": with no `absent_events`
/// and no `unpublished_events` in any scenario, the suite without `events` is read and runs, and
/// the same suite with the catalog kept is refused ("the suite event catalog is written only
/// where a scenario refers to it"). The runner requires `events` in every suite.
#[test]
fn ess_reads_a_suite_whose_scenarios_claim_no_absent_events_without_a_catalog() {
    let covered = ENTRY_WITHOUT_COVERED_BY.replace(r#""code""#, r#""covered_by": "probe", "code""#);
    let text = suite_with_refusal(&covered, "");
    let read = suite::read(&text);
    assert!(
        read.is_ok(),
        "the runner refuses a suite ESS reads: {:?}",
        read.err()
    );
}
