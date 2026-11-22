//! Adversary cases of story:the-client-prints-no-control-sequence-of-another-principal: every
//! control character but the line end (C0, DEL, C1) shown by its own escape in text and read back
//! unchanged from `--json`; every field of a row a service sets; the failure documents of
//! `--json` (a refusal, a transport failure, a reply printed after the outbox file failed); an
//! answer that is not JSON or not UTF-8; one body at the model's limit. Every key is made at run
//! time with the binary itself.

use crate::support;

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::Duration;

use inbox::server::json::{self, Value};

use support::{ALICE, BOB, LOCAL, REFUSED, TRANSPORT, World, keys_only, rows, run, stand_in, text};

/// The control characters the story names, written out, not taken from `char::is_control`: C0
/// without the line end, DEL, C1.
fn controls() -> Vec<char> {
    (0_u32..0x20)
        .chain(0x7f..0xa0)
        .filter(|&code| code != 0x0a)
        .filter_map(char::from_u32)
        .collect()
}

/// The characters of `output` the story forbids raw (every one of [`controls`]).
fn raw(output: &str) -> Vec<char> {
    let forbidden = controls();
    output.chars().filter(|c| forbidden.contains(c)).collect()
}

/// The visible form the unit documents for `c`: `\u{1b}`.
fn shown(c: char) -> String {
    format!("\\u{{{:x}}}", u32::from(c))
}

/// Gives a directory back its owner's write bit when dropped, so the test directory can go.
struct Writable(PathBuf);

impl Drop for Writable {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o700));
    }
}

fn member<'a>(value: &'a Value, path: &[&str]) -> &'a Value {
    path.iter().fold(value, |at, name| {
        at.member(name)
            .unwrap_or_else(|| panic!("no `{name}` in {at:?}"))
    })
}

/// Alice posts a free request with the body in the file `file` to bob; answers its id.
fn post_file(world: &World, file: &str, env: &[(&str, &str)]) -> String {
    let posted = world
        .client_env(
            "alice",
            &[
                "post",
                "--to",
                BOB,
                "--kind",
                "request",
                "--reply-shape",
                "free",
                "--body-file",
                file,
                "--json",
            ],
            env,
        )
        .ok()
        .json();
    text(&posted, "message_id")
}

/// Every one of the 65 control characters but the line end, NUL included (it cannot travel on a
/// command line, so the body comes from a file), reaches `pending` as its own escape, in order,
/// and `pending --json` reads back the body as it was sent with none of them raw.
#[test]
fn every_control_character_but_the_line_end_is_shown_by_its_own_escape() {
    let world = World::new("adv-ctl-every");
    world.client("bob", &["open"]).ok();
    let body: String = std::iter::once('<')
        .chain(controls())
        .chain(std::iter::once('>'))
        .collect();
    fs::write(world.dir.join("every.txt"), &body).expect("the body file");
    post_file(&world, "every.txt", &[]);

    let expected: String = std::iter::once("<".to_owned())
        .chain(controls().into_iter().map(shown))
        .chain(std::iter::once(">".to_owned()))
        .collect();
    let pending = world.client("bob", &["pending"]).ok();
    assert!(
        raw(&pending.stdout).is_empty(),
        "pending printed {:?} raw: {:?}",
        raw(&pending.stdout),
        pending.stdout
    );
    assert!(
        pending.stdout.ends_with(&format!("\n    {expected}\n")),
        "pending does not end with {expected:?}: {:?}",
        pending.stdout
    );

    let json = world.client("bob", &["pending", "--json"]).ok();
    assert!(raw(&json.stdout).is_empty(), "{:?}", json.stdout);
    assert_eq!(text(&rows(&json.json())[0], "body"), body);
}

/// The service refuses with a `refused` text holding C1 characters raw on the wire and DEL and
/// NEL as JSON escapes, then answers 302 with an 8-bit OSC 8 hyperlink. With `--json` neither
/// failure document holds one of them raw, and a JSON reader gets the service's text unchanged.
#[test]
fn with_json_a_refusal_and_a_transport_failure_hold_no_raw_control_character() {
    let (dir, _) = keys_only("adv-ctl-json-failure");
    let refused_text = "no\u{9b}31m\u{7f}\u{85}\u{90}x\u{9c}";
    let link = "\u{9d}8;;http://x\u{9c}\u{7f}";
    let answers: [(u16, &'static str, i32); 2] = [
        (
            401,
            "{\"refused\":\"no\u{9b}31m\\u007f\\u0085\u{90}x\u{9c}\"}",
            REFUSED,
        ),
        (
            302,
            "{\"moved\":\"\u{9d}8;;http://x\u{9c}\\u007f\"}",
            TRANSPORT,
        ),
    ];
    let mut wrong = Vec::new();
    for (status, body, exit) in answers {
        let (url, served) = stand_in(status, body);
        let pending = run(
            &dir.path,
            &["pending", "--json"],
            &[("INBOX_URL", &url), ("INBOX_SECRET_KEY_FILE", "alice.key")],
        );
        served.join().expect("the stand-in ends");
        assert_eq!(pending.status, exit, "{status}: {pending:#?}");
        for (stream, output) in [("stdout", &pending.stdout), ("stderr", &pending.stderr)] {
            if !raw(output).is_empty() {
                wrong.push(format!(
                    "{status}: {stream} holds {:?} raw: {output:?}",
                    raw(output)
                ));
            }
        }
        let document = pending.json();
        match status {
            401 => {
                if text(&document, "error") != refused_text
                    || text(member(&document, &["answer"]), "refused") != refused_text
                {
                    wrong.push(format!("401: the refusal reads {document:?}"));
                }
            }
            _ => {
                let detail = text(&document, "detail");
                if !detail.contains(&format!("{{\"moved\":\"{link}\"}}")) {
                    wrong.push(format!("302: the detail reads {detail:?}"));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// `collect` takes a reply off the service, then cannot change the outbox file: the reply is
/// printed beside the local failure (exit 4). In text it is shown escaped; with `--json` no
/// control character is raw and the reply reads back as sent.
#[test]
fn a_reply_printed_after_the_outbox_file_failed_shows_no_raw_control_character() {
    let hostile = "Blue\u{1b}[2J\u{1b}]0;owned\u{7}\rRed\u{9b}31m\u{7f}\tend\r\nnext";
    let visible = "    Blue\\u{1b}[2J\\u{1b}]0;owned\\u{7}\\u{d}Red\\u{9b}31m\\u{7f}\\u{9}end\\u{d}\n    next\n";
    let world = World::new("adv-ctl-after");
    let sub = world.dir.join("state-ro");
    fs::create_dir_all(&sub).expect("the outbox directory");
    let outbox = sub.join("alice.outbox").display().to_string();
    let env = [("INBOX_OUTBOX", outbox.as_str())];
    world.client("bob", &["open"]).ok();
    fs::write(world.dir.join("ask.txt"), "Which colour?").expect("the body file");
    let first = post_file(&world, "ask.txt", &env);
    let second = post_file(&world, "ask.txt", &env);
    for id in [&first, &second] {
        world.client("bob", &["answer", id, "--body", hostile]).ok();
    }

    let _writable = Writable(sub.clone());
    fs::set_permissions(&sub, fs::Permissions::from_mode(0o500)).expect("read-only");
    let mut wrong = Vec::new();
    let text_run = world.client_env("alice", &["collect", &first], &env);
    assert_eq!(text_run.status, LOCAL, "{text_run:#?}");
    if !raw(&text_run.stdout).is_empty() || !text_run.stdout.contains(visible) {
        wrong.push(format!(
            "collect after a local failure printed {:?}",
            text_run.stdout
        ));
    }
    let json_run = world.client_env("alice", &["collect", &second, "--json"], &env);
    assert_eq!(json_run.status, LOCAL, "{json_run:#?}");
    if !raw(&json_run.stdout).is_empty() {
        wrong.push(format!(
            "collect --json after a local failure holds {:?} raw: {:?}",
            raw(&json_run.stdout),
            json_run.stdout
        ));
    }
    let document = json_run.json();
    assert_eq!(
        text(member(&document, &["done", "message", "reply"]), "body"),
        hostile
    );
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// A service answers `pending` with 2 rows in which every member the text printer shows holds an
/// OSC title, a bell and a C1 control sequence introducer, and answers `read` with such an
/// outcome. No byte of the text output is a control character but the line end, and each member
/// is shown escaped.
#[test]
fn every_member_a_service_sets_reaches_the_terminal_escaped() {
    let (dir, _) = keys_only("adv-ctl-every-member");
    let value = |n: u32| format!("f{n}\u{1b}]0;t\u{7}\u{9b}");
    let seen = |n: u32| format!("f{n}\\u{{1b}}]0;t\\u{{7}}\\u{{9b}}");
    let text_value = |out: &mut String, name: &str, n: u32| {
        json::member(out, name);
        json::push_text(out, &value(n));
    };
    let mut answer = String::from("{\"rows\":[{");
    for (n, name) in [
        "message_id",
        "kind",
        "state",
        "from",
        "to",
        "thread",
        "expires",
        "on_behalf_of",
        "body",
    ]
    .into_iter()
    .enumerate()
    {
        text_value(&mut answer, name, n as u32);
    }
    answer.push_str(",\"decision\":{\"options\":[{");
    text_value(&mut answer, "letter", 9);
    text_value(&mut answer, "text", 10);
    answer.push_str("}]},\"reply\":{");
    text_value(&mut answer, "reply_id", 11);
    text_value(&mut answer, "from", 12);
    text_value(&mut answer, "option", 13);
    text_value(&mut answer, "body", 14);
    answer.push('}');
    text_value(&mut answer, "refusal", 15);
    answer.push_str("},{");
    text_value(&mut answer, "message_id", 16);
    text_value(&mut answer, "decided_option", 17);
    answer.push_str("}]}");
    let answer: &'static str = Box::leak(answer.into_boxed_str());

    let (url, served) = stand_in(200, answer);
    let pending = run(
        &dir.path,
        &["pending"],
        &[("INBOX_URL", &url), ("INBOX_SECRET_KEY_FILE", "alice.key")],
    );
    served.join().expect("the stand-in ends");
    assert_eq!(pending.status, 0, "{pending:#?}");
    let mut wrong: Vec<String> = Vec::new();
    if !raw(&pending.stdout).is_empty() {
        wrong.push(format!(
            "pending printed {:?} raw: {:?}",
            raw(&pending.stdout),
            pending.stdout
        ));
    }
    for n in 0..=17 {
        if !pending.stdout.contains(&seen(n)) {
            wrong.push(format!("member {n} is not shown as {:?}", seen(n)));
        }
    }

    let (url, served) = stand_in(
        200,
        "{\"outcome\":\"marked\\u001b]0;t\\u0007\u{9b}\",\"published\":[]}",
    );
    let read = run(
        &dir.path,
        &["read", "01JGT0AE000000000000000001"],
        &[("INBOX_URL", &url), ("INBOX_SECRET_KEY_FILE", "alice.key")],
    );
    served.join().expect("the stand-in ends");
    assert_eq!(read.status, 0, "{read:#?}");
    if read.stdout != "marked\\u{1b}]0;t\\u{7}\\u{9b} 01JGT0AE000000000000000001\n" {
        wrong.push(format!("read printed {:?}", read.stdout));
    }
    assert!(
        wrong.is_empty(),
        "{}\n{:?}",
        wrong.join("\n"),
        pending.stdout
    );
}

/// A stand-in that reads one request head and answers it with the raw bytes `answer`.
fn raw_stand_in(answer: &'static [u8]) -> (String, std::thread::JoinHandle<()>) {
    raw_stand_in_within(answer, support::STAND_IN_WAIT)
}

/// [`raw_stand_in`] with the wait for the request given: the case that holds the bound passes 1 s.
fn raw_stand_in_within(
    answer: &'static [u8],
    wait: Duration,
) -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
    let url = format!("http://{}", listener.local_addr().expect("an address"));
    let handle = std::thread::spawn(move || {
        let mut stream = support::accept_or_panic(
            &listener,
            wait,
            "the raw stand-in of adversary_control_sequences.rs",
            "the request of `pending`",
        );
        let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
        let mut received = Vec::new();
        let mut buffer = [0_u8; 4096];
        while !received.windows(4).any(|window| window == b"\r\n\r\n") {
            match stream.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => received.extend_from_slice(&buffer[..read]),
            }
        }
        let _ = stream.write_all(answer);
    });
    (url, handle)
}

/// A raw stand-in whose client never connects ends with a message that names it and the missing
/// request, and does not wait for it forever: the case that joins it fails with that message. The
/// wait is 1 s here, named in the message, and 10 s for every other case; the case waits for the
/// thread to end with `STAND_IN_WAIT` only as the end of a hang (it held the join to 3 s before,
/// a time a slow host can pass: story:no-test-holds-a-wall-clock-time).
#[test]
fn a_raw_stand_in_whose_client_never_connects_ends_and_names_the_request() {
    let (_url, served) = raw_stand_in_within(b"", Duration::from_secs(1));
    support::wait_until_finished(&served, "the raw stand-in");
    let message = match served.join() {
        Ok(()) => panic!("the raw stand-in ended and said nothing"),
        Err(panic) => panic.downcast_ref::<String>().cloned().unwrap_or_default(),
    };
    assert_eq!(
        message,
        "the raw stand-in of adversary_control_sequences.rs received no connection for the \
         request of `pending` within 1 s"
    );
}

/// Answers that are not UTF-8, not JSON (a lone surrogate escape), or carry control characters in
/// the status line: the line on standard error holds none of them raw, and neither does the
/// `--json` document.
#[test]
fn an_answer_that_is_not_utf8_or_not_json_is_named_without_a_raw_control_character() {
    let (dir, _) = keys_only("adv-ctl-not-json");
    let answers: [&'static [u8]; 3] = [
        b"HTTP/1.1 200 X\r\nContent-Length: 5\r\n\r\n\x1b[2J\xff",
        b"HTTP/1.1 200 X\r\nContent-Length: 29\r\n\r\n{\"rows\":[{\"body\":\"\\ud800\x1b\"}]}",
        b"HTTP/1.1 \x1b]0;t\x07 X\r\nContent-Length: 2\r\n\r\n{}",
    ];
    let mut wrong = Vec::new();
    for (at, answer) in answers.into_iter().enumerate() {
        for json in [false, true] {
            let (url, served) = raw_stand_in(answer);
            let mut args = vec!["pending"];
            if json {
                args.push("--json");
            }
            let pending = run(
                &dir.path,
                &args,
                &[("INBOX_URL", &url), ("INBOX_SECRET_KEY_FILE", "alice.key")],
            );
            served.join().expect("the stand-in ends");
            if pending.status != TRANSPORT {
                wrong.push(format!("answer {at} json {json}: {pending:#?}"));
            }
            for (stream, output) in [("stdout", &pending.stdout), ("stderr", &pending.stderr)] {
                if !raw(output).is_empty() {
                    wrong.push(format!(
                        "answer {at} json {json}: {stream} holds {:?} raw: {output:?}",
                        raw(output)
                    ));
                }
            }
            if json {
                let _ = pending.json();
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// One body at the model's limit, 65536 escape characters: `pending` shows each as `\u{1b}`
/// (7 bytes each), none raw. The case holds no time (it held `pending` to 20 s before, a time a
/// slow host can pass: story:no-test-holds-a-wall-clock-time); the runner's bound of 60 s ends a
/// print that never finishes.
#[test]
fn one_body_of_escape_characters_at_the_limit_is_shown_whole() {
    let world = World::new("adv-ctl-large");
    world.client("bob", &["open"]).ok();
    let body = "\u{1b}".repeat(65_536);
    fs::write(world.dir.join("large.txt"), &body).expect("the body file");
    post_file(&world, "large.txt", &[]);
    let pending = world.client("bob", &["pending"]).ok();
    assert!(raw(&pending.stdout).is_empty());
    assert!(
        pending
            .stdout
            .ends_with(&format!("\n    {}\n", "\\u{1b}".repeat(65_536))),
        "the body is not shown whole: {} bytes",
        pending.stdout.len()
    );
}

/// A body with a line end at its end, with its reply printed under it, is printed byte for byte
/// as the client printed it before the unit (the binary at the commit before it gives the same
/// lines): one indented line per line, and no empty indented line for the closing line end.
#[test]
fn a_body_ending_in_a_line_end_before_its_reply_is_printed_as_before() {
    let world = World::new("adv-ctl-trailing");
    world.client("bob", &["open"]).ok();
    fs::write(world.dir.join("ask.txt"), "Frage mit Zeilenende\n").expect("the body file");
    let id = post_file(&world, "ask.txt", &[]);
    world
        .client("bob", &["answer", &id, "--body", "Blau\nmit Zeilenende\n"])
        .ok();
    let collected = world.client("alice", &["collect", &id]).ok();
    let head =
        format!("collected {id}\n{id} request Answered from {ALICE} to {BOB} thread {id} expires ");
    assert!(
        collected.stdout.starts_with(&head),
        "{:?}",
        collected.stdout
    );
    assert!(
        collected
            .stdout
            .contains("Z\n    Frage mit Zeilenende\n  reply "),
        "{:?}",
        collected.stdout
    );
    assert!(
        collected
            .stdout
            .ends_with(&format!(" from {BOB}:\n    Blau\n    mit Zeilenende\n")),
        "{:?}",
        collected.stdout
    );
}
