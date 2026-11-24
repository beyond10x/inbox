//! Adversary pass 1 on story:a-key-id-with-a-control-character-is-refused-where-it-is-made: the
//! check of `inbox key new` and the check of the secret key file agree on every character, the
//! key file's line names its remedy and shows a direction character as its escape, a refused
//! `key new` leaves an existing file alone, and every client command that reads the key file
//! (`wake` and `bridge` too) opens no connection. Every key is made when the case runs.

use crate::support;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use inbox::server::json;
use support::{ALICE, BOB, Dir, LOCAL, USAGE, run};

const UNAUTHENTICATED: &str = r#"{"refused":"unauthenticated"}"#;

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
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 401 Stand-in\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{UNAUTHENTICATED}",
                    UNAUTHENTICATED.len()
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
/// the member `"key_id":"k-1"` replaced by `member` (the JSON text of the new member, or nothing
/// to drop it).
fn key_file_with(dir: &Dir, file: &str, member: &str) {
    if !dir.join("made.key").exists() {
        support::key_new(&dir.path, "made.key", Some(("k-1", ALICE)));
    }
    let made = std::fs::read_to_string(dir.join("made.key")).expect("the key file");
    assert!(made.contains(r#""key_id":"k-1","#), "{made:?}");
    let replaced = made.replace(r#""key_id":"k-1","#, member);
    std::fs::copy(dir.join("made.key"), dir.join(file)).expect("the key file is copied");
    std::fs::write(dir.join(file), replaced).expect("the member is replaced");
}

/// The member `"key_id":<value>` with `value` written as `inbox key new` writes it.
fn key_id_member(value: &str) -> String {
    let mut member = String::from(r#""key_id":"#);
    json::push_text(&mut member, value);
    member.push(',');
    member
}

/// The line on standard error without its end: exactly one line, no raw control character and
/// no raw direction character.
fn one_clean_line(stderr: &str, what: &str) -> String {
    let line = stderr.strip_suffix('\n').unwrap_or(stderr);
    assert!(
        !line.contains(|c: char| c.is_control()
            || ('\u{202a}'..='\u{202e}').contains(&c)
            || ('\u{2066}'..='\u{2069}').contains(&c)),
        "{what}: one line, no raw control or direction character: {stderr:?}"
    );
    line.to_owned()
}

/// For each C0 character but NUL (an argument cannot hold one), DEL, each C1 character, NBSP and
/// a few characters that are no control characters: `inbox key new` refuses the key id
/// `k<c>` exactly when a client command refuses a key file that holds it, and a key id it took
/// reaches the stand-in. The character is last in the key id, so a check that trims the key id
/// first does not see it.
#[test]
fn key_new_and_the_client_agree_on_each_character_at_the_end_of_the_key_id() {
    let dir = Dir::new("adv-keyid-agree");
    let (url, count) = counting();
    let mut characters: Vec<char> = (1_u32..=0x20)
        .chain(0x7f..=0xa0)
        .filter_map(char::from_u32)
        .collect();
    characters.extend(['\u{2028}', '\u{2029}', '\u{202e}', '\u{200b}', '\u{feff}']);
    let mut connected = 0;
    for (n, character) in characters.into_iter().enumerate() {
        let key_id = format!("k{character}");
        let out = format!("new{n}.key");
        let made = run(
            &dir.path,
            &[
                "key",
                "new",
                "--out",
                &out,
                "--key-id",
                &key_id,
                "--principal",
                ALICE,
            ],
            &[],
        );
        let refused_when_made = match made.status {
            0 => false,
            USAGE => true,
            other => panic!("{key_id:?}: key new exited {other}: {made:#?}"),
        };
        assert_eq!(
            refused_when_made,
            character.is_control(),
            "{key_id:?}: {made:#?}"
        );
        let file = format!("held{n}.key");
        key_file_with(&dir, &file, &key_id_member(&key_id));
        let env = [
            ("INBOX_URL", url.as_str()),
            ("INBOX_SECRET_KEY_FILE", file.as_str()),
            ("INBOX_OUTBOX", "alice.outbox"),
        ];
        let ran = run(&dir.path, &["pending"], &env);
        if refused_when_made {
            assert_eq!(ran.status, LOCAL, "{key_id:?}: {ran:#?}");
            assert!(ran.stderr.contains(&file), "{key_id:?}: {ran:#?}");
            one_clean_line(&ran.stderr, &key_id);
        } else {
            assert_ne!(ran.status, LOCAL, "{key_id:?}: {ran:#?}");
            connected += 1;
        }
    }
    assert_eq!(
        connections(&count),
        connected,
        "one connection per key id taken"
    );
}

/// A key id written with JSON escapes (`\u000a`, `\u0085`, a NUL, an upper-case `\u001B`) is
/// refused as its raw form is: exit 4, one line that names the file, no connection. A surrogate
/// pair is one character that is no control character: it passes.
#[test]
fn a_key_id_with_json_escapes_is_refused_as_its_raw_form() {
    let dir = Dir::new("adv-keyid-escapes");
    let (url, count) = counting();
    for (n, escaped) in [r"k\u000a", r"k\u0085", r"k\u0000", r"k\u001B[1m"]
        .into_iter()
        .enumerate()
    {
        let file = format!("escaped{n}.key");
        key_file_with(&dir, &file, &format!(r#""key_id":"{escaped}","#));
        let env = [
            ("INBOX_URL", url.as_str()),
            ("INBOX_SECRET_KEY_FILE", file.as_str()),
            ("INBOX_OUTBOX", "alice.outbox"),
        ];
        let ran = run(&dir.path, &["pending"], &env);
        assert_eq!(ran.status, LOCAL, "{escaped}: {ran:#?}");
        assert!(ran.stderr.contains(&file), "{escaped}: {ran:#?}");
        assert!(
            ran.stderr.contains("control character"),
            "{escaped}: {ran:#?}"
        );
        one_clean_line(&ran.stderr, escaped);
    }
    assert_eq!(connections(&count), 0, "no connection for a refused key id");
    key_file_with(&dir, "pair.key", r#""key_id":"k😀","#);
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "pair.key"),
        ("INBOX_OUTBOX", "alice.outbox"),
    ];
    let ran = run(&dir.path, &["pending"], &env);
    assert_ne!(ran.status, LOCAL, "a surrogate pair: {ran:#?}");
    assert_eq!(
        connections(&count),
        1,
        "the surrogate pair reaches the stand-in"
    );
}

/// A key file whose key id is no text, is there twice, or holds a control character but has no
/// principal keeps the line and the exit it had before the story: exit 4, the file named, and no
/// line about a control character; no connection.
#[test]
fn a_key_file_with_an_odd_key_id_member_keeps_its_line() {
    let dir = Dir::new("adv-keyid-odd-member");
    let (url, count) = counting();
    for (file, member, said) in [
        (
            "number.key",
            r#""key_id":7,"#,
            "names no key id and principal",
        ),
        (
            "twice.key",
            r#""key_id":"k-1","key_id":"k\n","#,
            "is there twice",
        ),
        ("absent.key", "", "names no key id and principal"),
    ] {
        key_file_with(&dir, file, member);
        let env = [
            ("INBOX_URL", url.as_str()),
            ("INBOX_SECRET_KEY_FILE", file),
            ("INBOX_OUTBOX", "alice.outbox"),
        ];
        let ran = run(&dir.path, &["pending"], &env);
        assert_eq!(ran.status, LOCAL, "{file}: {ran:#?}");
        assert!(ran.stderr.contains(file), "{file}: {ran:#?}");
        assert!(ran.stderr.contains(said), "{file}: {ran:#?}");
        one_clean_line(&ran.stderr, file);
    }
    // No principal beside a key id with a line break: the line of an operator key, as before.
    let made = std::fs::read_to_string(dir.join("made.key")).expect("the key file");
    let principal = format!(r#""principal":"{ALICE}","#);
    assert!(made.contains(&principal), "{made:?}");
    let broken = made
        .replace(r#""key_id":"k-1","#, r#""key_id":"k\n","#)
        .replace(&principal, "");
    std::fs::copy(dir.join("made.key"), dir.join("no-principal.key")).expect("copied");
    std::fs::write(dir.join("no-principal.key"), broken).expect("written");
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "no-principal.key"),
        ("INBOX_OUTBOX", "alice.outbox"),
    ];
    let ran = run(&dir.path, &["pending"], &env);
    assert_eq!(ran.status, LOCAL, "{ran:#?}");
    assert!(
        ran.stderr.contains("names no key id and principal"),
        "{ran:#?}"
    );
    assert_eq!(connections(&count), 0, "no connection");
}

/// The line of the key file names the remedy, `inbox key new --key-id`, and shows a
/// right-to-left override in the key id as its escape; with `--json` the document says `local`
/// and names the file, and standard error still holds the one line.
#[test]
fn the_line_of_the_key_file_names_the_remedy_and_escapes_a_direction_character() {
    let dir = Dir::new("adv-keyid-line");
    let (url, count) = counting();
    key_file_with(&dir, "rlo.key", &key_id_member("k\u{202e}1\u{1b}"));
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "rlo.key"),
        ("INBOX_OUTBOX", "alice.outbox"),
    ];
    let ran = run(&dir.path, &["pending"], &env);
    assert_eq!(ran.status, LOCAL, "{ran:#?}");
    let line = one_clean_line(&ran.stderr, "text");
    assert!(line.contains("rlo.key"), "{line:?}");
    assert!(line.contains(r"\u{202e}"), "{line:?}");
    assert!(
        line.contains("make a client key with `inbox key new --key-id <id> --principal"),
        "{line:?}"
    );
    let ran = run(&dir.path, &["--json", "pending"], &env);
    assert_eq!(ran.status, LOCAL, "{ran:#?}");
    let document = ran.json();
    assert_eq!(support::text(&document, "error"), "local", "{ran:#?}");
    let detail = support::text(&document, "detail");
    assert!(detail.contains("rlo.key"), "{detail:?}");
    assert!(!detail.contains('\u{202e}'), "{detail:?}");
    one_clean_line(&ran.stderr, "json");
    assert_eq!(connections(&count), 0, "no connection");
}

/// `inbox wake` and `inbox bridge` (with and without `--for` and `--authority`) read the key file
/// through the same place: exit 4, their own prefix, the file named, no connection.
#[test]
fn wake_and_bridge_refuse_the_key_file_before_any_connection() {
    let dir = Dir::new("adv-keyid-wake-bridge");
    let (url, count) = counting();
    key_file_with(&dir, "broken.key", &key_id_member("k\u{85}"));
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "broken.key"),
        ("INBOX_OUTBOX", "alice.outbox"),
    ];
    for (command, prefix) in [
        (vec!["wake"], "inbox wake: "),
        (vec!["wake", "--follow"], "inbox wake: "),
        (
            vec![
                "bridge",
                "--chat-channel",
                "C1",
                "--allowed-chat-user",
                "U1",
            ],
            "inbox bridge: ",
        ),
        (
            vec!["bridge", "--for", BOB, "--authority", "d-1"],
            "inbox bridge: ",
        ),
    ] {
        let ran = run(&dir.path, &command, &env);
        assert_eq!(ran.status, LOCAL, "{command:?}: {ran:#?}");
        let line = one_clean_line(&ran.stderr, "wake or bridge");
        assert!(line.starts_with(prefix), "{command:?}: {line:?}");
        assert!(line.contains("broken.key"), "{command:?}: {line:?}");
    }
    assert_eq!(connections(&count), 0, "no connection");
}

/// A refused `inbox key new` leaves an existing `--out` untouched (the key id is refused first,
/// exit 2), and a directory or a path that cannot be written gets the same exit 2.
#[test]
fn a_refused_key_new_leaves_an_existing_file_alone() {
    let dir = Dir::new("adv-keyid-out");
    std::fs::write(dir.join("kept.key"), "kept\n").expect("written");
    std::fs::create_dir(dir.join("adir")).expect("a directory");
    for out in ["kept.key", "adir", "missing/k.key"] {
        let ran = run(
            &dir.path,
            &[
                "key",
                "new",
                "--out",
                out,
                "--key-id",
                "k\r\n",
                "--principal",
                ALICE,
            ],
            &[],
        );
        assert_eq!(ran.status, USAGE, "{out}: {ran:#?}");
        let line = one_clean_line(&ran.stderr, out);
        assert!(line.contains("--key-id"), "{out}: {line:?}");
        assert!(line.contains("no key is made"), "{out}: {line:?}");
    }
    assert_eq!(
        std::fs::read_to_string(dir.join("kept.key")).expect("still there"),
        "kept\n"
    );
    assert!(!dir.join("missing").exists());
}

/// A key id of 1 MiB with a control character: exit 4 and one line. Printed for the report.
/// Since round 2 the line shows the first 64 characters of the key id (`output::quoted_cut`).
#[test]
fn a_key_id_of_one_mib_is_one_line() {
    let dir = Dir::new("adv-keyid-mib");
    let (url, count) = counting();
    let long = format!("{}\u{1}", "k".repeat(1 << 20));
    key_file_with(&dir, "long.key", &key_id_member(&long));
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "long.key"),
        ("INBOX_OUTBOX", "alice.outbox"),
    ];
    let ran = run(&dir.path, &["pending"], &env);
    assert_eq!(ran.status, LOCAL, "exit {}", ran.status);
    let line = one_clean_line(&ran.stderr, "long");
    eprintln!("the line of a key id of 1 MiB: {} bytes", line.len());
    assert_eq!(connections(&count), 0, "no connection");
}

/// The order of the refusals: a wrong `--for` and a wrong URL are wrong usage before the key
/// file is read.
#[test]
fn a_wrong_for_or_url_is_refused_before_the_key_file() {
    let dir = Dir::new("adv-keyid-order");
    let (url, count) = counting();
    key_file_with(&dir, "broken.key", &key_id_member("k\n"));
    let good = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "broken.key"),
        ("INBOX_OUTBOX", "alice.outbox"),
    ];
    let ran = run(&dir.path, &["pending", "--for", " x"], &good);
    assert_eq!(ran.status, USAGE, "{ran:#?}");
    assert!(ran.stderr.contains("--for"), "{ran:#?}");
    let bad_url = [
        ("INBOX_URL", "ftp://127.0.0.1:1"),
        ("INBOX_SECRET_KEY_FILE", "broken.key"),
        ("INBOX_OUTBOX", "alice.outbox"),
    ];
    let ran = run(&dir.path, &["pending"], &bad_url);
    assert_eq!(ran.status, USAGE, "{ran:#?}");
    assert!(ran.stderr.contains("INBOX_URL"), "{ran:#?}");
    assert_eq!(connections(&count), 0, "no connection");
}
