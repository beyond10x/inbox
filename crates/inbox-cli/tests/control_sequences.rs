//! The client writes no control character another principal or the service chose to the terminal
//! of whoever runs it: in text, each control character of such a value but the line end is shown
//! escaped (`\u{1b}`); with `--json` none is raw and a JSON reader gets the value as it was sent.

use crate::support;

use inbox::server::json::Value;
use support::{ALICE, BOB, REFUSED, TRANSPORT, World, keys_only, rows, run, stand_in, text};

/// Terminal control sequences of every kind the story names: an erase of the screen, a window
/// title set by OSC, a bell, a carriage return that writes over the line, a C1 control sequence
/// introducer, DEL, a tab and a carriage return before a line end.
const HOSTILE: &str = "Blue\u{1b}[2J\u{1b}]0;owned\u{7}\rRed\u{9b}31m\u{7f}\tend\r\nnext";

/// What a terminal shows of [`HOSTILE`] in a line of its own, up to its line end.
const SHOWN: &str =
    "Blue\\u{1b}[2J\\u{1b}]0;owned\\u{7}\\u{d}Red\\u{9b}31m\\u{7f}\\u{9}end\\u{d}\n";

/// Bob's inbox open and a request of alice in it, kept in alice's outbox file; answers its id.
fn requested(world: &World, body: &str) -> String {
    world.client("bob", &["open"]);
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
                body,
                "--json",
            ],
        )
        .ok()
        .json();
    text(&posted, "message_id")
}

/// The control characters of `output` but the line end, named with `what` and the output; none
/// when it holds none.
fn control_in(what: &str, output: &str) -> Option<String> {
    let control: Vec<char> = output
        .chars()
        .filter(|c| c.is_control() && *c != '\n')
        .collect();
    (!control.is_empty())
        .then(|| format!("{what} printed the control characters {control:?}: {output:?}"))
}

/// `output` holds no control character but the line end, and shows [`HOSTILE`] as visible text.
fn held(what: &str, output: &str) -> Vec<String> {
    let mut wrong: Vec<String> = control_in(what, output).into_iter().collect();
    if !output.contains(SHOWN) {
        wrong.push(format!("{what} does not show {SHOWN:?}: {output:?}"));
    }
    wrong
}

/// The recipient writes terminal control sequences into its reply and into the reason of a
/// refusal: an erase of the screen, a window title set by OSC, a carriage return that writes
/// over the line. `inbox collect` prints them to the sender's terminal; no byte of its text
/// output is a control character but the line end, and the reply and the reason are shown with
/// each sequence as visible text.
#[test]
fn control_sequences_of_the_recipient_do_not_reach_the_senders_terminal() {
    let world = World::new("adv2-collect-control");
    let hostile = "Blue\u{1b}[2J\u{1b}]0;owned\u{7}\rRed\u{9b}31m";
    let shown = "Blue\\u{1b}[2J\\u{1b}]0;owned\\u{7}\\u{d}Red\\u{9b}31m";
    let answered = requested(&world, "Which colour?");
    world
        .client("bob", &["answer", &answered, "--body", hostile])
        .ok();
    let refused = requested(&world, "Which size?");
    world
        .client("bob", &["refuse", &refused, "--reason", hostile])
        .ok();
    let mut wrong = Vec::new();
    for id in [&answered, &refused] {
        let run = world.client("alice", &["collect", id]).ok();
        let control: Vec<char> = run
            .stdout
            .chars()
            .filter(|c| c.is_control() && *c != '\n')
            .collect();
        if !control.is_empty() {
            wrong.push(format!(
                "collect {id} printed the control characters {control:?}: {:?}",
                run.stdout
            ));
        }
        if !run.stdout.contains(shown) {
            wrong.push(format!(
                "collect {id} does not show {shown:?}: {:?}",
                run.stdout
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// A sender writes the sequences into the body and into an option of a decision: `inbox pending`
/// of the recipient shows them as visible text.
#[test]
fn control_sequences_of_the_sender_do_not_reach_the_terminal_of_pending() {
    let world = World::new("control-pending");
    world.client("bob", &["open"]).ok();
    let option = format!("A={HOSTILE}");
    world
        .client(
            "alice",
            &[
                "post", "--to", BOB, "--kind", "decision", "--option", &option, "--option",
                "B=plain", "--body", HOSTILE,
            ],
        )
        .ok();
    let pending = world.client("bob", &["pending"]).ok();
    let mut wrong = held("pending", &pending.stdout);
    if !pending.stdout.contains(&format!("  A) {SHOWN}")) {
        wrong.push(format!(
            "pending does not show the option: {:?}",
            pending.stdout
        ));
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The recipient answers with the sequences: `inbox outbox` of the sender shows its reply as
/// visible text, and so does `inbox thread` for both principals.
#[test]
fn control_sequences_of_another_principal_do_not_reach_the_terminal_of_outbox_or_thread() {
    let world = World::new("control-outbox-thread");
    let answered = requested(&world, HOSTILE);
    world
        .client("bob", &["answer", &answered, "--body", HOSTILE])
        .ok();
    let mut wrong = Vec::new();
    let outbox = world.client("alice", &["outbox"]).ok();
    wrong.extend(held("outbox", &outbox.stdout));
    if !outbox.stdout.contains(&format!("from {BOB}:\n    {SHOWN}")) {
        wrong.push(format!(
            "outbox does not show the reply: {:?}",
            outbox.stdout
        ));
    }
    for who in ["alice", "bob"] {
        let thread = world.client(who, &["thread", &answered]).ok();
        wrong.extend(held(&format!("thread of {who}"), &thread.stdout));
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// A line of the client's own on standard error repeats text of the service: the `refused` text
/// of a refusal and the body of an answer that is neither success nor refusal. Neither reaches
/// the terminal as a control character. The body is quoted as JSON, so its C0 characters carry
/// JSON's escape and DEL and C1 the visible one.
#[test]
fn a_line_on_standard_error_shows_the_services_control_characters_as_visible_text() {
    let (dir, _) = keys_only("control-stderr");
    let answers: [(u16, &'static str, i32, &str); 2] = [
        (
            401,
            r#"{"refused":"no\u001b[2J\u009b31m\r"}"#,
            REFUSED,
            "inbox: refused: no\\u{1b}[2J\\u{9b}31m\\u{d} (401)\n",
        ),
        (
            302,
            r#"{"moved":"\u001b]0;owned\u0007\u007f\u0085"}"#,
            TRANSPORT,
            "{\"moved\":\"\\u001b]0;owned\\u0007\\u{7f}\\u{85}\"}\n",
        ),
    ];
    let mut wrong = Vec::new();
    for (status, body, exit, shown) in answers {
        let (url, served) = stand_in(status, body);
        let pending = run(
            &dir.path,
            &["pending"],
            &[("INBOX_URL", &url), ("INBOX_SECRET_KEY_FILE", "alice.key")],
        );
        served.join().expect("the stand-in ends");
        assert_eq!(pending.status, exit, "{status}: {pending:#?}");
        wrong.extend(control_in(&format!("{status}: stderr"), &pending.stderr));
        wrong.extend(control_in(&format!("{status}: stdout"), &pending.stdout));
        if !pending.stderr.contains(shown) {
            wrong.push(format!(
                "{status}: stderr does not show {shown:?}: {:?}",
                pending.stderr
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// With `--json` no control character is raw in the output (JSON's own escapes cover C0, a `\u`
/// escape DEL and C1), and a JSON reader gets the body and the reply as they were sent.
#[test]
fn with_json_no_control_character_is_raw_and_the_value_is_unchanged() {
    let world = World::new("control-json");
    let answered = requested(&world, HOSTILE);
    world
        .client("bob", &["answer", &answered, "--body", HOSTILE])
        .ok();
    let outbox = world.client("alice", &["outbox", "--json"]).ok();
    let collected = world
        .client("alice", &["collect", &answered, "--json"])
        .ok();
    let wrong: Vec<String> = [("outbox --json", &outbox), ("collect --json", &collected)]
        .into_iter()
        .filter_map(|(what, run)| control_in(what, &run.stdout))
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    let row = &rows(&outbox.json())[0];
    assert_eq!(text(row, "body"), HOSTILE);
    assert_eq!(
        text(row.member("reply").unwrap_or(&Value::Null), "body"),
        HOSTILE
    );
    let document = collected.json();
    let message = document.member("message").unwrap_or(&Value::Null);
    assert_eq!(text(message, "from"), ALICE);
    assert_eq!(text(message, "body"), HOSTILE);
    assert_eq!(
        text(message.member("reply").unwrap_or(&Value::Null), "body"),
        HOSTILE
    );
}

/// A body without control characters, with text outside ASCII, an empty line and a line end at
/// its end, is printed byte for byte as before: each line indented by 4 spaces.
#[test]
fn a_body_without_control_characters_is_printed_as_before() {
    let world = World::new("control-plain");
    let body = "Grüße, 世界 — \\u{1b} as text\nzweite Zeile\n\nEnde\n";
    let answered = requested(&world, "Welche Farbe? äöü");
    world
        .client("bob", &["answer", &answered, "--body", body])
        .ok();
    let collected = world.client("alice", &["collect", &answered]).ok();
    assert!(
        collected
            .stdout
            .contains("\n    Welche Farbe? äöü\n  reply "),
        "{:?}",
        collected.stdout
    );
    assert!(
        collected.stdout.ends_with(&format!(
            " from {BOB}:\n    Grüße, 世界 — \\u{{1b}} as text\n    zweite Zeile\n    \n    Ende\n"
        )),
        "{:?}",
        collected.stdout
    );
}
