//! story:collect-returns-the-settled-record against the binary: `inbox collect` prints the record
//! the answer of its `Collect` carries, not a row it read before, and removes its outbox line only
//! after it holds that answer. A message that is not settled keeps its line, and a third principal
//! gets `unknown-message` and no record.

use crate::support;

use inbox::server::json::Value;

use support::{
    ALICE, BOB, REFUSED, World, add_key, ids, key_new, keys_only, serve, stand_in_answers,
    stand_in_by_method, text,
};

const CAROL: &str = "agent:test/carol";

fn member<'a>(value: &'a Value, path: &[&str]) -> &'a Value {
    path.iter().fold(value, |at, name| {
        at.member(name)
            .unwrap_or_else(|| panic!("no `{name}` in {at:?}"))
    })
}

/// Bob's inbox open and a request of alice in it, kept in alice's outbox file; answers its id.
fn requested(world: &World) -> String {
    world.client("bob", &["open"]).ok();
    let posted = world
        .client(
            "alice",
            &[
                "post",
                "--to",
                BOB,
                "--kind",
                "request",
                "--reply-shape",
                "free",
                "--body",
                "Which colour?",
                "--json",
            ],
        )
        .ok()
        .json();
    let request = text(&posted, "message_id");
    assert_eq!(world.kept("alice"), std::slice::from_ref(&request));
    request
}

/// The answer the service gives a `Collect` of `id` after bob answered it "Blue".
fn collected_answer(id: &str) -> String {
    format!(
        "{{\"outcome\":\"collected\",\"published\":[{{\"event\":\"inbox.messaging.MessageHandled\",\
         \"payload\":{{\"message_id\":\"{id}\"}}}}],\"response\":{{\"message_id\":\"{id}\",\
         \"from\":\"{ALICE}\",\"on_behalf_of\":\"{ALICE}\",\"to\":\"{BOB}\",\"kind\":\"request\",\
         \"thread\":\"{id}\",\"reply_shape\":{{\"shape\":\"free\",\"value\":{{}}}},\
         \"created\":\"2026-01-05T09:00:00Z\",\"expires\":\"2026-01-06T09:05:00Z\",\
         \"dedup_key\":\"{id}\",\"body\":\"Which colour?\",\
         \"signature\":{{\"key_id\":\"alice-1\",\"value\":\"c2lnbmF0dXJl\"}},\
         \"answered_by\":\"principal\",\
         \"reply\":{{\"reply_id\":\"01JGT0CR000000000000000050\",\"from\":\"{BOB}\",\
         \"on_behalf_of\":\"{BOB}\",\"created\":\"2026-01-05T09:05:00Z\",\"body\":\"Blue\",\
         \"signature\":{{\"key_id\":\"bob-1\",\"value\":\"c2lnbmF0dXJl\"}}}},\
         \"state\":\"Answered\"}}}}"
    )
}

/// The recipient answers between the client's read of `Outbox` and its `Collect`: the stand-in
/// answers every read with the row as it was before the answer (state `Sent`, no reply), and the
/// `Collect` with the record the service removed. What is printed is that record.
#[test]
fn collect_prints_the_record_of_its_answer_not_a_row_read_before() {
    let world = World::new("collect-record-race");
    let request = requested(&world);
    let pending = world.client("alice", &["outbox", "--json"]).ok();
    assert_eq!(ids(&pending.json()), std::slice::from_ref(&request));

    for json in [false, true] {
        if json {
            let again = requested(&world);
            assert_eq!(world.kept("alice").last(), Some(&again));
        }
        let id = world.kept("alice").last().cloned().expect("a kept line");
        let before = world.client("alice", &["outbox", "--json"]).ok();
        let (url, stand_in) =
            stand_in_by_method(before.stdout.trim().to_owned(), collected_answer(&id));
        let mut args = vec!["collect", id.as_str()];
        if json {
            args.push("--json");
        }
        let run = world.client_env("alice", &args, &[("INBOX_URL", url.as_str())]);
        let requests = stand_in.join().expect("the stand-in answered");
        assert_eq!(run.status, 0, "{run:#?}");
        if json {
            let printed = run.json();
            assert_eq!(text(member(&printed, &["answer"]), "outcome"), "collected");
            assert_eq!(text(member(&printed, &["message"]), "state"), "Answered");
            assert_eq!(
                text(member(&printed, &["message", "reply"]), "body"),
                "Blue"
            );
        } else {
            assert!(
                run.stdout.starts_with(&format!("collected {id}\n")),
                "{run:#?}"
            );
            assert!(run.stdout.contains(" Answered "), "{run:#?}");
            assert!(run.stdout.contains("    Blue"), "{run:#?}");
            assert!(!run.stdout.contains(" Sent "), "{run:#?}");
        }
        assert!(
            requests.iter().all(|line| line.starts_with("POST ")),
            "the client read a row before its Collect: {requests:?}"
        );
        assert!(
            !world.kept("alice").contains(&id),
            "a collected message leaves the outbox file"
        );
    }
    assert!(world.kept("alice").is_empty());
}

#[test]
fn collect_of_a_message_not_settled_keeps_it_and_its_outbox_line() {
    let world = World::new("collect-record-early");
    let request = requested(&world);
    let early = world.client("alice", &["collect", &request]);
    assert_eq!(early.status, REFUSED, "{early:#?}");
    assert!(early.stderr.contains("NotSettled"), "{early:#?}");
    assert!(!early.stdout.contains("Which colour?"), "{early:#?}");
    assert_eq!(world.kept("alice"), std::slice::from_ref(&request));

    world
        .client("bob", &["answer", &request, "--body", "Blue"])
        .ok();
    let collected = world
        .client("alice", &["collect", &request, "--json"])
        .ok()
        .json();
    assert_eq!(text(member(&collected, &["message"]), "state"), "Answered");
    assert_eq!(
        text(member(&collected, &["message", "reply"]), "body"),
        "Blue"
    );
    assert!(world.kept("alice").is_empty());
}

/// A third principal (carol, neither sender nor recipient) gets `unknown-message` and no record;
/// the recipient gets `not-sender`. Neither removes the message, and the sender then collects it.
#[test]
fn a_third_principal_gets_unknown_message_and_no_record() {
    let (dir, operator_public_key) = keys_only("collect-record-third");
    let public_key = key_new(&dir.path, "carol.key", Some(("carol-1", CAROL)));
    add_key(&dir.path, "operator.key", "carol-1", CAROL, &public_key);
    let world = World {
        service: serve(&dir.path, &operator_public_key),
        operator_public_key,
        dir,
    };
    let request = requested(&world);
    world
        .client("bob", &["answer", &request, "--body", "Blue"])
        .ok();

    for (who, name) in [
        ("carol", "inbox.messaging.UnknownMessage"),
        ("bob", "inbox.messaging.NotSender"),
    ] {
        let refused = world.client(who, &["collect", &request, "--json"]);
        assert_eq!(refused.status, REFUSED, "{who}: {refused:#?}");
        assert!(refused.stderr.contains(name), "{who}: {refused:#?}");
        assert!(!refused.stdout.contains("Blue"), "{who}: {refused:#?}");
        assert!(
            !refused.stdout.contains("\"response\""),
            "{who}: {refused:#?}"
        );
    }
    assert_eq!(world.kept("alice"), std::slice::from_ref(&request));
    let collected = world
        .client("alice", &["collect", &request, "--json"])
        .ok()
        .json();
    assert_eq!(
        text(member(&collected, &["message", "reply"]), "body"),
        "Blue"
    );
}

/// A decision that silence answered: the record carries `answered_by` `silence` and the
/// `decided_option`, and no reply. `inbox collect` prints the row with the line that names the
/// option silence took, exits 0 and drops the outbox line. The service decides on silence only
/// when `expires` passes, so a stand-in answers the `Collect` with that record.
#[test]
fn collect_of_a_decision_taken_on_silence_prints_the_option_silence_took() {
    let world = World::new("collect-record-silence");
    let id = requested(&world);
    let answer = format!(
        "{{\"outcome\":\"collected\",\"published\":[],\"response\":{{\"message_id\":\"{id}\",\
         \"from\":\"{ALICE}\",\"on_behalf_of\":\"{ALICE}\",\"to\":\"{BOB}\",\"kind\":\"decision\",\
         \"thread\":\"{id}\",\"decision\":{{\"options\":[{{\"letter\":\"A\",\"text\":\"Ship\"}},\
         {{\"letter\":\"B\",\"text\":\"Hold\"}}]}},\"default_on_silence\":\"B\",\
         \"created\":\"2026-01-05T09:00:00Z\",\"expires\":\"2026-01-06T09:00:00Z\",\
         \"dedup_key\":\"{id}\",\"body\":\"Ship or hold?\",\
         \"signature\":{{\"key_id\":\"alice-1\",\"value\":\"c2lnbmF0dXJl\"}},\
         \"answered_by\":\"silence\",\"decided_option\":\"B\",\"state\":\"Answered\"}}}}"
    );
    let (url, stand_in) = stand_in_answers(vec![(200, None, Box::leak(answer.into_boxed_str()))]);
    let run = world.client_env("alice", &["collect", &id], &[("INBOX_URL", url.as_str())]);
    let _ = stand_in.join();
    assert_eq!(run.status, 0, "{run:#?}");
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines[0], format!("collected {id}"), "{run:#?}");
    assert!(
        lines[1].starts_with(&format!("{id} decision Answered from {ALICE} to {BOB} ")),
        "{run:#?}"
    );
    assert_eq!(
        &lines[2..],
        [
            "    Ship or hold?",
            "  A) Ship",
            "  B) Hold",
            "  decided on silence: option B"
        ],
        "{run:#?}"
    );
    assert!(
        world.kept("alice").is_empty(),
        "the collected line is dropped"
    );
}
