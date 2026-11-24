//! Adversary cases of story:the-chat-bridge-runs-with-its-own-key-under-a-delegation, pass 1.
//!
//! Every case runs against a real `inbox serve` on loopback with the clock file of `bridge_once`,
//! a key file made in the case, the stand-in `connectors` and the forwarder of `bridge_once`. No
//! case holds a wall-clock time: each waits on the stand-in's record, the forwarder's record or a
//! fact, with a bound that only ends a hang.

use crate::bridge_once::{self, Bridge, CHANNEL, Connectors, Forwarder};

use support::{BOB, Signer, World, chat_bridge_key, keys_only};

use inbox::server::json;
use support::text;

use crate::support;

/// The refused end writes one line, whatever the operator typed in `--authority`: a line break
/// in the delegation id must not make a second line on standard error (the value of `--for` is
/// refused for one before any request; `--authority` is printed as typed).
#[test]
fn a_line_break_in_authority_does_not_make_a_second_line_of_the_refused_end() {
    let (dir, operator_public_key) = keys_only("adv-own-key-authority-line");
    chat_bridge_key(&dir.path, None);
    let now = bridge_once::this_second();
    bridge_once::set_clock(&dir.path, "clock", now);
    let service = bridge_once::serve(&dir.path, &operator_public_key);
    let world = World {
        service,
        operator_public_key,
        dir,
    };
    let forwarder = Forwarder::start(&world.service.url);
    let connectors = Connectors::new(&world);
    let (status, said) = Bridge::signed(
        &world,
        &connectors,
        &forwarder.url(),
        CHANNEL,
        &[],
        Signer::Own {
            person: BOB,
            delegation: "chat-bridge-for-bob\ninbox bridge: a second line",
        },
    )
    .exit();
    assert_eq!(status, Some(1), "{said}");
    assert_eq!(
        said.lines().count(),
        1,
        "one line on standard error: {said:?}"
    );
}

/// Pass 2. The characters a reader may take for a line end or a change of order besides `\n`: a
/// carriage return, U+2028, U+2029, the next-line character U+0085, an escape sequence of a
/// terminal, a right-to-left override, a zero width space, and a very long value.
const ODD: &str = "d\r\u{2028}x\u{2029}y\u{85}z\u{1b}[2J\u{202e}rev\u{200b}e";

/// [`ODD`] as the line of the refused end shows it.
const ODD_SHOWN: &str = "d\\u{d}\\u{2028}x\\u{2029}y\\u{85}z\\u{1b}[2J\\u{202e}rev\\u{200b}e";

/// Whether `character` is one the line of the refused end must not hold raw.
fn raw(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{2028}' | '\u{2029}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{200b}'
        )
}

/// Pass 2, attack 1: `inbox bridge --json --for bob --authority <odd>` with a key the service
/// does not hold, and a principal in the secret key file that holds a line end and odd
/// characters too. Standard error is one line ending in its one line end, with no raw control,
/// separator or direction character, the ids in the order written; standard output is one JSON
/// document on one line, the failure `key refused`.
#[test]
fn odd_characters_in_the_ids_keep_the_refused_end_one_line_on_both_streams() {
    let (dir, operator_public_key) = keys_only("adv-own-key-odd-ids");
    let principal = format!("agent:demo/odd\n{ODD}");
    let made = support::run(
        &dir.path,
        &[
            "key",
            "new",
            "--out",
            "odd.key",
            "--key-id",
            "odd-key-1",
            "--principal",
            &principal,
        ],
        &[],
    );
    assert_eq!(made.status, 0, "{made:#?}");
    let now = bridge_once::this_second();
    bridge_once::set_clock(&dir.path, "clock", now);
    let service = bridge_once::serve(&dir.path, &operator_public_key);
    let world = World {
        service,
        operator_public_key,
        dir,
    };
    let connectors = Connectors::new(&world);
    let long = "a".repeat(20_000);
    let authority = format!("{ODD}{long}");
    let out = world.dir.join("odd.out");
    let err = world.dir.join("odd.err");
    let mut child = world
        .client_command(
            "bob",
            &[
                "bridge",
                "--json",
                "--chat-channel",
                CHANNEL,
                "--allowed-chat-user",
                "U0000000001",
                "--chat-connector",
                "slack",
                "--for",
                BOB,
                "--authority",
                &authority,
            ],
        )
        .env("INBOX_SECRET_KEY_FILE", "odd.key")
        .env("PATH", connectors.path())
        .stdout(std::fs::File::create(&out).expect("the stdout file"))
        .stderr(std::fs::File::create(&err).expect("the stderr file"))
        .spawn()
        .expect("the binary starts");
    let mut status = None;
    bridge_once::wait_for("the bridge ends", || {
        status = child.try_wait().expect("the bridge is waited for");
        status.is_some()
    });
    let said = std::fs::read_to_string(&err).expect("standard error is UTF-8");
    let printed = std::fs::read_to_string(&out).expect("standard output is UTF-8");
    assert_eq!(status.and_then(|status| status.code()), Some(1), "{said:?}");
    let line = said.strip_suffix('\n').expect("one line end at the end");
    let held: Vec<char> = line.chars().filter(|character| raw(*character)).collect();
    assert!(held.is_empty(), "raw {held:?} in {said:?}");
    let shown = format!(
        "acting for {BOB} under the delegation {ODD_SHOWN}{long} (401): its key file holds no \
         live key entry odd-key-1 or no live delegation {ODD_SHOWN}{long} of \
         agent:demo/odd\\u{{a}}{ODD_SHOWN} for {BOB};"
    );
    assert!(line.contains(&shown), "{line:?}");
    assert!(line.starts_with("inbox bridge: the service refused the key odd-key-1 "));
    let document = printed.strip_suffix('\n').unwrap_or(&printed);
    assert!(
        !document.chars().any(|character| character.is_control()),
        "one line on standard output: {printed:?}"
    );
    let parsed = json::parse(document).unwrap_or_else(|error| panic!("{error}: {printed:?}"));
    assert_eq!(text(&parsed, "error"), "key refused", "{printed:?}");
}

/// Pass 2, attack 1: a key id in the secret key file with a line separator, a right-to-left
/// override and an escape sequence (a line end cannot reach the service: the client refuses it
/// in a header). The service answers the unknown key `401`; the refused end is one line that
/// shows each of them as its escape.
#[test]
fn odd_characters_in_the_key_id_keep_the_refused_end_one_line() {
    let (dir, operator_public_key) = keys_only("adv-own-key-odd-key-id");
    let key_id = "odd\u{2028}key\u{202e}1\u{1b}[1m";
    let made = support::run(
        &dir.path,
        &[
            "key",
            "new",
            "--out",
            "odd.key",
            "--key-id",
            key_id,
            "--principal",
            "agent:demo/odd",
        ],
        &[],
    );
    assert_eq!(made.status, 0, "{made:#?}");
    let now = bridge_once::this_second();
    bridge_once::set_clock(&dir.path, "clock", now);
    let service = bridge_once::serve(&dir.path, &operator_public_key);
    let world = World {
        service,
        operator_public_key,
        dir,
    };
    let connectors = Connectors::new(&world);
    let mut command = world.client_command(
        "bob",
        &[
            "bridge",
            "--chat-channel",
            CHANNEL,
            "--allowed-chat-user",
            "U0000000001",
            "--chat-connector",
            "slack",
            "--for",
            BOB,
            "--authority",
            "chat-bridge-for-bob",
        ],
    );
    let ran = command
        .env("INBOX_SECRET_KEY_FILE", "odd.key")
        .env("PATH", connectors.path())
        .output()
        .expect("the binary starts");
    let said = String::from_utf8(ran.stderr).expect("standard error is UTF-8");
    assert_eq!(ran.status.code(), Some(1), "{said:?}");
    let line = said.strip_suffix('\n').expect("one line end at the end");
    let held: Vec<char> = line.chars().filter(|character| raw(*character)).collect();
    assert!(held.is_empty(), "raw {held:?} in {said:?}");
    assert!(
        line.starts_with(
            "inbox bridge: the service refused the key odd\\u{2028}key\\u{202e}1\\u{1b}[1m acting \
             for agent:test/bob under the delegation chat-bridge-for-bob (401)"
        ),
        "{said:?}"
    );
}
