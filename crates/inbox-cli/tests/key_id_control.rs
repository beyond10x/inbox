//! story:a-key-id-with-a-control-character-is-refused-where-it-is-made, `inbox key new`: a key id
//! with a control character (`char::is_control`: C0, DEL, C1) is wrong usage before a key is made
//! and before a file exists; a key id with a line separator or a direction character is still
//! taken. The client's side, a key file that holds such a key id, is
//! `adversary_acting_for::a_line_break_in_the_key_id_opens_no_connection`. A line shows the key
//! id cut to its first 64 characters. The host of `INBOX_URL` with a control character is wrong
//! usage before it is resolved.

use crate::support;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use inbox::server::json;
use support::{ALICE, Dir, LOCAL, USAGE, run, text};

/// A stand-in on a loopback address that counts the connections it accepted and answers each
/// `401` once its head arrived (or the connection ended).
fn counting() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("binds");
    let url = format!("http://{}", listener.local_addr().expect("an address"));
    let count = Arc::new(AtomicUsize::new(0));
    let kept = Arc::clone(&count);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            kept.fetch_add(1, Ordering::SeqCst);
            let mut received = Vec::new();
            let mut buffer = [0_u8; 4096];
            while !received.windows(4).any(|window| window == b"\r\n\r\n") {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => received.extend_from_slice(&buffer[..read]),
                }
            }
            let body = r#"{"refused":"unauthenticated"}"#;
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 401 Stand-in\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            );
        }
    });
    (url, count)
}

/// The connections the stand-in accepted, after a pause for one that is still on its way.
fn connections(count: &AtomicUsize) -> usize {
    std::thread::sleep(std::time::Duration::from_millis(150));
    count.load(Ordering::SeqCst)
}

/// A client key made by `inbox key new` with the key id `k-1`, copied to `file` (mode 0600) with
/// its key id replaced by `key_id`, written as `inbox key new` writes a text.
fn key_file_with_key_id(dir: &Dir, file: &str, key_id: &str) {
    if !dir.join("made.key").exists() {
        support::key_new(&dir.path, "made.key", Some(("k-1", ALICE)));
    }
    let made = std::fs::read_to_string(dir.join("made.key")).expect("the key file");
    assert!(made.contains(r#""key_id":"k-1","#), "{made:?}");
    let mut member = String::from(r#""key_id":"#);
    json::push_text(&mut member, key_id);
    member.push(',');
    std::fs::copy(dir.join("made.key"), dir.join(file)).expect("the key file is copied");
    std::fs::write(dir.join(file), made.replace(r#""key_id":"k-1","#, &member))
        .expect("the key id is replaced");
}

/// The one line on standard error without its end; it holds no raw control character.
fn one_line(stderr: &str, what: &str) -> String {
    let line = stderr.strip_suffix('\n').unwrap_or(stderr);
    assert!(
        !line.contains(char::is_control),
        "{what}: one line, no raw control character: {stderr:?}"
    );
    line.to_owned()
}

/// A key file whose key id is 1 MiB of `k` and a control character at its end: exit 4 and one
/// line under 1024 bytes that names the file, the cause and the remedy and shows the first 64
/// characters of the key id and how many more there are; no connection.
#[test]
fn the_line_of_a_key_file_with_a_key_id_of_one_mib_is_short() {
    let dir = Dir::new("key-id-mib");
    let (url, count) = counting();
    let long = format!("{}\u{1}", "k".repeat(1 << 20));
    key_file_with_key_id(&dir, "long.key", &long);
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "long.key"),
        ("INBOX_OUTBOX", "alice.outbox"),
    ];
    let ran = run(&dir.path, &["pending"], &env);
    assert_eq!(ran.status, LOCAL, "exit {}", ran.status);
    assert!(ran.stderr.len() < 1024, "{} bytes", ran.stderr.len());
    let line = one_line(&ran.stderr, "long");
    assert!(line.contains("long.key"), "{line:?}");
    assert!(line.contains("control character"), "{line:?}");
    assert!(line.contains("`inbox key new --key-id"), "{line:?}");
    let shown = format!("{:?} (and 1048513 more characters) holds", "k".repeat(64));
    assert!(line.contains(&shown), "{line:?}");
    assert_eq!(connections(&count), 0, "no connection");
}

/// A key id of 64 characters is shown whole and one of 65 is cut to its first 64, in the line of
/// `inbox key new` and in the line of a key file alike; the cut is made before the escape, so no
/// escape is cut in half.
#[test]
fn a_key_id_of_64_characters_is_shown_whole_and_one_of_65_is_cut() {
    let dir = Dir::new("key-id-64");
    let (url, count) = counting();
    let whole = format!("k{}", "\u{1b}".repeat(63));
    let cut = format!("k{}", "\u{1b}".repeat(64));
    let first = format!("k{}", "\u{1b}".repeat(63));
    for (n, (key_id, shown)) in [
        (&whole, format!("{whole:?} holds")),
        (&cut, format!("{first:?} (and 1 more character) holds")),
    ]
    .into_iter()
    .enumerate()
    {
        let out = format!("new{n}.key");
        let made = run(
            &dir.path,
            &[
                "key",
                "new",
                "--out",
                &out,
                "--key-id",
                key_id,
                "--principal",
                ALICE,
            ],
            &[],
        );
        assert_eq!(made.status, USAGE, "{n}: {made:#?}");
        let line = one_line(&made.stderr, "key new");
        assert!(line.contains(&format!("--key-id {shown}")), "{n}: {line:?}");
        let file = format!("held{n}.key");
        key_file_with_key_id(&dir, &file, key_id);
        let env = [
            ("INBOX_URL", url.as_str()),
            ("INBOX_SECRET_KEY_FILE", file.as_str()),
            ("INBOX_OUTBOX", "alice.outbox"),
        ];
        let ran = run(&dir.path, &["pending"], &env);
        assert_eq!(ran.status, LOCAL, "{n}: {ran:#?}");
        let line = one_line(&ran.stderr, "key file");
        assert!(
            line.contains(&format!("its key id {shown}")),
            "{n}: {line:?}"
        );
    }
    assert_eq!(connections(&count), 0, "no connection");
}

/// `inbox key new` with a key id of 100,000 characters (an argument of Linux may be up to 128 KiB)
/// that ends with a control character: exit 2, one line under 1024 bytes, no file.
#[test]
fn key_new_with_a_long_key_id_writes_a_short_line() {
    let dir = Dir::new("key-id-long-new");
    let long = format!("{}\u{1}", "k".repeat(99_999));
    let ran = run(
        &dir.path,
        &[
            "key",
            "new",
            "--out",
            "long.key",
            "--key-id",
            &long,
            "--principal",
            ALICE,
        ],
        &[],
    );
    assert_eq!(ran.status, USAGE, "exit {}", ran.status);
    assert!(ran.stderr.len() < 1024, "{} bytes", ran.stderr.len());
    let line = one_line(&ran.stderr, "long");
    assert!(line.contains("--key-id"), "{line:?}");
    assert!(line.contains("(and 99936 more characters)"), "{line:?}");
    assert!(!dir.join("long.key").exists(), "a file was written");
}

/// A line break, another C0 character, DEL and two C1 characters in `--key-id`: exit 2, one line
/// that names `--key-id` with no raw character of the value, nothing on standard output, and no
/// file at `--out`.
#[test]
fn key_new_refuses_a_key_id_with_a_control_character_and_writes_no_file() {
    let dir = Dir::new("key-id-control");
    for (n, key_id) in [
        "k\r\nX-Injected: 1",
        "k\n",
        "k\u{1b}[2J",
        "k\u{7f}",
        "k\u{85}",
        "k\u{9b}31m",
    ]
    .into_iter()
    .enumerate()
    {
        let out = format!("k{n}.key");
        let ran = run(
            &dir.path,
            &[
                "key",
                "new",
                "--out",
                &out,
                "--key-id",
                key_id,
                "--principal",
                ALICE,
            ],
            &[],
        );
        assert_eq!(ran.status, USAGE, "{key_id:?}: {ran:#?}");
        assert!(ran.stdout.is_empty(), "{key_id:?}: {ran:#?}");
        assert_eq!(ran.stderr.lines().count(), 1, "{key_id:?}: {ran:#?}");
        assert!(ran.stderr.contains("--key-id"), "{key_id:?}: {ran:#?}");
        let raw: Vec<char> = key_id
            .chars()
            .filter(|c| c.is_control() && ran.stderr.trim_end_matches('\n').contains(*c))
            .collect();
        assert!(
            raw.is_empty(),
            "{key_id:?}: raw {raw:?} in {:?}",
            ran.stderr
        );
        assert!(
            !dir.join(&out).exists(),
            "{key_id:?}: a file was written: {ran:#?}"
        );
    }
}

/// U+2028, U+2029 and a right-to-left override are no control characters
/// (`char::is_control`): `inbox key new` takes them and writes them into the file as given.
#[test]
fn key_new_takes_a_key_id_with_a_line_separator_or_a_direction_character() {
    let dir = Dir::new("key-id-separator");
    for (n, key_id) in ["k\u{2028}1", "k\u{2029}1", "k\u{202e}1"]
        .into_iter()
        .enumerate()
    {
        let out = format!("k{n}.key");
        let ran = run(
            &dir.path,
            &[
                "key",
                "new",
                "--out",
                &out,
                "--key-id",
                key_id,
                "--principal",
                ALICE,
            ],
            &[],
        );
        assert_eq!(ran.status, 0, "{key_id:?}: {ran:#?}");
        let stored = std::fs::read_to_string(dir.join(&out)).expect("the key file");
        let value = json::parse(&stored).expect("the key file is JSON");
        assert_eq!(text(&value, "key_id"), key_id);
    }
}

/// A host of `INBOX_URL` or `--url` with a control character in the name or the port, and a long
/// one, is wrong usage: exit 2 and one line that names `INBOX_URL`, shows the host escaped and cut,
/// and holds no raw control character; the key file (here none) is not read. The check reads the
/// text before the host reaches the resolver (`client::tests::a_host_with_a_control_character_is_wrong_usage`),
/// so no name of this case is resolved; it runs only with the check in place.
#[test]
fn a_host_of_the_url_with_a_control_character_is_wrong_usage() {
    let dir = Dir::new("url-host-control");
    let long = format!("{}\u{1}.invalid:7700", "h".repeat(1 << 16));
    for host in [
        "in\tbox.invalid:7700",
        "in\u{1b}[2Jbox.invalid:7700",
        "inbox.invalid\u{85}:7700",
        "inbox.invalid:77\u{7f}00",
        "inbox.invalid:7700\r\n/",
        long.as_str(),
    ] {
        let url = format!("http://{host}");
        let cut: String = host.chars().take(64).collect();
        for ran in [
            run(
                &dir.path,
                &["pending"],
                &[
                    ("INBOX_URL", url.as_str()),
                    ("INBOX_SECRET_KEY_FILE", "missing.key"),
                ],
            ),
            run(
                &dir.path,
                &["pending", "--url", &url, "--secret-key-file", "missing.key"],
                &[],
            ),
        ] {
            assert_eq!(ran.status, USAGE, "{host:?}: {ran:#?}");
            assert!(ran.stderr.len() < 1024, "{} bytes", ran.stderr.len());
            let line = one_line(&ran.stderr, host);
            assert!(line.contains("INBOX_URL: its host"), "{line:?}");
            assert!(line.contains(&format!("{cut:?}")), "{line:?}");
            assert!(line.contains("control character"), "{line:?}");
        }
    }
}
