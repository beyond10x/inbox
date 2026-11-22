//! An argument of `inbox` that is not UTF-8 is wrong usage: the client ends with the exit status
//! 2 and one line on standard error that names the argument escaped, so no invalid byte and no
//! control character reaches the terminal, and it sends no request. It does not panic (the
//! exit status 101).

use std::ffi::{OsStr, OsString};
use std::net::TcpListener;
use std::os::unix::ffi::OsStrExt;

use crate::support::{ALICE, Dir, USAGE, command, key_new};

/// What one run of the binary printed, as bytes.
struct Output {
    status: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// A listener that stands in for the service: the client's address points at it, and a request
/// would be a connection to it.
struct Listener {
    url: String,
    listener: TcpListener,
}

impl Listener {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the listener binds");
        listener
            .set_nonblocking(true)
            .expect("the listener does not block");
        let url = format!("http://{}", listener.local_addr().expect("an address"));
        Self { url, listener }
    }

    /// Asserts no connection was made.
    fn received_nothing(&self, what: &str) {
        match self.listener.accept() {
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            other => panic!("{what} sent a request: {other:?}"),
        }
    }
}

fn os(bytes: &[u8]) -> OsString {
    OsStr::from_bytes(bytes).to_owned()
}

/// `inbox <args>` in `dir`, the service at `service`, caller `alice`'s key.
fn inbox(dir: &Dir, service: &Listener, args: &[OsString]) -> Output {
    let output = command(&dir.path)
        .env("INBOX_URL", &service.url)
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

/// `args` ends the client with the status 2 and one line on standard error that holds `named`
/// and no byte that is not printable ASCII, and sends no request.
fn refused(name: &str, args: Vec<OsString>, named: &str) {
    let dir = Dir::new(name);
    key_new(&dir.path, "alice.key", Some(("alice-test-key", ALICE)));
    let service = Listener::new();
    let output = inbox(&dir, &service, &args);
    let stderr = String::from_utf8(output.stderr.clone())
        .unwrap_or_else(|_| panic!("{name}: standard error is not UTF-8: {:?}", output.stderr));
    assert_eq!(output.status, Some(USAGE), "{name}: {stderr:?}");
    assert!(output.stdout.is_empty(), "{name}: {:?}", output.stdout);
    assert_eq!(stderr.matches('\n').count(), 1, "{name}: {stderr:?}");
    assert!(stderr.ends_with('\n'), "{name}: {stderr:?}");
    assert!(
        stderr
            .chars()
            .all(|c| c == '\n' || (c.is_ascii() && !c.is_ascii_control())),
        "{name}: a raw byte reached standard error: {stderr:?}"
    );
    assert!(
        stderr.contains(named),
        "{name}: {named:?} not in {stderr:?}"
    );
    service.received_nothing(name);
}

#[test]
fn a_positional_argument_that_is_not_utf_8_is_a_usage_error() {
    refused(
        "positional",
        vec![os(b"answer"), os(b"msg-\xe9\xff"), os(b"--body"), os(b"x")],
        r#"argument 2 is not UTF-8: "msg-\xE9\xFF""#,
    );
}

#[test]
fn an_option_value_that_is_not_utf_8_is_a_usage_error() {
    refused(
        "option-value",
        vec![
            os(b"post"),
            os(b"--to"),
            os(b"agent:test/\xc3("),
            os(b"--kind"),
            os(b"request"),
        ],
        r#"argument 3 is not UTF-8: "agent:test/\xC3(""#,
    );
}

#[test]
fn an_argument_before_the_subcommand_that_is_not_utf_8_is_a_usage_error() {
    refused(
        "before-subcommand",
        vec![os(b"\xfe"), os(b"open")],
        r#"argument 1 is not UTF-8: "\xFE""#,
    );
}

/// A file name that is not UTF-8 beside an option value read as text: the line names the
/// value read as text, not the file name, which a path option takes.
#[test]
fn the_refused_argument_is_the_text_not_the_file_name_beside_it() {
    refused(
        "beside-path",
        vec![
            os(b"key"),
            os(b"new"),
            os(b"--out"),
            os(b"caf\xe9.key"),
            os(b"--key-id"),
            os(b"\xff"),
            os(b"--principal"),
            os(ALICE.as_bytes()),
        ],
        r#"argument 6 is not UTF-8: "\xFF""#,
    );
}

/// A line clap refuses for another reason, with a valid file name that is not UTF-8 on it,
/// does not blame the file name: clap's own message, the exit status 2, no request.
#[test]
fn a_line_refused_for_another_reason_does_not_blame_a_file_name_that_is_not_utf_8() {
    let dir = Dir::new("other-reason");
    let service = Listener::new();
    let output = inbox(
        &dir,
        &service,
        &[
            os(b"key"),
            os(b"new"),
            os(b"--out"),
            os(b"caf\xe9.key"),
            os(b"--no-such-option"),
        ],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status, Some(USAGE), "{stderr:?}");
    assert!(
        stderr.contains("--no-such-option") && !stderr.contains("not UTF-8"),
        "{stderr:?}"
    );
    assert!(!dir.join("caf\u{fffd}.key").exists());
    service.received_nothing("other reason");
}

#[test]
fn the_equals_form_of_a_path_option_is_not_the_refused_argument_either() {
    refused(
        "beside-path-equals",
        vec![
            os(b"key"),
            os(b"new"),
            os(b"--out=caf\xe9.key"),
            os(b"--key-id"),
            os(b"alice-test-key"),
            os(b"--principal"),
            os(b"agent:\xfe"),
        ],
        r#"argument 7 is not UTF-8: "agent:\xFE""#,
    );
}

#[test]
fn a_value_of_the_form_option_equals_bytes_that_is_not_utf_8_is_a_usage_error() {
    refused(
        "equals-form",
        vec![os(b"open"), os(b"--adapter=ter\x80m")],
        r#"argument 2 is not UTF-8: "--adapter=ter\x80m""#,
    );
}

/// An argument that holds control characters next to the invalid byte reaches the terminal
/// escaped as well: an erase of the screen, a window title set by OSC, a carriage return.
#[test]
fn control_characters_of_an_argument_that_is_not_utf_8_are_shown_escaped() {
    refused(
        "control",
        vec![os(b"open"), os(b"\x1b[2J\x1b]0;owned\x07\rx\xff\xc2\x9b")],
        r#"\xFF"#,
    );
}

/// With `--json` the failure is the usual one document on standard output; standard error holds
/// the same one escaped line.
#[test]
fn the_json_flag_still_gets_a_usage_document_for_an_argument_that_is_not_utf_8() {
    let dir = Dir::new("json");
    let service = Listener::new();
    let output = inbox(&dir, &service, &[os(b"open"), os(b"--json"), os(b"\xff")]);
    assert_eq!(output.status, Some(USAGE), "{:?}", output.stderr);
    let document = String::from_utf8(output.stdout).expect("standard output is UTF-8");
    assert!(
        document.starts_with(r#"{"error":"usage","detail":"argument 3 is not UTF-8"#),
        "{document:?}"
    );
    assert_eq!(
        output.stderr.iter().filter(|byte| **byte == b'\n').count(),
        1,
        "{:?}",
        output.stderr
    );
    service.received_nothing("--json");
}

/// A valid argument that is not ASCII is not affected.
#[test]
fn an_argument_that_is_utf_8_but_not_ascii_is_not_a_usage_error_of_this_kind() {
    let dir = Dir::new("utf8-ok");
    let service = Listener::new();
    let output = inbox(&dir, &service, &[os("grüße".as_bytes()), os(b"open")]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("not UTF-8"), "{stderr:?}");
    service.received_nothing("a UTF-8 argument");
}
