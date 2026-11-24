//! An `INBOX_*` variable that a command reads as text and that is not UTF-8 is wrong usage:
//! the command ends with the exit status 2 and one line of printable ASCII on standard error that
//! names the variable and shows its first 64 bytes escaped, before it opens a listener or sends a
//! request. `INBOX_BACKEND` read as unset before (`inbox serve` started with `memory`); the
//! variables clap reads ended with clap's message of 5 lines that named none. A variable the
//! command does not read stays ignored, and the 2 path variables take a name that is not UTF-8.

use std::ffi::{OsStr, OsString};
use std::io::{BufRead, BufReader, Read};
use std::net::TcpListener;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::{Child, Stdio};

use crate::support::{BOB, Dir, USAGE, World, command, keys_only};

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

/// What one run of the binary printed, as bytes.
struct Output {
    status: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl Output {
    fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
}

fn output(mut command: std::process::Command) -> Output {
    let output = command.output().expect("the binary starts");
    Output {
        status: output.status.code(),
        stdout: output.stdout,
        stderr: output.stderr,
    }
}

/// The run ended with the status 2, wrote exactly `line` and a line feed on standard error, and
/// every byte of it is printable ASCII.
fn assert_refused(name: &str, run: &Output, line: &str) {
    let stderr = run.stderr();
    assert_eq!(run.status, Some(USAGE), "{name}: {stderr:?}");
    assert!(
        run.stderr
            .iter()
            .all(|byte| *byte == b'\n' || (0x20..0x7f).contains(byte)),
        "{name}: a byte that is not printable ASCII reached standard error: {stderr:?}"
    );
    assert_eq!(stderr, format!("{line}\n"), "{name}");
}

/// A child that is killed and waited for when dropped, so no service outlives a case.
struct Started(Child);

impl Drop for Started {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// What `inbox serve` did: printed its first line, the URL it answers at once it listens
/// (`Ok`, the service is then ended), or ended without one (`Err`, with what it printed).
fn serve(dir: &Dir, args: &[&str], env: &[(&str, OsString)]) -> Result<String, Output> {
    let mut run = command(&dir.path);
    run.arg("serve")
        .args(args)
        .envs(env.iter().map(|(name, value)| (name, value)))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = Started(run.spawn().expect("the binary starts"));
    let mut stdout = BufReader::new(child.0.stdout.take().expect("stdout is piped"));
    let mut first = Vec::new();
    // The read ends at the first line or when the process closed its standard output: a fact,
    // not a pause.
    stdout
        .read_until(b'\n', &mut first)
        .expect("standard output is read");
    if first.ends_with(b"\n") {
        return Ok(String::from_utf8_lossy(&first).trim_end().to_owned());
    }
    let mut rest = Vec::new();
    stdout
        .read_to_end(&mut rest)
        .expect("standard output is read");
    first.extend(rest);
    let mut stderr = Vec::new();
    child
        .0
        .stderr
        .take()
        .expect("stderr is piped")
        .read_to_end(&mut stderr)
        .expect("standard error is read");
    let status = child.0.wait().expect("the process is waited for");
    Err(Output {
        status: status.code(),
        stdout: first,
        stderr,
    })
}

/// `inbox serve` that must refuse to start: what it printed. Fails when it printed the URL it
/// listens at.
fn serve_refused(dir: &Dir, args: &[&str], env: &[(&str, OsString)]) -> Output {
    match serve(dir, args, env) {
        Ok(url) => panic!("inbox serve opened a listener and printed {url:?}"),
        Err(output) => output,
    }
}

/// The arguments of `inbox serve` over the key set of `keys_only`, on an ephemeral loopback
/// port.
fn serve_args(operator: &str) -> Vec<&str> {
    vec![
        "--listen",
        "127.0.0.1:0",
        "--key-file",
        "keys.json",
        "--operator-public-key",
        operator,
    ]
}

/// `inbox <args>` as a client of `service`, with the variables `env`.
fn client(dir: &Dir, service: &Quiet, args: &[&str], env: &[(&str, OsString)]) -> Output {
    let mut run = command(&dir.path);
    run.env("INBOX_URL", &service.url)
        .env("INBOX_SECRET_KEY_FILE", dir.join("alice.key"))
        .args(args)
        .envs(env.iter().map(|(name, value)| (name, value)));
    output(run)
}

/// Before: `inbox serve` read the value as unset and started with the backend `memory`. After:
/// exit 2, one line that names the variable, and standard output ends without the URL, the
/// first line the service prints once it listens.
#[test]
fn serve_with_an_inbox_backend_that_is_not_utf_8_is_a_usage_error_and_opens_no_listener() {
    let (dir, operator) = keys_only("var-backend");
    let refused = serve_refused(
        &dir,
        &serve_args(&operator),
        &[("INBOX_BACKEND", os(b"mem\xffory"))],
    );
    assert!(refused.stdout.is_empty(), "{:?}", refused.stdout);
    assert_refused(
        "backend",
        &refused,
        r#"inbox: usage: INBOX_BACKEND is not UTF-8: "mem\xFFory""#,
    );
}

#[test]
fn serve_with_an_operator_public_key_variable_that_is_not_utf_8_names_it() {
    let dir = Dir::new("var-operator");
    let refused = serve_refused(
        &dir,
        &["--listen", "127.0.0.1:0", "--key-file", "keys.json"],
        &[("INBOX_OPERATOR_PUBLIC_KEY", os(b"\xfe\xff"))],
    );
    assert!(refused.stdout.is_empty(), "{:?}", refused.stdout);
    assert_refused(
        "operator",
        &refused,
        r#"inbox: usage: INBOX_OPERATOR_PUBLIC_KEY is not UTF-8: "\xFE\xFF""#,
    );
}

#[test]
fn a_client_command_with_an_inbox_url_that_is_not_utf_8_names_it_and_sends_nothing() {
    let dir = Dir::new("var-url");
    let service = Quiet::new();
    let mut url = service.url.clone().into_bytes();
    url.push(0xff);
    let refused = client(&dir, &service, &["open"], &[("INBOX_URL", os(&url))]);
    assert!(refused.stdout.is_empty(), "{:?}", refused.stdout);
    assert_refused(
        "url",
        &refused,
        &format!(
            r#"inbox: usage: INBOX_URL is not UTF-8: "{}\xFF""#,
            service.url
        ),
    );
    service.received_nothing("url");
}

#[test]
fn a_client_command_with_a_tls_pin_that_is_not_utf_8_names_it_and_sends_nothing() {
    let dir = Dir::new("var-pin");
    let service = Quiet::new();
    let refused = client(
        &dir,
        &service,
        &["pending"],
        &[("INBOX_TLS_PIN", os(b"sha256:\xe9"))],
    );
    assert_refused(
        "pin",
        &refused,
        r#"inbox: usage: INBOX_TLS_PIN is not UTF-8: "sha256:\xE9""#,
    );
    service.received_nothing("pin");
}

/// A value of 200 bytes: the line shows the first 64, escaped, and counts the other 136.
#[test]
fn a_long_variable_shows_its_first_64_bytes_and_the_count_of_the_rest() {
    let dir = Dir::new("var-long");
    let service = Quiet::new();
    let mut value = vec![b'a'; 63];
    value.push(0xff);
    value.extend(std::iter::repeat_n(b'b', 136));
    assert_eq!(value.len(), 200);
    let refused = client(&dir, &service, &["open"], &[("INBOX_URL", os(&value))]);
    assert_refused(
        "long",
        &refused,
        &format!(
            r#"inbox: usage: INBOX_URL is not UTF-8: "{}\xFF" (and 136 more bytes)"#,
            "a".repeat(63)
        ),
    );
    service.received_nothing("long");
}

/// A line feed, a carriage return, a tab, an escape byte, a quote and a backslash in the value:
/// still one line of printable ASCII, so no text of the value starts a line of its own.
#[test]
fn a_line_break_and_an_escape_byte_in_a_variable_stay_on_one_printable_line() {
    let dir = Dir::new("var-line-feed");
    let service = Quiet::new();
    let refused = client(
        &dir,
        &service,
        &["open"],
        &[(
            "INBOX_TLS_PIN",
            os(b"\xff\ninbox: refused: forged (403)\r\t\x1b[2J\"\\"),
        )],
    );
    assert_refused(
        "line-feed",
        &refused,
        r#"inbox: usage: INBOX_TLS_PIN is not UTF-8: "\xFF\ninbox: refused: forged (403)\r\t\x1B[2J\"\\""#,
    );
    service.received_nothing("line-feed");
}

#[test]
fn the_json_flag_gets_the_usage_document_for_a_client_variable() {
    let dir = Dir::new("var-json");
    let service = Quiet::new();
    let refused = client(
        &dir,
        &service,
        &["--json", "open"],
        &[("INBOX_URL", os(b"\xff"))],
    );
    assert_eq!(
        String::from_utf8_lossy(&refused.stdout).trim_end(),
        r#"{"error":"usage","detail":"INBOX_URL is not UTF-8: \"\\xFF\""}"#
    );
    assert_refused(
        "json",
        &refused,
        r#"inbox: usage: INBOX_URL is not UTF-8: "\xFF""#,
    );
    service.received_nothing("json");
}

#[test]
fn the_json_flag_gets_the_usage_document_for_the_backend() {
    let (dir, operator) = keys_only("var-json-backend");
    let mut args = vec!["--json"];
    args.extend(serve_args(&operator));
    // With `--json` the first line is a document either way: the usage document, or the one
    // naming the bound address when the service listens. The read ends at the first line.
    match serve(&dir, &args, &[("INBOX_BACKEND", os(b"\xff"))]) {
        Ok(first) => assert_eq!(
            first, r#"{"error":"usage","detail":"INBOX_BACKEND is not UTF-8: \"\\xFF\""}"#,
            "the service listened, or printed another document"
        ),
        Err(output) => panic!("no document: {:?}", output.stderr()),
    }
}

/// An argument that is not UTF-8 on the line beside a variable that is not UTF-8: the argument
/// is named, as before.
#[test]
fn an_argument_that_is_not_utf_8_is_named_before_a_variable() {
    let dir = Dir::new("var-and-argument");
    let service = Quiet::new();
    let mut run = command(&dir.path);
    run.env("INBOX_TLS_PIN", os(b"\xfe"))
        .env("INBOX_URL", &service.url)
        .env("INBOX_SECRET_KEY_FILE", dir.join("alice.key"))
        .arg("answer")
        .arg(os(b"msg-\xff"))
        .args(["--body", "x"]);
    let refused = output(run);
    assert_refused(
        "argument",
        &refused,
        r#"inbox: usage: argument 2 is not UTF-8: "msg-\xFF""#,
    );
    service.received_nothing("argument");
}

/// 2 variables of the command that are not UTF-8: the first the command defines is named,
/// `INBOX_URL` before `INBOX_TLS_PIN`.
#[test]
fn of_2_variables_that_are_not_utf_8_the_url_is_named() {
    let dir = Dir::new("var-two");
    let service = Quiet::new();
    let refused = client(
        &dir,
        &service,
        &["open"],
        &[("INBOX_TLS_PIN", os(b"\xfa")), ("INBOX_URL", os(b"\xfb"))],
    );
    assert_refused(
        "two",
        &refused,
        r#"inbox: usage: INBOX_URL is not UTF-8: "\xFB""#,
    );
    service.received_nothing("two");
}

/// `--url` on the line: `INBOX_URL` is not read, so the pin that is not UTF-8 is named.
#[test]
fn a_variable_whose_flag_is_on_the_line_is_not_named() {
    let dir = Dir::new("var-flag-given");
    let service = Quiet::new();
    let refused = client(
        &dir,
        &service,
        &["open", "--url", &service.url],
        &[("INBOX_URL", os(b"\xfb")), ("INBOX_TLS_PIN", os(b"\xfa"))],
    );
    assert_refused(
        "flag-given",
        &refused,
        r#"inbox: usage: INBOX_TLS_PIN is not UTF-8: "\xFA""#,
    );
    service.received_nothing("flag-given");
}

/// A client command does not read `INBOX_OPERATOR_PUBLIC_KEY`, a variable of `inbox serve`.
#[test]
fn a_variable_of_another_command_is_not_named() {
    let dir = Dir::new("var-other-command");
    let service = Quiet::new();
    let refused = client(
        &dir,
        &service,
        &["open"],
        &[
            ("INBOX_OPERATOR_PUBLIC_KEY", os(b"\xfb")),
            ("INBOX_TLS_PIN", os(b"\xfa")),
        ],
    );
    assert_refused(
        "other-command",
        &refused,
        r#"inbox: usage: INBOX_TLS_PIN is not UTF-8: "\xFA""#,
    );
    service.received_nothing("other-command");
}

/// `inbox serve` with both of its variables not UTF-8 names `INBOX_OPERATOR_PUBLIC_KEY`: the
/// command line is read before the backend.
#[test]
fn serve_with_both_variables_not_utf_8_names_the_operator_public_key() {
    let dir = Dir::new("var-serve-two");
    let refused = serve_refused(
        &dir,
        &["--listen", "127.0.0.1:0", "--key-file", "keys.json"],
        &[
            ("INBOX_BACKEND", os(b"\xfa")),
            ("INBOX_OPERATOR_PUBLIC_KEY", os(b"\xfb")),
        ],
    );
    assert_refused(
        "serve-two",
        &refused,
        r#"inbox: usage: INBOX_OPERATOR_PUBLIC_KEY is not UTF-8: "\xFB""#,
    );
}

/// `--url` on the line beside an `INBOX_URL` that is not UTF-8 works as before: clap applies a
/// variable only when its flag is absent.
#[test]
fn the_url_flag_beside_an_inbox_url_that_is_not_utf_8_works() {
    let world = World::new("var-url-flag");
    let mut run = world.client_command("alice", &["open", "--url", &world.service.url]);
    run.env("INBOX_URL", os(b"\xff"));
    let done = output(run);
    assert_eq!(done.status, Some(0), "{:?}", done.stderr());
}

/// `inbox serve` does not read `INBOX_URL` or `INBOX_TLS_PIN`: it starts.
#[test]
fn serve_with_client_variables_that_are_not_utf_8_starts() {
    let (dir, operator) = keys_only("var-serve-client");
    let url = serve(
        &dir,
        &serve_args(&operator),
        &[("INBOX_URL", os(b"\xff")), ("INBOX_TLS_PIN", os(b"\xfe"))],
    )
    .unwrap_or_else(|output| panic!("inbox serve did not start: {:?}", output.stderr()));
    assert!(url.starts_with("http://127.0.0.1:"), "{url:?}");
}

/// A client command does not read `INBOX_BACKEND`: it works.
#[test]
fn a_client_command_with_an_inbox_backend_that_is_not_utf_8_works() {
    let world = World::new("var-client-backend");
    let mut run = world.client_command("alice", &["open"]);
    run.env("INBOX_BACKEND", os(b"\xff"));
    let done = output(run);
    assert_eq!(done.status, Some(0), "{:?}", done.stderr());
}

/// `INBOX_OUTBOX` is a path, and a file name that is not UTF-8 is one: the post is kept in that
/// file.
#[test]
fn an_outbox_variable_that_is_not_utf_8_names_the_outbox_file() {
    let world = World::new("var-outbox");
    world.client("bob", &["open"]).ok();
    let outbox = path_in(&world.dir, b"caf\xe9.outbox");
    let mut run = world.client_command("alice", &["post", "--to", BOB, "--body", "first"]);
    run.env("INBOX_OUTBOX", &outbox);
    let done = output(run);
    assert_eq!(done.status, Some(0), "{:?}", done.stderr());
    assert!(outbox.exists(), "the outbox file was not written");
    assert!(!world.outbox("alice").exists());
}
