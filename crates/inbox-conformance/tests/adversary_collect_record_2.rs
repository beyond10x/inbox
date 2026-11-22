//! Adversary pass 2 on story:collect-returns-the-settled-record against the runner: a response of
//! `Collect` that states a member twice, inside the union `ReplyShape` or at its top level, is
//! no value of the declared response, and `expect_direct_response` must not pass it.

use inbox::server::json::{self, Value};
use inbox_conformance::suite::{self, Scenario};
use inbox_conformance::{Answer, Command, Executed, Read, Reader, Status, Target, run_scenario};

fn scenario(steps: &str) -> Scenario {
    let text = format!(
        r#"{{"provenance": {{"suite_version": "ess-conformance/65", "scenario_initial_state": "empty"}},
            "coverage": {{"authored": [], "authored_sources": {{}},
              "counts": {{"authored": 0, "generated": 1, "outside": 0, "refused": 0}},
              "generated": ["probe"], "knowledge": "complete_inventory", "outside": [], "refused": [],
              "selection": {{"filter": {{"kind": "all"}}, "origins": "generated_and_authored", "scope": {{"kind": "system"}}}}}},
            "scenarios": {{"probe": {{"purpose": "a probe", "source": [], "steps": {steps}}}}}}}"#
    );
    let mut read = suite::read(&text).expect("the probe suite reads");
    read.scenarios.remove(0)
}

/// A target that answers the one command with `collected` and this response.
struct Collected(Option<Value>);

impl Target for Collected {
    fn reset(&mut self) {}

    fn execute(&mut self, _command: &Command<'_>) -> Executed {
        Executed {
            answer: Answer::Outcome {
                outcome: "collected".to_owned(),
                error: None,
                response: self.0.take(),
            },
            events: Vec::new(),
        }
    }

    fn query(&mut self, view: &str, _reader: &Reader, _params: Value) -> Read {
        panic!("the probe reads no view, and `{view}` was read")
    }
}

const STEPS: &str = r#"[{"step": "execute_command", "command": "inbox.messaging.Collect",
    "actor": "inbox.messaging.Client", "caller": {"inbox": "agent:ws/planner", "principal": "agent:ws/planner"},
    "input": {"message_id": {"kind": "literal", "value": "01JGT0AF000000000000000001"}}},
  {"step": "expect_direct_response", "response": {
    "command": "inbox.messaging.Collect",
    "outcome": {"command": "inbox.messaging.Collect", "outcome": "collected"},
    "fields": [{"name": "message_id", "type": "inbox.messaging.MessageId"},
      {"name": "reply_shape", "type": "Optional<inbox.messaging.ReplyShape>"},
      {"name": "state", "type": "inbox.messaging.Message.State"}],
    "declarations": {
      "inbox.messaging.MessageId": {"kind": "newtype", "of": "String"},
      "inbox.messaging.Message.State": {"kind": "enum", "variants": ["Answered", "Refused", "Sent"]},
      "inbox.messaging.ReplyShape": {"kind": "union", "tag": "shape",
        "variants": {"free": "inbox.messaging.FreeShape", "lines": "Integer", "schema": "String"}},
      "inbox.messaging.FreeShape": {"kind": "struct", "fields": []}},
    "constraints": {"inbox.messaging.MessageId": {"alphabet": "0123456789ABCDEFGHJKMNPQRSTVWXYZ",
      "invariants": ["value.count == 26"], "prefix": null}},
    "expected": {}}}]"#;

/// The same response with each member once passes; stating the tag of the union twice (two
/// variants at once), its `value` twice (two payloads, one of the wrong type), or a top-level
/// member twice (a second `state` that is no label, a second `message_id` off its alphabet)
/// fails the step.
#[test]
fn a_response_that_states_a_member_twice_fails_the_step() {
    let id = "\"message_id\": \"01JGT0AF000000000000000001\"";
    let passing = format!(
        r#"{{{id}, "reply_shape": {{"shape": "lines", "value": 2}}, "state": "Answered"}}"#
    );
    let read = scenario(STEPS);
    let mut target = Collected(Some(json::parse(&passing).expect("JSON")));
    let result = run_scenario(&read, &mut target);
    assert_eq!(result.status, Status::Passed, "{:?}", result.message);

    let mut wrong = Vec::new();
    for (name, response) in [
        (
            "the union's tag twice",
            format!(
                r#"{{{id}, "reply_shape": {{"shape": "lines", "value": 2, "shape": "words"}}, "state": "Answered"}}"#
            ),
        ),
        (
            "the union's value twice",
            format!(
                r#"{{{id}, "reply_shape": {{"shape": "lines", "value": 2, "value": "two"}}, "state": "Answered"}}"#
            ),
        ),
        (
            "`state` twice",
            format!(r#"{{{id}, "state": "Answered", "state": "Gone"}}"#),
        ),
        (
            "`message_id` twice",
            format!(r#"{{{id}, "state": "Answered", "message_id": "short"}}"#),
        ),
    ] {
        let mut target = Collected(Some(json::parse(&response).expect("JSON")));
        let result = run_scenario(&read, &mut target);
        if result.status != Status::Failed {
            wrong.push(format!(
                "{name}: {:?} ({:?})",
                result.status, result.message
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
