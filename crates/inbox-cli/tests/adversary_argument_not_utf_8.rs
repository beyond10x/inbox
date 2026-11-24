//! Adversary pass on story:an-argument-that-is-not-utf-8-is-a-usage-error. Cases that try to
//! break the refusal of an argument that is not UTF-8: a path option, which the client took
//! as a file name before the change; a refused argument with letters that are not ASCII; a
//! line feed in the refused argument; `--json` after the refused argument; the program name and
//! `INBOX_URL` not UTF-8; a UTF-8 argument that is not ASCII reaching the command.

use std::ffi::{OsStr, OsString};
use std::net::TcpListener;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;

use crate::support::{ALICE, ClosedPort, Dir, USAGE, command, key_new};

fn os(bytes: &[u8]) -> OsString {
    OsStr::from_bytes(bytes).to_owned()
}

/// A path inside `dir` whose file name is `name`, bytes that need not be UTF-8.
fn path_in(dir: &Dir, name: &[u8]) -> PathBuf {
    dir.path.join(OsStr::from_bytes(name))
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

fn output(mut command: std::process::Command) -> Output {
    let output = command.output().expect("the binary starts");
    Output {
        status: output.status.code(),
        stdout: output.stdout,
        stderr: output.stderr,
    }
}

/// One line on standard error, every character printable ASCII.
fn assert_one_printable_ascii_line(name: &str, stderr: &[u8]) {
    let text = String::from_utf8_lossy(stderr);
    assert_eq!(
        stderr.iter().filter(|byte| **byte == b'\n').count(),
        1,
        "{name}: {text:?}"
    );
    assert!(
        stderr
            .iter()
            .all(|byte| *byte == b'\n' || (0x20..0x7f).contains(byte)),
        "{name}: a byte that is not printable ASCII reached standard error: {text:?}"
    );
}

/// Before the change, clap read the arguments as `OsString` and a `PathBuf` option took any
/// file name; `std::env::args()` ran only on clap's error path. `inbox key new --out` with a
/// file name that is not UTF-8 made the key file. It is a valid file name on Linux.
#[test]
fn a_path_option_that_is_not_utf_8_still_names_a_file() {
    let dir = Dir::new("adv-path-out");
    let out = path_in(&dir, b"caf\xe9.key");
    let mut run = command(&dir.path);
    run.args([
        OsStr::new("key"),
        OsStr::new("new"),
        OsStr::new("--out"),
        out.as_os_str(),
        OsStr::new("--key-id"),
        OsStr::new("alice-test-key"),
        OsStr::new("--principal"),
        OsStr::new(ALICE),
    ]);
    let result = output(run);
    assert_eq!(
        result.status,
        Some(0),
        "key new --out <file name that is not UTF-8>: {:?}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(out.exists(), "the key file was not made");
}

/// The same file name is taken from `INBOX_SECRET_KEY_FILE` and refused from
/// `--secret-key-file`: one option, two answers.
#[test]
fn the_secret_key_file_option_takes_the_path_its_variable_takes() {
    let dir = Dir::new("adv-path-key");
    key_new(&dir.path, "alice.key", Some(("alice-test-key", ALICE)));
    let key = path_in(&dir, b"caf\xe9.key");
    std::fs::rename(dir.join("alice.key"), &key).expect("the key file is renamed");
    let closed_port = ClosedPort::new();
    let url = closed_port.url();

    let mut by_variable = command(&dir.path);
    by_variable
        .env("INBOX_URL", &url)
        .env("INBOX_SECRET_KEY_FILE", &key)
        .arg("open");
    let by_variable = output(by_variable);
    assert_ne!(
        by_variable.status,
        Some(USAGE),
        "the variable form: {:?}",
        String::from_utf8_lossy(&by_variable.stderr)
    );

    let mut by_option = command(&dir.path);
    by_option
        .env("INBOX_URL", &url)
        .arg("open")
        .arg("--secret-key-file")
        .arg(&key);
    let by_option = output(by_option);
    assert_eq!(
        by_option.status,
        by_variable.status,
        "the option form {:?} answered other than the variable form {:?}",
        String::from_utf8_lossy(&by_option.stderr),
        String::from_utf8_lossy(&by_variable.stderr)
    );
}

/// The refused argument holds letters that are not ASCII beside the invalid byte. The line
/// names it in printable ASCII only, as it does for the cases of the unit.
#[test]
fn a_refused_argument_with_letters_that_are_not_ascii_is_written_in_printable_ascii() {
    let dir = Dir::new("adv-not-ascii");
    let service = Quiet::new();
    let mut bad = "grüße".as_bytes().to_vec();
    bad.push(0xff);
    let mut run = command(&dir.path);
    run.env("INBOX_URL", &service.url).arg("open").arg(os(&bad));
    let result = output(run);
    assert_eq!(result.status, Some(USAGE));
    assert!(result.stdout.is_empty());
    assert_one_printable_ascii_line("not-ascii", &result.stderr);
    service.received_nothing("not-ascii");
}

/// A line feed in the refused argument cannot start a second line on standard error that
/// looks like one of the client's own.
#[test]
fn a_line_feed_in_a_refused_argument_does_not_start_a_second_line() {
    let dir = Dir::new("adv-line-feed");
    let service = Quiet::new();
    let mut run = command(&dir.path);
    run.env("INBOX_URL", &service.url)
        .arg("open")
        .arg(os(b"\xff\ninbox: refused: forged (403)\n\x85\xe2\x80\xa8"));
    let result = output(run);
    assert_eq!(result.status, Some(USAGE));
    assert_one_printable_ascii_line("line-feed", &result.stderr);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.starts_with(r#"inbox: usage: argument 2 is not UTF-8: "\xFF\ninbox: refused"#),
        "{stderr:?}"
    );
    service.received_nothing("line-feed");
}

/// `--json` after the refused argument gets the usage document as well.
#[test]
fn the_json_flag_after_the_refused_argument_gets_the_usage_document() {
    let dir = Dir::new("adv-json-after");
    let service = Quiet::new();
    let mut run = command(&dir.path);
    run.env("INBOX_URL", &service.url)
        .arg("open")
        .arg(os(b"\xff"))
        .arg("--json");
    let result = output(run);
    assert_eq!(result.status, Some(USAGE));
    let document = String::from_utf8(result.stdout).expect("standard output is UTF-8");
    assert_eq!(
        document.trim_end(),
        "{\"error\":\"usage\",\"detail\":\"argument 2 is not UTF-8: \\\"\\\\xFF\\\"\"}"
    );
    assert_one_printable_ascii_line("json-after", &result.stderr);
    service.received_nothing("json-after");
}

/// A very long refused argument stays one line of printable ASCII.
#[test]
fn a_long_refused_argument_stays_one_printable_line() {
    let dir = Dir::new("adv-long");
    let service = Quiet::new();
    let bad: Vec<u8> = b"\x01\xff\n"
        .iter()
        .copied()
        .cycle()
        .take(96 * 1024)
        .collect();
    let mut run = command(&dir.path);
    run.env("INBOX_URL", &service.url).arg("open").arg(os(&bad));
    let result = output(run);
    assert_eq!(result.status, Some(USAGE));
    assert_one_printable_ascii_line("long", &result.stderr);
    service.received_nothing("long");
}

/// A program name that is not UTF-8 is not an argument: a usage error of clap still ends with
/// the status 2 and no panic. Before the change, clap's error path read `std::env::args()`,
/// which panicked on it.
#[test]
fn a_program_name_that_is_not_utf_8_does_not_panic() {
    let dir = Dir::new("adv-arg0");
    let service = Quiet::new();
    let mut run = command(&dir.path);
    run.arg0(os(b"inbox\xff"))
        .env("INBOX_URL", &service.url)
        .args(["open", "--no-such-option"]);
    let result = output(run);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert_eq!(result.status, Some(USAGE), "{stderr:?}");
    assert!(!stderr.contains("panicked"), "{stderr:?}");
    service.received_nothing("arg0");
}

/// `INBOX_URL` that is not UTF-8 is a usage error without a panic and without a raw byte on
/// standard error.
#[test]
fn an_inbox_url_that_is_not_utf_8_is_a_usage_error() {
    let dir = Dir::new("adv-url");
    key_new(&dir.path, "alice.key", Some(("alice-test-key", ALICE)));
    let mut run = command(&dir.path);
    run.env("INBOX_URL", os(b"http://127.0.0.1:1/\xff\x1b[2J"))
        .env("INBOX_SECRET_KEY_FILE", dir.join("alice.key"))
        .arg("open");
    let result = output(run);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert_eq!(result.status, Some(USAGE), "{stderr:?}");
    assert!(!stderr.contains("panicked"), "{stderr:?}");
    assert!(
        !result
            .stderr
            .iter()
            .any(|byte| *byte == 0x1b || *byte >= 0x80),
        "{stderr:?}"
    );
}

/// A UTF-8 argument that is not ASCII reaches the command: `key new --out grüße.key` makes
/// the file.
#[test]
fn a_utf_8_argument_that_is_not_ascii_reaches_the_command() {
    let dir = Dir::new("adv-utf8-out");
    key_new(&dir.path, "grüße.key", Some(("alice-test-key", ALICE)));
    assert!(dir.join("grüße.key").exists());
}
