//! The pieces of the runner that can be wrong without the suite noticing: what a step compares,
//! when a scenario is unsupported, the target over the core, the results document and the gate.

use inbox::server::json::{self, Value};
use inbox_conformance::suite::{self, Scenario};
use inbox_conformance::{
    Answer, Command, CoreTarget, Event, Executed, Read, Reader, Status, StepContext, Target, gate,
    results, run_scenario,
};

/// A suite of one scenario with these steps and these extra scenario members.
fn scenario(steps: &str, extra: &str) -> Scenario {
    let text = format!(
        r#"{{"provenance": {{"suite_version": "ess-conformance/52", "scenario_initial_state": "empty"}},
            "events": ["inbox.messaging.InboxOpened", "inbox.messaging.MessagePosted"],
            "scenarios": {{"probe": {{"purpose": "a probe", "source": [], "steps": {steps}{extra}}}}}}}"#
    );
    let mut read = suite::read(&text).expect("the probe suite reads");
    assert_eq!(read.scenarios.len(), 1);
    read.scenarios.remove(0)
}

fn parse(text: &str) -> Value {
    json::parse_written(text).expect("JSON")
}

/// A target that answers from a script, and records what it was sent.
#[derive(Default)]
struct Scripted {
    answers: Vec<Executed>,
    reads: Vec<Read>,
    sent: Vec<(String, Option<String>, Value)>,
    queried: Vec<(String, Reader)>,
}

impl Target for Scripted {
    fn reset(&mut self) {}

    fn execute(&mut self, command: &Command<'_>) -> Executed {
        self.sent.push((
            command.name.to_owned(),
            command.expected.map(str::to_owned),
            command.input.clone(),
        ));
        self.answers.remove(0)
    }

    fn query(&mut self, view: &str, reader: &Reader, _params: Value) -> Read {
        self.queried.push((view.to_owned(), reader.clone()));
        self.reads.remove(0)
    }
}

/// A target that echoes the outcome the step expects and does nothing else: the target a runner
/// that compares only outcomes would pass.
struct Echo;

impl Target for Echo {
    fn reset(&mut self) {}

    fn execute(&mut self, command: &Command<'_>) -> Executed {
        Executed {
            answer: Answer::Outcome {
                outcome: command.expected.unwrap_or("posted").to_owned(),
                error: None,
            },
            events: Vec::new(),
        }
    }

    fn query(&mut self, _view: &str, _reader: &Reader, _params: Value) -> Read {
        Read::Rows(Vec::new())
    }
}

fn outcome(outcome: &str, error: Option<&str>, events: &[(&str, &str)]) -> Executed {
    Executed {
        answer: Answer::Outcome {
            outcome: outcome.to_owned(),
            error: error.map(str::to_owned),
        },
        events: events
            .iter()
            .map(|(name, payload)| Event {
                name: (*name).to_owned(),
                payload: parse(payload),
            })
            .collect(),
    }
}

const OPEN: &str = r#"{"step": "execute_command", "command": "inbox.messaging.OpenInbox",
    "actor": "inbox.messaging.Client", "caller": {"inbox": "agent:box/worker", "principal": "agent:box/worker"},
    "input": {"adapter": {"kind": "literal", "value": "claude_code"}, "address": {"kind": "literal", "value": "agent:box/worker"}}}"#;

fn post(id: &str, key: &str) -> String {
    format!(
        r#"{{"step": "execute_command", "command": "inbox.messaging.PostMessage", "actor": "inbox.messaging.Client",
        "caller": {{"inbox": "agent:ws/planner", "principal": "agent:ws/planner"}},
        "input": {{"body": {{"kind": "literal", "value": "Build 512 is green."}},
          "created": {{"kind": "literal", "value": "2026-01-05T09:00:00Z"}},
          "dedup_key": {{"kind": "literal", "value": "{key}"}},
          "expires": {{"kind": "literal", "value": "2026-01-06T09:00:00Z"}},
          "from": {{"kind": "literal", "value": "agent:ws/planner"}},
          "kind": {{"kind": "literal", "value": "note"}},
          "message_id": {{"kind": "literal", "value": "{id}"}},
          "on_behalf_of": {{"kind": "literal", "value": "agent:ws/planner"}},
          "signature": {{"kind": "literal", "value": {{"key_id": "key-1", "value": "c2lnbmF0dXJl"}}}},
          "thread": {{"kind": "literal", "value": "{id}"}},
          "to": {{"kind": "literal", "value": "agent:box/worker"}}}}}}"#
    )
}

fn expect_outcome(command: &str, outcome: &str) -> String {
    format!(
        r#"{{"step": "expect_outcome", "outcome": {{"command": "{command}", "outcome": "{outcome}"}}}}"#
    )
}

const FIRST: &str = "01JGT0DK000000000000000001";
const SECOND: &str = "01JGT0DK000000000000000002";

/// The authored scenario `repeated-dedup-key`, cut down: open, post, post again with the key,
/// expecting `second` of the second post, then read the inbox.
fn repeated_key(second: &str) -> Scenario {
    let steps = [
        OPEN.to_owned(),
        expect_outcome("inbox.messaging.OpenInbox", "opened"),
        post(FIRST, "deploy-42"),
        expect_outcome("inbox.messaging.PostMessage", "posted"),
        r#"{"step": "expect_no_error"}"#.to_owned(),
        post(SECOND, "deploy-42"),
        expect_outcome("inbox.messaging.PostMessage", second),
        r#"{"step": "expect_no_event", "event": "inbox.messaging.MessagePosted"}"#.to_owned(),
        r#"{"step": "query_view", "view": "inbox.messaging.Pending", "params": {"inbox": {"kind": "literal", "value": "agent:box/worker"}}}"#.to_owned(),
        r#"{"step": "expect_view", "view": "inbox.messaging.Pending", "expectation": {"expect": "counts", "at_least": 1, "at_most": 1}}"#.to_owned(),
    ];
    scenario(&format!("[{}]", steps.join(",")), "")
}

#[test]
fn the_core_refuses_a_repeated_dedup_key_and_the_runner_sees_it() {
    let passed = run_scenario(&repeated_key("duplicate-key"), &mut CoreTarget::new());
    assert_eq!(passed.status, Status::Passed, "{:?}", passed.message);

    let wrong = run_scenario(&repeated_key("posted"), &mut CoreTarget::new());
    assert_eq!(wrong.status, Status::Failed);
    let message = wrong.message.expect("a failure says why");
    assert!(
        message.contains("should answer `posted`") && message.contains("`duplicate-key`"),
        "{message}"
    );
}

#[test]
fn a_step_whose_expected_outcome_differs_from_the_answer_fails() {
    let steps = format!(
        "[{}, {}]",
        post(FIRST, "k"),
        expect_outcome("inbox.messaging.PostMessage", "duplicate-key")
    );
    let mut target = Scripted {
        answers: vec![outcome("posted", None, &[])],
        ..Scripted::default()
    };
    let result = run_scenario(&scenario(&steps, ""), &mut target);
    assert_eq!(result.status, Status::Failed);
    assert_eq!(
        result.message.as_deref(),
        Some(
            "step 1: `inbox.messaging.PostMessage` should answer `duplicate-key`; it answered `posted`"
        )
    );
    assert_eq!(target.sent[0].1.as_deref(), Some("duplicate-key"));
}

#[test]
fn a_target_that_only_echoes_the_expected_outcome_fails() {
    let result = run_scenario(&repeated_key("duplicate-key"), &mut Echo);
    assert_eq!(result.status, Status::Failed, "{:?}", result.message);
    let steps = format!(
        r#"[{}, {}, {{"step": "expect_event", "event": "inbox.messaging.InboxOpened", "payload": {{"address": "agent:box/worker"}}}}]"#,
        OPEN,
        expect_outcome("inbox.messaging.OpenInbox", "opened")
    );
    let result = run_scenario(&scenario(&steps, ""), &mut Echo);
    assert_eq!(result.status, Status::Failed);
    assert!(result.message.unwrap().contains("it published nothing"));
}

#[test]
fn an_error_an_event_payload_and_a_row_are_each_compared() {
    let opened = (
        "inbox.messaging.InboxOpened",
        r#"{"address": "agent:box/worker"}"#,
    );
    let run = |answer: Executed, rows: &str, step: &str| {
        let steps = format!(
            r#"[{OPEN}, {step}, {{"step": "query_view", "view": "inbox.messaging.Inboxes"}},
               {{"step": "expect_view", "view": "inbox.messaging.Inboxes", "expectation": {{"expect": "contains",
                 "fields": {{"address": {{"kind": "literal", "value": "agent:box/worker"}}, "state": {{"kind": "literal", "value": "Open"}}}}}}}}]"#
        );
        let mut target = Scripted {
            answers: vec![answer],
            reads: vec![Read::Rows(match parse(rows) {
                Value::Array(rows) => rows,
                _ => unreachable!(),
            })],
            ..Scripted::default()
        };
        run_scenario(&scenario(&steps, ""), &mut target)
    };
    let row = r#"[{"address": "agent:box/worker", "state": "Open", "owner": "agent:box/worker"}]"#;
    let event_claim = r#"{"step": "expect_event", "event": "inbox.messaging.InboxOpened", "payload": {"address": "agent:box/worker"}}"#;

    let passed = run(outcome("opened", None, &[opened]), row, event_claim);
    assert_eq!(passed.status, Status::Passed, "{:?}", passed.message);

    let wrong_payload = run(
        outcome(
            "opened",
            None,
            &[(
                "inbox.messaging.InboxOpened",
                r#"{"address": "agent:box/other"}"#,
            )],
        ),
        row,
        event_claim,
    );
    assert_eq!(wrong_payload.status, Status::Failed);

    let extra_member = run(
        outcome(
            "opened",
            None,
            &[(
                "inbox.messaging.InboxOpened",
                r#"{"address": "agent:box/worker", "owner": "x"}"#,
            )],
        ),
        row,
        event_claim,
    );
    assert_eq!(extra_member.status, Status::Failed);

    let wrong_row = run(
        outcome("opened", None, &[opened]),
        r#"[{"address": "agent:box/worker", "state": "Closed"}]"#,
        event_claim,
    );
    assert_eq!(wrong_row.status, Status::Failed);
    assert!(
        wrong_row
            .message
            .unwrap()
            .contains("no row of `inbox.messaging.Inboxes` holds")
    );

    let error = run(
        outcome("opened", Some("inbox.messaging.Duplicate"), &[opened]),
        row,
        r#"{"step": "expect_no_error"}"#,
    );
    assert_eq!(error.status, Status::Failed);
}

#[test]
fn a_claimed_null_holds_of_an_absent_or_null_member_and_not_of_a_value() {
    let contains_null = |rows: &str| {
        let steps = r#"[{"step": "query_view", "view": "inbox.messaging.Inboxes"},
            {"step": "expect_view", "view": "inbox.messaging.Inboxes", "expectation": {"expect": "contains",
              "fields": {"address": {"kind": "literal", "value": "a"}, "owner": {"kind": "literal", "value": null}}}}]"#;
        let mut target = Scripted {
            reads: vec![Read::Rows(match parse(rows) {
                Value::Array(rows) => rows,
                _ => unreachable!(),
            })],
            ..Scripted::default()
        };
        run_scenario(&scenario(steps, ""), &mut target)
    };
    for rows in [
        r#"[{"address": "a", "owner": null}]"#,
        r#"[{"address": "a"}]"#,
    ] {
        let result = contains_null(rows);
        assert_eq!(result.status, Status::Passed, "{rows}: {result:?}");
    }
    let valued = contains_null(r#"[{"address": "a", "owner": "agent:box/worker"}]"#);
    assert_eq!(valued.status, Status::Failed, "{valued:?}");
}

#[test]
fn two_claims_of_one_event_need_two_occurrences() {
    let claim = r#"{"step": "expect_event", "event": "inbox.messaging.InboxOpened", "payload": {"address": "a"}}"#;
    let steps = format!("[{OPEN}, {claim}, {claim}]");
    let once = ("inbox.messaging.InboxOpened", r#"{"address": "a"}"#);
    let mut target = Scripted {
        answers: vec![outcome("opened", None, &[once])],
        ..Scripted::default()
    };
    assert_eq!(
        run_scenario(&scenario(&steps, ""), &mut target).status,
        Status::Failed
    );
    let mut target = Scripted {
        answers: vec![outcome("opened", None, &[once, once])],
        ..Scripted::default()
    };
    assert_eq!(
        run_scenario(&scenario(&steps, ""), &mut target).status,
        Status::Passed
    );
}

#[test]
fn absent_events_expand_to_an_expect_no_event_per_catalog_event_left_out() {
    let read = scenario(
        &format!("[{OPEN}]"),
        r#", "absent_events": [{"before": 1, "except": ["inbox.messaging.InboxOpened"]}]"#,
    );
    let steps = read.steps.as_ref().expect("admitted");
    assert_eq!(steps.len(), 2);
    let posted = ("inbox.messaging.MessagePosted", r#"{"message_id": "m"}"#);
    let mut target = Scripted {
        answers: vec![outcome("opened", None, &[posted])],
        ..Scripted::default()
    };
    let result = run_scenario(&read, &mut target);
    assert_eq!(result.status, Status::Failed);
    assert!(
        result
            .message
            .unwrap()
            .contains("should not publish `inbox.messaging.MessagePosted`")
    );
}

#[test]
fn a_construct_the_runner_does_not_interpret_is_unsupported_and_nothing_runs() {
    for (steps, reason) in [
        (
            format!(r#"[{OPEN}, {{"step": "deliver_event", "event": "x"}}]"#),
            "step 1: the step kind `deliver_event` is not interpreted by this runner",
        ),
        (
            r#"[{"step": "expect_error", "error": "e", "fields": {}}]"#.to_owned(),
            "step 0: `expect_error`: the key `fields` is not interpreted by this runner",
        ),
        (
            r#"[{"step": "query_view", "view": "v", "params": {"p": {"kind": "observed", "event": "e", "field": "f"}}}]"#.to_owned(),
            "step 0: the value reference `observed` in params is not interpreted by this runner",
        ),
        (
            r#"[{"step": "expect_view", "view": "v", "expectation": {"expect": "satisfies", "predicate": "a.count + 1 == 2"}}]"#.to_owned(),
            "step 0: the predicate `a.count + 1 == 2` is not one this runner interprets",
        ),
    ] {
        let read = scenario(&steps, "");
        assert_eq!(read.unsupported(), Some(reason));
        let mut target = Scripted::default();
        let result = run_scenario(&read, &mut target);
        assert_eq!(result.status, Status::Unsupported);
        assert_eq!(result.message.as_deref(), Some(reason));
        assert!(target.sent.is_empty() && target.queried.is_empty());
    }
    let unknown_member = scenario("[]", r#", "retained": []"#);
    assert_eq!(
        unknown_member.unsupported(),
        Some("the scenario: the key `retained` is not interpreted by this runner")
    );
}

#[test]
fn a_suite_of_another_format_is_refused_whole() {
    let refused = suite::read(
        r#"{"provenance": {"suite_version": "ess-conformance/54", "scenario_initial_state": "empty"}, "events": [], "scenarios": {}}"#,
    )
    .unwrap_err();
    assert!(refused.contains("`ess-conformance/54`"), "{refused}");
}

#[test]
fn not_granted_names_its_actor_and_reads_keep_their_reader() {
    let steps = r#"[{"step": "query_view", "view": "inbox.messaging.Due"},
        {"step": "read_as", "actor": "inbox.messaging.Client"},
        {"step": "query_view", "view": "inbox.messaging.Due"},
        {"step": "expect_not_granted", "actor": "inbox.messaging.Client"},
        {"step": "query_view", "view": "inbox.messaging.Due"},
        {"step": "expect_not_granted", "actor": null}]"#;
    let mut target = Scripted {
        reads: vec![
            Read::Rows(Vec::new()),
            Read::NotGranted(Some("inbox.messaging.Client".to_owned())),
            Read::NotGranted(Some("inbox.messaging.Client".to_owned())),
        ],
        ..Scripted::default()
    };
    let result = run_scenario(&scenario(steps, ""), &mut target);
    assert_eq!(result.status, Status::Failed);
    assert_eq!(
        result.message.as_deref(),
        Some("step 5: the refusal should name no actor; it names `inbox.messaging.Client`")
    );
    let client = Reader::As(Some("inbox.messaging.Client".to_owned()));
    assert_eq!(
        target.queried,
        vec![
            ("inbox.messaging.Due".to_owned(), Reader::Unstated),
            ("inbox.messaging.Due".to_owned(), client.clone()),
            ("inbox.messaging.Due".to_owned(), client),
        ]
    );
}

#[test]
fn a_forced_external_outcome_the_step_does_not_expect_is_an_error() {
    let steps = format!(
        r#"[{{"step": "configure_external_outcome", "force": {{"command": "inbox.messaging.PostMessage", "outcome": "signature-invalid"}}}},
            {}, {}]"#,
        post(FIRST, "k"),
        expect_outcome("inbox.messaging.PostMessage", "posted")
    );
    let mut target = Scripted {
        answers: vec![outcome("posted", None, &[])],
        ..Scripted::default()
    };
    let result = run_scenario(&scenario(&steps, ""), &mut target);
    assert_eq!(result.status, Status::Error);
    assert!(target.sent.is_empty());
}

#[test]
fn the_step_context_takes_only_the_expected_branch() {
    let caller = [
        ("principal".to_owned(), "agent:ws/planner".to_owned()),
        ("inbox".to_owned(), "agent:ws/planner".to_owned()),
    ];
    let context = StepContext::new(&caller, Some("signature-invalid")).unwrap();
    assert!(context.takes("signature-invalid"));
    assert!(!context.takes("authority-invalid") && !context.takes("unknown-option"));
    assert!(
        !StepContext::new(&caller, Some("posted"))
            .unwrap()
            .takes("signature-invalid")
    );
    assert!(
        !StepContext::new(&caller, None)
            .unwrap()
            .takes("signature-invalid")
    );
    let stray = [("tenant".to_owned(), "x".to_owned())];
    assert!(StepContext::new(&stray, None).is_err());
}

#[test]
fn a_predicate_and_a_value_invariant_are_checked_on_every_row() {
    let satisfies = |rows: &str| {
        let steps = r#"[{"step": "query_view", "view": "inbox.messaging.Due"},
            {"step": "expect_view", "view": "inbox.messaging.Due", "expectation": {"expect": "satisfies", "predicate": "message_id.count == 3"}},
            {"step": "expect_view", "view": "inbox.messaging.Due", "expectation": {"expect": "satisfies",
              "predicate": {"compare": {"left": {"utf8_bytes": "body"}, "op": "lte", "right": 4.0}}}}]"#;
        let mut target = Scripted {
            reads: vec![Read::Rows(match parse(rows) {
                Value::Array(rows) => rows,
                _ => unreachable!(),
            })],
            ..Scripted::default()
        };
        run_scenario(&scenario(steps, ""), &mut target)
    };
    assert_eq!(
        satisfies(
            r#"[{"message_id": "abc", "body": "äb"}, {"message_id": "xyz", "body": "abcd"}]"#
        )
        .status,
        Status::Passed
    );
    assert_eq!(
        satisfies(r#"[{"message_id": "abcd", "body": "a"}]"#).status,
        Status::Failed
    );
    assert_eq!(
        satisfies(r#"[{"message_id": "abc", "body": "äbcd"}]"#).status,
        Status::Failed
    );

    let declarations = r#"{"inbox.messaging.DecisionAsk": {"kind": "struct", "fields": [
                  {"name": "options", "type": "List<inbox.messaging.DecisionOption>"},
                  {"name": "recommended_option", "type": "Optional<inbox.messaging.OptionLetter>"},
                  {"name": "request_ref", "type": "Optional<String>"}], "invariants": []},
                "inbox.messaging.DecisionOption": {"kind": "struct", "fields": [
                  {"name": "letter", "type": "inbox.messaging.OptionLetter"},
                  {"name": "text", "type": "String"}], "invariants": []},
                "inbox.messaging.MessageId": {"kind": "newtype", "of": "String",
                  "invariants": [{"statement": "value.count == 26", "predicate": "value.count == 26"}]},
                "inbox.messaging.OptionLetter": {"kind": "newtype", "of": "String",
                  "invariants": [{"statement": "value.count == 1", "predicate": "value.count == 1"}], "alphabet": "AB"}}"#;
    let options = |letter: &str| {
        format!(
            r#"{{"options": [{{"letter": "A", "text": "t"}}, {{"letter": "{letter}", "text": "u"}}]}}"#
        )
    };
    let witnessed = |rows: &str, declarations: &str| {
        let steps = format!(
            r#"[{}, {{"step": "query_view", "view": "inbox.messaging.Due"}},
            {{"step": "expect_view", "view": "inbox.messaging.Due", "expectation": {{"expect": "value_invariants",
              "root": {{"name": "decision", "type": "Optional<inbox.messaging.DecisionAsk>"}},
              "declarations": {declarations},
              "value": "inbox.messaging.OptionLetter",
              "position": {{"kind": "site", "path": [{{"op": "present"}}, {{"op": "field", "name": "options"}}, {{"op": "element"}}, {{"op": "field", "name": "letter"}}]}},
              "witness": {{"kind": "arranged_row", "selector": {{"kind": "projected_identity", "entity": "inbox.messaging.Message",
                "identity": {{"name": "message_id", "type": "inbox.messaging.MessageId"}},
                "expected": {{"kind": "observed", "event": "inbox.messaging.MessagePosted", "field": "message_id"}}}}}},
              "work_limit": 1048576}}}}]"#,
            post(FIRST, "k")
        );
        let mut target = Scripted {
            answers: vec![outcome(
                "posted",
                None,
                &[(
                    "inbox.messaging.MessagePosted",
                    format!(r#"{{"message_id": "{FIRST}"}}"#).as_str(),
                )],
            )],
            reads: vec![Read::Rows(match parse(rows) {
                Value::Array(rows) => rows,
                _ => unreachable!(),
            })],
            ..Scripted::default()
        };
        run_scenario(&scenario(&steps, ""), &mut target)
    };
    let invariant = |letter: &str| {
        witnessed(
            &format!(
                r#"[{{"message_id": "{FIRST}", "decision": {}}}]"#,
                options(letter)
            ),
            declarations,
        )
    };
    assert_eq!(invariant("B").status, Status::Passed);
    assert_eq!(invariant("BB").status, Status::Failed);
    assert_eq!(invariant("C").status, Status::Failed);
    let unwitnessed = witnessed(
        &format!(
            r#"[{{"message_id": "{FIRST}", "decision": null}}, {{"message_id": "{SECOND}", "decision": {}}}]"#,
            options("B")
        ),
        declarations,
    );
    assert_eq!(unwitnessed.status, Status::Failed, "{unwitnessed:?}");
    for empty in [
        format!(r#"[{{"message_id": "{FIRST}"}}]"#),
        format!(r#"[{{"message_id": "{FIRST}", "decision": {{"options": []}}}}]"#),
    ] {
        let none = witnessed(&empty, declarations);
        assert_eq!(none.status, Status::Failed, "{empty}: {none:?}");
    }
    let other_row = witnessed(
        &format!(
            r#"[{{"message_id": "{FIRST}", "decision": {}}}, {{"message_id": "{SECOND}", "decision": {}}}]"#,
            options("B"),
            options("BB")
        ),
        declarations,
    );
    assert_eq!(other_row.status, Status::Failed, "{other_row:?}");
    let undeclared = witnessed(
        &format!(
            r#"[{{"message_id": "{FIRST}", "decision": {}}}]"#,
            options("B")
        ),
        r#"{"inbox.messaging.OptionLetter": {"kind": "newtype", "of": "String",
            "invariants": [{"statement": "value.count == 1", "predicate": "value.count == 1"}]}}"#,
    );
    assert_eq!(undeclared.status, Status::Error, "{undeclared:?}");
}

#[test]
fn the_results_document_is_one_terminal_status_per_scenario() {
    let digest = results::suite_digest(b"abc");
    assert_eq!(
        digest,
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let outcomes = [
        inbox_conformance::ScenarioResult {
            id: "a".to_owned(),
            status: Status::Passed,
            message: None,
        },
        inbox_conformance::ScenarioResult {
            id: "b".to_owned(),
            status: Status::Unsupported,
            message: Some("step 1: \"why\"".to_owned()),
        },
    ];
    let document = parse(&results::document(&digest, 1_700_000_000_000, &outcomes));
    assert_eq!(
        document,
        parse(&format!(
            r#"{{"format": "ess-conformance-results/1", "suite_digest": "{digest}", "completed_at": 1700000000000,
                "results": [{{"scenario_id": "a", "status": "passed"}},
                            {{"scenario_id": "b", "status": "unsupported", "message": "step 1: \"why\""}}]}}"#
        ))
    );
    let names: Vec<&str> = [
        Status::Passed,
        Status::Failed,
        Status::Error,
        Status::Unsupported,
    ]
    .iter()
    .map(|status| status.name())
    .collect();
    assert_eq!(names, ["passed", "failed", "error", "unsupported"]);
}

fn report(failed: &[&str], error: &[&str], unsupported: &[&str], passed: u64) -> gate::Report {
    let ids = |ids: &[&str]| ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>();
    gate::Report {
        total: passed + (failed.len() + error.len() + unsupported.len()) as u64,
        passed,
        failed: ids(failed),
        error: ids(error),
        skipped: Vec::new(),
        unsupported: ids(unsupported),
    }
}

#[test]
fn the_unsupported_scenarios_must_be_exactly_the_listed_ones() {
    let listed = gate::read_listed("x/one\tthe step kind `deliver_event`\n").unwrap();

    let held = gate::verdict(&report(&[], &[], &["x/one"], 91), &listed);
    assert!(held.passes(), "{:?}", held.problems);
    assert_eq!(
        held.line,
        "92 scenarios: 91 passed, 0 failed, 0 error, 1 unsupported"
    );

    let unlisted = gate::verdict(&report(&[], &[], &["x/one", "x/two"], 90), &listed);
    assert_eq!(
        unlisted.problems,
        ["unsupported and not listed in unsupported.tsv: x/two"]
    );

    let stale = gate::verdict(&report(&[], &[], &[], 92), &listed);
    assert_eq!(
        stale.problems,
        ["listed in unsupported.tsv and not unsupported in the run: x/one"]
    );

    let broken = gate::verdict(&report(&["x/f"], &["x/e"], &["x/one"], 89), &listed);
    assert_eq!(broken.problems, ["failed: x/f", "error: x/e"]);

    let empty = gate::verdict(&report(&[], &[], &[], 0), &gate::Listed::new());
    assert!(!empty.passes());
}

#[test]
fn the_unsupported_list_is_read_strictly() {
    assert!(gate::read_listed("").unwrap().is_empty());
    assert!(gate::read_listed("x/one\n").is_err());
    assert!(gate::read_listed("x/one\t\n").is_err());
    assert!(gate::read_listed("\treason\n").is_err());
    assert!(gate::read_listed("x/one\ta\nx/one\tb\n").is_err());
}

#[test]
fn the_report_is_read_and_its_counts_must_agree_with_its_lists() {
    let text = r#"{"format": "ess-conformance-report/2",
        "counts": {"total": 3, "passed": 1, "failed": 1, "error": 0, "skipped": 0, "unsupported": 1},
        "outcomes": {"passed": ["a"], "failed": ["b"], "error": [], "skipped": [], "unsupported": ["c"]}}"#;
    let read = gate::read_report(text).unwrap();
    assert_eq!(read, report(&["b"], &[], &["c"], 1));
    let disagreeing = text.replace(r#""failed": 1"#, r#""failed": 0"#);
    assert!(gate::read_report(&disagreeing).is_err());
    assert!(gate::read_report(&text.replace("report/2", "report/1")).is_err());
}
