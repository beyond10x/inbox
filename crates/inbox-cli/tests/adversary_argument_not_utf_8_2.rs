//! Adversary pass 2 on story:an-argument-that-is-not-utf-8-is-a-usage-error. The scan that
//! tells a file name from text (`refused_argument` in `src/main.rs`) reads every argument that
//! equals a path option, or starts with `<path option>=`, as the start of a file name, wherever
//! it stands. clap does not: after `--` every argument is a positional, an option of another
//! subcommand is an unknown option, and a token that starts with `-` is not a value. In those
//! places an argument read as text that is not UTF-8 must still give the one escaped line of
//! the acceptance. The other cases hold the escape of every byte and the file names of the
//! path options of nested subcommands.

use std::ffi::{OsStr, OsString};
use std::net::TcpListener;
use std::os::unix::ffi::OsStrExt;

use crate::support::{ALICE, ClosedPort, Dir, USAGE, command, key_new};

fn os(bytes: &[u8]) -> OsString {
    OsStr::from_bytes(bytes).to_owned()
}

/// A listener no request may reach.
struct Quiet {
    url: String,
    listener: TcpListener,
}

impl Quiet {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the listener binds");
        listener
            .set_nonblocking(true)
            .expect("the listener does not block");
        let url = format!("http://{}", listener.local_addr().expect("an address"));
        Self { url, listener }
    }

    fn received_nothing(&self, what: &str) {
        match self.listener.accept() {
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            other => panic!("{what} sent a request: {other:?}"),
        }
    }
}

struct Output {
    status: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// `inbox <args>` in `dir`, the service at `service`, caller `alice`'s key.
fn inbox(dir: &Dir, url: &str, args: &[OsString]) -> Output {
    let output = command(&dir.path)
        .env("INBOX_URL", url)
        .env("INBOX_SECRET_KEY_FILE", dir.join("alice.key"))
        .args(args)
        .output()
        .expect("the binary starts");
    Output {
        status: output.status.code(),
        stdout: output.stdout,
        stderr: output.stderr,
    }
}

/// The acceptance: exit status 2, one line on standard error of printable ASCII that names
/// `named`, nothing on standard output, no request.
fn refused_with_one_line(name: &str, args: &[OsString], named: &str) {
    let dir = Dir::new(name);
    key_new(&dir.path, "alice.key", Some(("alice-test-key", ALICE)));
    let service = Quiet::new();
    let output = inbox(&dir, &service.url, args);
    let text = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status, Some(USAGE), "{name}: {text:?}");
    assert!(output.stdout.is_empty(), "{name}: {:?}", output.stdout);
    assert!(
        output
            .stderr
            .iter()
            .all(|byte| *byte == b'\n' || (0x20..0x7f).contains(byte)),
        "{name}: a byte that is not printable ASCII reached standard error: {text:?}"
    );
    assert_eq!(
        output.stderr.iter().filter(|byte| **byte == b'\n').count(),
        1,
        "{name}: not one line: {text:?}"
    );
    assert!(text.contains(named), "{name}: {named:?} not in {text:?}");
    service.received_nothing(name);
}

/// After `--` the argument `--out=<bytes>` is the message id `read` reads as text, not the
/// option `--out` (which `read` does not have).
#[test]
fn a_message_id_after_double_dash_that_starts_like_a_path_option_is_named() {
    refused_with_one_line(
        "adv2-dashdash-equals",
        &[os(b"read"), os(b"--"), os(b"--out=\xff")],
        r#"argument 3 is not UTF-8: "--out=\xFF""#,
    );
}

/// After `--` the argument `--out` is the message id, and the next argument is a stray one
/// read as text, not a file name.
#[test]
fn a_stray_argument_after_double_dash_and_a_path_option_name_is_named() {
    refused_with_one_line(
        "adv2-dashdash-split",
        &[os(b"read"), os(b"--"), os(b"--out"), os(b"\xff")],
        r#"argument 4 is not UTF-8: "\xFF""#,
    );
}

/// `-<bytes>` after `--out` is not the value of `--out` for clap (a token that starts with `-`
/// is an option name); clap refuses it as an unexpected argument, an option name that is not
/// UTF-8.
#[test]
fn an_option_name_that_is_not_utf_8_after_a_path_option_is_named() {
    refused_with_one_line(
        "adv2-hyphen-value",
        &[
            os(b"key"),
            os(b"new"),
            os(b"--out"),
            os(b"-\xff"),
            os(b"--key-id"),
            os(b"alice-test-key"),
            os(b"--principal"),
            os(ALICE.as_bytes()),
        ],
        r#"argument 4 is not UTF-8: "-\xFF""#,
    );
}

/// `--file` is a path option of `keyfile`, not of `open`: for `open` the argument
/// `--file=<bytes>` is an unknown option, text that is not UTF-8.
#[test]
fn a_path_option_of_another_subcommand_is_not_a_file_name() {
    refused_with_one_line(
        "adv2-other-subcommand",
        &[os(b"open"), os(b"--file=\xff")],
        r#"argument 2 is not UTF-8: "--file=\xFF""#,
    );
}

/// Every byte value an argument can hold (1 to 255; a 0 cannot be in an argument) together,
/// each written as the documents say: printable ASCII as itself, `"` and `\` with a backslash,
/// line feed, carriage return and tab as `\n`, `\r`, `\t`, every other byte as `\x` and two
/// upper-case hex digits.
#[test]
fn every_byte_value_of_a_refused_argument_is_written_as_documented() {
    let argument: Vec<u8> = (1..=255u8).collect();
    let mut expected = String::from("argument 2 is not UTF-8: \"");
    for byte in 1..=255u8 {
        match byte {
            b'"' => expected.push_str("\\\""),
            b'\\' => expected.push_str("\\\\"),
            b'\n' => expected.push_str("\\n"),
            b'\r' => expected.push_str("\\r"),
            b'\t' => expected.push_str("\\t"),
            b' '..=b'~' => expected.push(char::from(byte)),
            _ => expected.push_str(&format!("\\x{byte:02X}")),
        }
    }
    expected.push('"');
    refused_with_one_line("adv2-every-byte", &[os(b"open"), os(&argument)], &expected);
}

/// A surrogate half in the WTF-8 form and an overlong encoding are not UTF-8: refused and
/// written byte by byte.
#[test]
fn a_surrogate_half_and_an_overlong_encoding_are_refused_and_escaped() {
    refused_with_one_line(
        "adv2-surrogate",
        &[os(b"open"), os(b"--adapter"), os(b"\xed\xa0\x80\xc0\xaf")],
        r#"argument 3 is not UTF-8: "\xED\xA0\x80\xC0\xAF""#,
    );
}

/// The path options of nested subcommands and of the client take a file name that is not
/// UTF-8 in both spellings: the command goes past parsing (a missing file, or the transport),
/// and nothing says "not UTF-8".
#[test]
fn the_path_options_of_every_command_take_a_file_name_that_is_not_utf_8() {
    let dir = Dir::new("adv2-paths");
    let public_key = key_new(&dir.path, "alice.key", Some(("alice-test-key", ALICE)));
    let closed_port = ClosedPort::new();
    let url = closed_port.url();
    let runs: [(&str, Vec<OsString>); 5] = [
        (
            "keyfile add-key --file <bytes> --operator-secret-key-file=<bytes>",
            vec![
                os(b"keyfile"),
                os(b"add-key"),
                os(b"--file"),
                os(b"keys-\xe9.json"),
                os(b"--operator-secret-key-file=op-\xe9.key"),
                os(b"--key-id"),
                os(b"k1"),
                os(b"--principal"),
                os(ALICE.as_bytes()),
                os(b"--public-key"),
                os(public_key.as_bytes()),
            ],
        ),
        (
            "keyfile add-delegation --file=<bytes> --operator-secret-key-file <bytes>",
            vec![
                os(b"keyfile"),
                os(b"add-delegation"),
                os(b"--file=keys-\xe9.json"),
                os(b"--operator-secret-key-file"),
                os(b"op-\xe9.key"),
                os(b"--delegation-id"),
                os(b"d1"),
                os(b"--agent"),
                os(ALICE.as_bytes()),
                os(b"--on-behalf-of"),
                os(b"agent:test/bob"),
            ],
        ),
        (
            "post --body-file=<bytes>",
            vec![
                os(b"post"),
                os(b"--to"),
                os(b"agent:test/bob"),
                os(b"--body-file=body-\xe9.txt"),
            ],
        ),
        (
            "answer --body-file <bytes>",
            vec![
                os(b"answer"),
                os(b"01JGT0AE000000000000000001"),
                os(b"--body-file"),
                os(b"body-\xe9.txt"),
            ],
        ),
        (
            "--json open --outbox=<bytes>",
            vec![
                os(b"--json"),
                os(b"open"),
                os(b"--outbox=outbox-\xe9.jsonl"),
            ],
        ),
    ];
    for (name, args) in runs {
        let output = inbox(&dir, &url, &args);
        let text = String::from_utf8_lossy(&output.stderr);
        assert_ne!(output.status, Some(USAGE), "{name}: {text:?}");
        assert_ne!(output.status, Some(101), "{name}: {text:?}");
        assert!(output.status.is_some(), "{name}: ended by a signal");
        assert!(!text.contains("not UTF-8"), "{name}: {text:?}");
    }
}
