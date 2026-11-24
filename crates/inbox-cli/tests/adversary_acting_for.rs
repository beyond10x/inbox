//! Adversary pass 1 on story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal, the
//! client's `--for`: a value the service can never honour is wrong usage before any request,
//! and the client sends the value it signs, byte for byte. Every key is made when the case runs.

use crate::support;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use support::{ALICE, BOB, USAGE, keys_only, run};

const UNAUTHENTICATED: &str = r#"{"refused":"unauthenticated"}"#;

/// Reads one request from `stream` and hands back its head as received, not lower-cased.
fn raw_head(stream: &mut TcpStream) -> String {
    let mut received = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        if let Some(end) = received.windows(4).position(|window| window == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&received[..end]).into_owned();
            let length: usize = head
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse().ok())?
                })
                .unwrap_or(0);
            if received.len() >= end + 4 + length {
                return head;
            }
        }
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => return String::from_utf8_lossy(&received).into_owned(),
            Ok(read) => received.extend_from_slice(&buffer[..read]),
        }
    }
}

/// A stand-in that answers every request `401` at once and keeps each head as received.
fn recording() -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("binds");
    let url = format!("http://{}", listener.local_addr().expect("an address"));
    let heads = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&heads);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let head = raw_head(&mut stream);
            kept.lock().expect("not poisoned").push(head);
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 401 Stand-in\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{UNAUTHENTICATED}",
                    UNAUTHENTICATED.len()
                )
                .as_bytes(),
            );
        }
    });
    (url, heads)
}

/// Runs `pending`, `post` and `answer` with `--for value` against a recording stand-in, and
/// hands back, per run, the value, the exit status and the heads the stand-in received.
fn sent_for(name: &str, values: &[&str]) -> Vec<(String, i32, Vec<String>)> {
    let (dir, _) = keys_only(name);
    let (url, heads) = recording();
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "alice.key"),
        ("INBOX_OUTBOX", "alice.outbox"),
    ];
    let mut seen = Vec::new();
    for value in values {
        for command in [
            vec!["pending", "--for", value],
            vec!["post", "--to", BOB, "--body", "x", "--for", value],
            vec![
                "answer",
                "01J0000000000000000000000A",
                "--body",
                "x",
                "--for",
                value,
            ],
        ] {
            let taken = heads.lock().expect("not poisoned").len();
            let ran = run(&dir.path, &command, &env);
            let sent = heads.lock().expect("not poisoned")[taken..].to_vec();
            seen.push((format!("{value:?} {}", command[0]), ran.status, sent));
        }
    }
    seen
}

/// Asserts that each run ended as wrong usage and sent nothing.
fn usage_and_nothing_sent(seen: &[(String, i32, Vec<String>)]) {
    let wrong: Vec<&(String, i32, Vec<String>)> = seen
        .iter()
        .filter(|(_, status, sent)| *status != USAGE || !sent.is_empty())
        .collect();
    assert!(
        wrong.is_empty(),
        "sent or not wrong usage (value, status, heads): {wrong:#?}"
    );
}

/// A line break in `--for` ends the header line on the wire: the client signs one text and the
/// service reads another (and the rest as a header of its own). It is wrong usage before any
/// request, as an empty or padded value is.
#[test]
fn a_line_break_in_for_is_wrong_usage_and_sends_nothing() {
    let injected = format!("{BOB}\r\nX-Injected: 1");
    let lf = format!("{BOB}\nX-Injected: 1");
    let cr = format!("{BOB}\rX");
    usage_and_nothing_sent(&sent_for("adv-for-crlf", &[&injected, &lf, &cr]));
}

/// A control character in `--for` names no inbox the service can be asked for honestly: wrong
/// usage before any request.
#[test]
fn a_control_character_in_for_is_wrong_usage_and_sends_nothing() {
    let values = [
        format!("{BOB}\u{1}"),
        format!("{BOB}\u{1b}[2J"),
        format!("{BOB}\u{7f}"),
    ];
    let values: Vec<&str> = values.iter().map(String::as_str).collect();
    usage_and_nothing_sent(&sent_for("adv-for-control", &values));
}

/// The service trims a header value of every white space at its ends (`str::trim`), not only of
/// a space and a tab: a value that ends in another white space is signed with it and read
/// without it, so the service never honours it, as it never honours a value padded with a
/// space. Wrong usage before any request.
#[test]
fn a_value_the_service_trims_is_wrong_usage_and_sends_nothing() {
    let values = [
        format!("{BOB}\u{a0}"),
        format!("\u{3000}{BOB}"),
        format!("{BOB}\u{b}"),
        format!("{BOB}\u{c}"),
    ];
    let values: Vec<&str> = values.iter().map(String::as_str).collect();
    usage_and_nothing_sent(&sent_for("adv-for-trimmed", &values));
}

/// `--for` given twice is wrong usage before any request.
#[test]
fn for_given_twice_is_wrong_usage_and_sends_nothing() {
    let (dir, _) = keys_only("adv-for-twice");
    let (url, heads) = recording();
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "alice.key"),
        ("INBOX_OUTBOX", "alice.outbox"),
    ];
    for second in [BOB, ALICE] {
        let ran = run(&dir.path, &["pending", "--for", BOB, "--for", second], &env);
        assert_eq!(ran.status, USAGE, "{second}: {ran:#?}");
    }
    assert!(
        heads.lock().expect("not poisoned").is_empty(),
        "a request was sent"
    );
}

/// Pass 2. A control character outside ASCII (C1: U+0085, U+009B, the one-byte CSI of a
/// terminal) inside the value is a control character too (`char::is_control`): wrong usage
/// before any request, as an ASCII one is.
#[test]
fn a_c1_control_inside_for_is_wrong_usage_and_sends_nothing() {
    let values = [
        format!("{BOB}\u{9b}2J"),
        format!("{ALICE}\u{85}bob"),
        format!("{BOB}\u{9d}x"),
    ];
    let values: Vec<&str> = values.iter().map(String::as_str).collect();
    usage_and_nothing_sent(&sent_for("adv-for-c1", &values));
}

/// Pass 2. The check of `--for` comes before the service, the key file and the outbox are read:
/// a refused value is exit 2 with the line that names `--for`, also when no URL is given, the key
/// file is missing, and no outbox, home or state directory is named.
#[test]
fn a_wrong_for_is_named_before_the_url_the_key_and_the_outbox() {
    let dir = support::Dir::new("adv-for-first");
    let (url, heads) = recording();
    for env in [
        vec![],
        vec![("INBOX_URL", url.as_str())],
        vec![
            ("INBOX_URL", url.as_str()),
            ("INBOX_SECRET_KEY_FILE", "missing.key"),
        ],
    ] {
        let mut command = support::command(&dir.path);
        command
            .args(["pending", "--for", ""])
            .envs(env.iter().copied())
            .env_remove("HOME")
            .env_remove("XDG_STATE_HOME");
        let output = command.output().expect("the binary starts");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(USAGE), "{env:?}: {stderr}");
        assert!(stderr.contains("--for"), "{env:?}: {stderr}");
    }
    assert!(
        heads.lock().expect("not poisoned").is_empty(),
        "a request was sent"
    );
    assert_eq!(
        std::fs::read_dir(&dir.path).expect("the directory").count(),
        0,
        "a file was written"
    );
}

/// Pass 2. The line of the refusal names the value on one line, with no character of it that
/// moves the cursor, ends the line or turns the direction of the text on a terminal.
#[test]
fn the_refusal_line_shows_no_raw_character_of_the_value() {
    let (dir, _) = keys_only("adv-for-line");
    let (url, _) = recording();
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "alice.key"),
        ("INBOX_OUTBOX", "alice.outbox"),
    ];
    for value in [
        format!(" {BOB}\u{202e}x"),
        format!("{BOB}\u{2028}"),
        format!("\u{2029}{BOB}\u{1b}[2J"),
        format!("{BOB}\u{9b}31m\u{85}"),
        format!("\u{feff}{BOB}\u{200b}\u{7}"),
    ] {
        let ran = run(&dir.path, &["pending", "--for", &value], &env);
        assert_eq!(ran.status, USAGE, "{value:?}: {ran:#?}");
        let raw: Vec<char> = value
            .chars()
            .filter(|c| !c.is_ascii_graphic() && *c != ' ')
            .filter(|c| ran.stderr.contains(*c))
            .collect();
        assert!(raw.is_empty(), "{value:?}: raw {raw:?} in {:?}", ran.stderr);
        assert_eq!(ran.stderr.lines().count(), 1, "{value:?}: {:?}", ran.stderr);
    }
}

/// Pass 2. A key id with a line break, from the caller's own key file, is a header value the
/// client writes: the head is refused before a connection is opened (the stand-in sees no
/// connection at all), with exit 2 and the line that says no request is sent.
#[test]
fn a_line_break_in_the_key_id_opens_no_connection() {
    let dir = support::Dir::new("adv-key-id-crlf");
    let made = run(
        &dir.path,
        &[
            "key",
            "new",
            "--out",
            "broken.key",
            "--key-id",
            "k\r\nX-Injected: 1",
            "--principal",
            ALICE,
        ],
        &[],
    );
    assert_eq!(made.status, 0, "the key is made: {made:#?}");
    let (url, heads) = recording();
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "broken.key"),
        ("INBOX_OUTBOX", "alice.outbox"),
    ];
    for command in [
        vec!["pending"],
        vec!["pending", "--for", BOB],
        vec!["post", "--to", BOB, "--body", "x"],
    ] {
        let ran = run(&dir.path, &command, &env);
        assert_eq!(ran.status, USAGE, "{command:?}: {ran:#?}");
        assert!(ran.stderr.contains("line break"), "{command:?}: {ran:#?}");
    }
    // A connection the client opened and closed without a head is kept as an empty head.
    std::thread::sleep(std::time::Duration::from_millis(200));
    let heads = heads.lock().expect("not poisoned").clone();
    assert!(heads.is_empty(), "connections: {heads:?}");
}

/// The value on the wire is the value of the flag, byte for byte: no case folding, no
/// normalization, no percent-encoding, the name of the header as the design writes it.
#[test]
fn the_header_on_the_wire_is_the_flag_byte_for_byte() {
    let (dir, _) = keys_only("adv-for-bytes");
    let (url, heads) = recording();
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "alice.key"),
        ("INBOX_OUTBOX", "alice.outbox"),
    ];
    // A composed and a decomposed letter, upper case, a percent sign and a colon.
    let value = "Agent:Test/B\u{f3}b%20e\u{301}:x";
    run(&dir.path, &["pending", "--for", value], &env);
    let heads = heads.lock().expect("not poisoned").clone();
    assert!(!heads.is_empty(), "no request");
    for head in &heads {
        let lines: Vec<&str> = head
            .split("\r\n")
            .filter(|line| {
                line.split_once(':')
                    .is_some_and(|(name, _)| name.eq_ignore_ascii_case("inbox-on-behalf-of"))
            })
            .collect();
        assert_eq!(
            lines,
            vec![format!("Inbox-On-Behalf-Of: {value}")],
            "{head}"
        );
    }
}
