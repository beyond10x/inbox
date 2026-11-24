//! Adversary pass 1 on story:a-variable-that-is-not-utf-8-is-a-usage-error. Cases that try to
//! break the refusal of an `INBOX_*` variable that is not UTF-8: every command against each of
//! the 6 variables, the flag on the line in each spelling, `inbox serve` that must open nothing,
//! the line for values at and around the cut, the usage document, and 2 bad things at once.
//! Every value is made by the case; no service is reached, and no case sends a request.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::net::TcpListener;
use std::os::unix::ffi::OsStrExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use inbox::server::json::{self, Value};

use crate::support::{ClosedPort, Dir, USAGE, command, keys_only, text};

/// The 4 variables read as text, and the 2 path variables.
const ALL: [&str; 6] = [
    "INBOX_URL",
    "INBOX_TLS_PIN",
    "INBOX_OPERATOR_PUBLIC_KEY",
    "INBOX_BACKEND",
    "INBOX_SECRET_KEY_FILE",
    "INBOX_OUTBOX",
];

fn os(bytes: &[u8]) -> OsString {
    OsStr::from_bytes(bytes).to_owned()
}

fn words(list: &[&str]) -> Vec<OsString> {
    list.iter().map(OsString::from).collect()
}

#[derive(Debug, PartialEq, Eq)]
struct Ran {
    status: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl Ran {
    fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }

    fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    fn lines(&self) -> usize {
        self.stderr.iter().filter(|byte| **byte == b'\n').count()
    }
}

/// Runs `run` to its end. A run still going after 10 s (an `inbox serve` that listens, where the
/// case expects a refusal) is killed and has no status, so such a case fails at once instead of
/// at the runner's bound. Every run of these cases ends within milliseconds otherwise.
fn started(mut run: Command) -> Ran {
    let mut child = run
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary starts");
    let limit = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().expect("the child is polled") {
            break status.code();
        }
        if Instant::now() >= limit {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    child
        .stdout
        .take()
        .expect("stdout is piped")
        .read_to_end(&mut stdout)
        .expect("standard output is read");
    child
        .stderr
        .take()
        .expect("stderr is piped")
        .read_to_end(&mut stderr)
        .expect("standard error is read");
    Ran {
        status,
        stdout,
        stderr,
    }
}

/// `inbox <args>` in `dir` with the variables `env` and no other `INBOX_*` variable.
fn ran(dir: &Dir, args: &[OsString], env: &[(&str, OsString)]) -> Ran {
    let mut run = command(&dir.path);
    run.args(args)
        .envs(env.iter().map(|(name, value)| (*name, value)));
    started(run)
}

/// The line that names `name` with the value `\xFF`.
fn named(name: &str) -> String {
    format!("inbox: usage: {name} is not UTF-8: \"\\xFF\"\n")
}

/// The client commands, each with the least arguments that pass parsing.
const CLIENT: [&[&str]; 16] = [
    &["open"],
    &["close"],
    &["post", "--to", "agent:test/bob", "--body", "b"],
    &["pending"],
    &["wait"],
    &["wake"],
    &["wake", "--follow"],
    &["bridge"],
    &["read", "m"],
    &["answer", "m", "--body", "b"],
    &["refuse", "m", "--reason", "r"],
    &["outbox"],
    &["collect", "m"],
    &["thread", "t"],
    &["replies", "t"],
    &["repost"],
];

/// The other commands; each ends before a listener opens or a file is written: `serve` at the
/// operator public key the base environment gives (no key), the others at a file that is there
/// or absent.
const LOCAL: [&[&str]; 5] = [
    &[
        "serve",
        "--listen",
        "127.0.0.1:0",
        "--key-file",
        "keys.json",
    ],
    &["key", "new", "--out", "taken"],
    &["tls", "new", "--out", "taken"],
    &[
        "keyfile",
        "add-key",
        "--file",
        "keys.json",
        "--operator-secret-key-file",
        "absent.key",
        "--key-id",
        "k",
        "--principal",
        "p",
        "--public-key",
        "x",
    ],
    &[
        "keyfile",
        "add-delegation",
        "--file",
        "keys.json",
        "--operator-secret-key-file",
        "absent.key",
        "--delegation-id",
        "d",
        "--agent",
        "a",
        "--on-behalf-of",
        "b",
    ],
];

/// Whether the command `args` reads `variable` as text: a client command `INBOX_URL` and
/// `INBOX_TLS_PIN`, `inbox serve` `INBOX_OPERATOR_PUBLIC_KEY` and `INBOX_BACKEND`.
fn reads_as_text(args: &[&str], variable: &str) -> bool {
    if args[0] == "serve" {
        matches!(variable, "INBOX_OPERATOR_PUBLIC_KEY" | "INBOX_BACKEND")
    } else if CLIENT.iter().any(|client| client[0] == args[0]) {
        matches!(variable, "INBOX_URL" | "INBOX_TLS_PIN")
    } else {
        false
    }
}

/// Every command against each of the 6 variables set to `\xFF`: a command that reads the variable
/// as text ends with 2 and the one line that names it; every other run is byte for byte the run
/// without the variable. The base environment gives `inbox serve` an operator public key that is
/// none (so it ends before it listens) and a client no `INBOX_URL` (so it ends before a request).
#[test]
fn every_command_refuses_exactly_the_variables_it_reads_as_text() {
    let dir = Dir::new("adv-var-table");
    std::fs::write(dir.join("taken"), "").expect("the taken file is written");
    let base = |variable: Option<&'static str>| {
        let mut env = vec![("INBOX_OPERATOR_PUBLIC_KEY", OsString::from("no-key"))];
        if let Some(variable) = variable {
            env.retain(|(name, _)| *name != variable);
            env.push((variable, os(b"\xff")));
        }
        env
    };
    let mut table = Vec::new();
    let mut wrong = Vec::new();
    for args in CLIENT.iter().chain(LOCAL.iter()) {
        let line = words(args);
        let before = ran(&dir, &line, &base(None));
        for variable in ALL {
            let after = ran(&dir, &line, &base(Some(variable)));
            let names = after.stderr().contains(&format!("{variable} is not UTF-8"));
            table.push(format!(
                "| {} | {variable} | {:?} | {} | {} |",
                args.join(" "),
                after.status,
                after.lines(),
                if names { "yes" } else { "no" }
            ));
            if reads_as_text(args, variable) {
                if after.status != Some(USAGE)
                    || after.stderr() != named(variable)
                    || !after.stdout.is_empty()
                {
                    wrong.push(format!("{args:?} {variable}: not refused: {after:?}"));
                }
            } else if after != before {
                wrong.push(format!(
                    "{args:?} {variable}: differs from the run without it: {:?} against {:?}",
                    after.stderr(),
                    before.stderr()
                ));
            }
        }
    }
    println!("{}", table.join("\n"));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Help and version print beside every bad variable: exit 0, the text on standard output,
/// nothing on standard error.
#[test]
fn help_and_version_print_beside_every_bad_variable() {
    let dir = Dir::new("adv-var-help");
    let lines: [&[&str]; 8] = [
        &["--help"],
        &["--version"],
        &["help", "open"],
        &["help", "serve"],
        &["open", "--help"],
        &["serve", "--help"],
        &["wake", "--follow", "--help"],
        &["bridge", "--help"],
    ];
    let mut wrong = Vec::new();
    for args in lines {
        for variable in ALL {
            let after = ran(&dir, &words(args), &[(variable, os(b"\xff"))]);
            if after.status != Some(0) || after.stdout.is_empty() || !after.stderr.is_empty() {
                wrong.push(format!("{args:?} {variable}: {after:?}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The flag on the line beside its variable set to `\xFF`, in each spelling clap takes and in 2
/// it refuses: the run is byte for byte the run without the variable.
#[test]
fn a_flag_on_the_line_wins_over_its_bad_variable_in_every_spelling() {
    let dir = Dir::new("adv-var-flag");
    let closed = ClosedPort::new();
    let url = closed.url();
    let url_eq = format!("--url={url}");
    let cases: Vec<(&str, Vec<&str>)> = vec![
        ("INBOX_URL", vec!["pending", "--url", &url]),
        ("INBOX_URL", vec!["pending", &url_eq]),
        ("INBOX_URL", vec!["pending", "--url", ""]),
        ("INBOX_URL", vec!["pending", "--url="]),
        ("INBOX_URL", vec!["pending", "--url", &url, "--url", &url]),
        ("INBOX_URL", vec!["--url", &url, "pending"]),
        ("INBOX_URL", vec!["wake", "--url", &url, "--follow"]),
        ("INBOX_TLS_PIN", vec!["pending", "--tls-pin", "x"]),
        ("INBOX_TLS_PIN", vec!["pending", "--tls-pin=x"]),
        ("INBOX_TLS_PIN", vec!["pending", "--tls-pin", ""]),
        ("INBOX_TLS_PIN", vec!["pending", "--tls-pin="]),
        (
            "INBOX_TLS_PIN",
            vec!["pending", "--tls-pin", "x", "--tls-pin", "y"],
        ),
        ("INBOX_TLS_PIN", vec!["--json", "pending", "--tls-pin", "x"]),
        (
            "INBOX_OPERATOR_PUBLIC_KEY",
            vec![
                "serve",
                "--listen",
                "127.0.0.1:0",
                "--key-file",
                "keys.json",
                "--operator-public-key",
                "x",
            ],
        ),
        (
            "INBOX_OPERATOR_PUBLIC_KEY",
            vec![
                "serve",
                "--operator-public-key=x",
                "--listen",
                "127.0.0.1:0",
                "--key-file",
                "keys.json",
            ],
        ),
        (
            "INBOX_OPERATOR_PUBLIC_KEY",
            vec![
                "serve",
                "--listen",
                "127.0.0.1:0",
                "--key-file",
                "keys.json",
                "--operator-public-key",
                "",
            ],
        ),
    ];
    let mut wrong = Vec::new();
    for (variable, args) in &cases {
        let line = words(args);
        let before = ran(&dir, &line, &[]);
        let after = ran(&dir, &line, &[(*variable, os(b"\xff"))]);
        if after != before {
            wrong.push(format!(
                "{args:?} {variable}: {:?} against {:?}",
                after.stderr(),
                before.stderr()
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// After `--` a word that looks like the flag is a value: clap applied the variable, and it is
/// named. A flag on the line that is itself not UTF-8 beside a bad variable: the argument is
/// named. A path option with a name that is not UTF-8 beside a bad variable: the variable.
#[test]
fn after_a_double_dash_the_variable_is_named_and_a_bad_flag_value_wins() {
    let dir = Dir::new("adv-var-dash");
    for (variable, flag) in [("INBOX_URL", "--url"), ("INBOX_TLS_PIN", "--tls-pin")] {
        let after = ran(
            &dir,
            &words(&["read", "--", flag]),
            &[(variable, os(b"\xff"))],
        );
        assert_eq!(after.status, Some(USAGE), "{flag}: {after:?}");
        assert_eq!(after.stderr(), named(variable), "{flag}");
    }
    let mut line = words(&["pending", "--url"]);
    line.push(os(b"http://127.0.0.1:9\xfe"));
    let after = ran(&dir, &line, &[("INBOX_TLS_PIN", os(b"\xff"))]);
    assert_eq!(
        after.stderr(),
        "inbox: usage: argument 3 is not UTF-8: \"http://127.0.0.1:9\\xFE\"\n"
    );
    let mut line = words(&["pending"]);
    line.push(os(b"--url=\xfe"));
    let after = ran(&dir, &line, &[("INBOX_TLS_PIN", os(b"\xff"))]);
    assert_eq!(
        after.stderr(),
        "inbox: usage: argument 2 is not UTF-8: \"--url=\\xFE\"\n"
    );
    let mut line = words(&["pending", "--outbox"]);
    line.push(os(b"out\xfe"));
    let after = ran(&dir, &line, &[("INBOX_URL", os(b"\xff"))]);
    assert_eq!(after.stderr(), named("INBOX_URL"));
}

/// `inbox serve` over a valid key set on a port the case picked: with `INBOX_BACKEND` or
/// `INBOX_OPERATOR_PUBLIC_KEY` not UTF-8 it ends with 2, prints nothing on standard output,
/// writes no file into its directory (its home and state directory are there), and the port is
/// free to bind afterwards.
#[test]
fn serve_with_a_bad_variable_opens_nothing_and_writes_nothing() {
    let (dir, operator) = keys_only("adv-var-serve");
    let listing = || {
        let mut names: Vec<OsString> = std::fs::read_dir(&dir.path)
            .expect("the directory is read")
            .map(|entry| entry.expect("an entry").file_name())
            .collect();
        names.sort();
        names
    };
    let before = listing();
    for (variable, with_flag) in [
        ("INBOX_BACKEND", true),
        ("INBOX_OPERATOR_PUBLIC_KEY", false),
    ] {
        for json in [false, true] {
            let port = {
                let probe = TcpListener::bind("127.0.0.1:0").expect("a port");
                probe.local_addr().expect("an address").port()
            };
            let listen = format!("127.0.0.1:{port}");
            let mut args = vec!["serve", "--listen", &listen, "--key-file", "keys.json"];
            if with_flag {
                args.extend(["--operator-public-key", operator.trim()]);
            }
            if json {
                args.insert(0, "--json");
            }
            let after = ran(&dir, &words(&args), &[(variable, os(b"\xff"))]);
            assert_eq!(after.status, Some(USAGE), "{variable}: {after:?}");
            assert_eq!(after.stderr(), named(variable), "{variable}");
            if json {
                assert_eq!(after.stdout().lines().count(), 1, "{variable}");
            } else {
                assert!(after.stdout.is_empty(), "{variable}: {:?}", after.stdout());
            }
            TcpListener::bind(("127.0.0.1", port))
                .unwrap_or_else(|error| panic!("{variable}: the port is not free: {error}"));
            assert_eq!(listing(), before, "{variable}: a file was written");
            assert!(!dir.join("state").exists(), "{variable}");
        }
    }
}

/// Reads the line back: the escaped value between the quotes as bytes, and the count after it.
fn read_back(name: &str, line: &str) -> (Vec<u8>, usize) {
    let rest = line
        .strip_prefix(&format!("inbox: usage: {name} is not UTF-8: \""))
        .unwrap_or_else(|| panic!("{line:?}"));
    let rest = rest
        .strip_suffix('\n')
        .expect("the line ends with a line feed");
    let mut bytes = Vec::new();
    let mut chars = rest.char_indices();
    let mut end = None;
    while let Some((at, character)) = chars.next() {
        match character {
            '"' => {
                end = Some(at + 1);
                break;
            }
            '\\' => match chars.next().map(|(_, c)| c) {
                Some('n') => bytes.push(b'\n'),
                Some('r') => bytes.push(b'\r'),
                Some('t') => bytes.push(b'\t'),
                Some('"') => bytes.push(b'"'),
                Some('\\') => bytes.push(b'\\'),
                Some('x') => {
                    let high = chars.next().map(|(_, c)| c).expect("a digit");
                    let low = chars.next().map(|(_, c)| c).expect("a digit");
                    let digits: String = [high, low].iter().collect();
                    bytes.push(u8::from_str_radix(&digits, 16).expect("2 hex digits"));
                }
                other => panic!("an unknown escape {other:?} in {line:?}"),
            },
            other => {
                assert!(other.is_ascii_graphic() || other == ' ', "{line:?}");
                bytes.push(other as u8);
            }
        }
    }
    let after = &rest[end.expect("a closing quote")..];
    let more = match after {
        "" => 0,
        " (and 1 more byte)" => 1,
        other => other
            .strip_prefix(" (and ")
            .and_then(|count| count.strip_suffix(" more bytes)"))
            .and_then(|count| count.parse().ok())
            .filter(|count| *count > 1)
            .unwrap_or_else(|| panic!("{line:?}")),
    };
    (bytes, more)
}

/// Values of 1, 64, 65 and 4096 bytes; a line feed, a carriage return, a tab, an escape byte, a
/// DEL, a quote, a backslash; the 4 characters of an escape written as text beside the byte; a
/// value that is UTF-8 up to the cut and breaks inside a character there; one that is UTF-8 but
/// its last byte. Each is one line of printable ASCII under 400 bytes that reads back as the
/// first 64 bytes of the value and the count of the rest.
#[test]
fn the_line_reads_back_as_the_first_64_bytes_and_the_count_of_the_rest() {
    let dir = Dir::new("adv-var-line");
    let mut values: Vec<Vec<u8>> = vec![
        b"\xff".to_vec(),
        b"\\xFF\xff".to_vec(),
        b"\\\\xff\xff\"\\".to_vec(),
        b"\xff\n\r\t\x1b[2J\x7f\x80 ok".to_vec(),
        "a\u{e9}\u{20ac}".bytes().chain([0xff]).collect(),
    ];
    let mut cut = vec![b'a'; 63];
    cut.extend("\u{e9}".bytes());
    cut.push(0xff);
    values.push(cut);
    for length in [64, 65, 4096] {
        let mut value = vec![0xff];
        value.extend(std::iter::repeat_n(b'z', length - 1));
        values.push(value);
    }
    for value in values {
        let after = ran(&dir, &words(&["open"]), &[("INBOX_TLS_PIN", os(&value))]);
        let line = after.stderr();
        assert_eq!(after.status, Some(USAGE), "{line:?}");
        assert_eq!(after.lines(), 1, "{line:?}");
        assert!(line.len() < 400, "{} bytes: {line:?}", line.len());
        assert!(
            after
                .stderr
                .iter()
                .all(|byte| *byte == b'\n' || (0x20..0x7f).contains(byte)),
            "{line:?}"
        );
        let (shown, more) = read_back("INBOX_TLS_PIN", &line);
        let first = value.len().min(64);
        assert_eq!(shown, value[..first], "{line:?}");
        assert_eq!(more, value.len() - first, "{line:?}");
    }
}

/// `--json` before and after the subcommand, for each of the 4: exit 2, one document on standard
/// output with the members of the sibling's document for an argument (`error`, `detail`), the
/// detail the line without `inbox: usage: `, and the line on standard error.
#[test]
fn the_usage_document_names_each_variable_before_and_after_the_subcommand() {
    let (dir, operator) = keys_only("adv-var-json");
    let operator = operator.trim().to_owned();
    let serve_key: Vec<&str> = vec!["--listen", "127.0.0.1:0", "--key-file", "keys.json"];
    let mut serve_backend = serve_key.clone();
    serve_backend.extend(["--operator-public-key", operator.as_str()]);
    let cases: [(&str, &str, Vec<&str>); 4] = [
        ("INBOX_URL", "open", vec![]),
        ("INBOX_TLS_PIN", "pending", vec![]),
        ("INBOX_OPERATOR_PUBLIC_KEY", "serve", serve_key),
        ("INBOX_BACKEND", "serve", serve_backend),
    ];
    for (variable, subcommand, rest) in cases {
        for json_first in [true, false] {
            let mut args = vec![subcommand];
            args.extend(rest.iter().copied());
            if json_first {
                args.insert(0, "--json");
            } else {
                args.push("--json");
            }
            let after = ran(&dir, &words(&args), &[(variable, os(b"\x80"))]);
            assert_eq!(after.status, Some(USAGE), "{args:?}: {after:?}");
            let stdout = after.stdout();
            assert_eq!(stdout.lines().count(), 1, "{args:?}: {stdout:?}");
            let document = json::parse(stdout.trim_end()).expect("one JSON document");
            let Value::Object(members) = &document else {
                panic!("{stdout:?}")
            };
            let keys: Vec<&str> = members.iter().map(|(key, _)| key.as_str()).collect();
            assert_eq!(keys, ["error", "detail"], "{stdout:?}");
            assert_eq!(text(&document, "error"), "usage");
            let detail = format!("{variable} is not UTF-8: \"\\x80\"");
            assert_eq!(text(&document, "detail"), detail, "{args:?}");
            assert_eq!(after.stderr(), format!("inbox: usage: {detail}\n"));
        }
    }
}

/// 2 bad variables the command reads, 20 runs each: always the same one (no order of a map).
#[test]
fn of_2_bad_variables_the_same_is_named_over_20_runs() {
    let dir = Dir::new("adv-var-twenty");
    for _ in 0..20 {
        let after = ran(
            &dir,
            &words(&["wake"]),
            &[("INBOX_TLS_PIN", os(b"\xfa")), ("INBOX_URL", os(b"\xfb"))],
        );
        assert!(
            after.stderr().starts_with("inbox: usage: INBOX_URL is not"),
            "{:?}",
            after.stderr()
        );
        let after = ran(
            &dir,
            &words(&[
                "serve",
                "--listen",
                "127.0.0.1:0",
                "--key-file",
                "keys.json",
            ]),
            &[
                ("INBOX_BACKEND", os(b"\xfa")),
                ("INBOX_OPERATOR_PUBLIC_KEY", os(b"\xfb")),
            ],
        );
        assert!(
            after
                .stderr()
                .starts_with("inbox: usage: INBOX_OPERATOR_PUBLIC_KEY is not"),
            "{:?}",
            after.stderr()
        );
    }
}

/// A bad variable beside another usage error. An unknown flag: clap's message, as without the
/// variable. A missing argument: clap reads the variables before it checks for one, so the
/// variable is named, in one line.
#[test]
fn a_bad_variable_beside_another_usage_error() {
    let dir = Dir::new("adv-var-other");
    for args in [&["pending", "--nope"][..], &["--nope", "open"], &["nope"]] {
        let line = words(args);
        let before = ran(&dir, &line, &[]);
        let after = ran(&dir, &line, &[("INBOX_URL", os(b"\xff"))]);
        assert_eq!(after, before, "{args:?}: {:?}", after.stderr());
    }
    for args in [&["read"][..], &["refuse", "m"], &["post", "--to", "x"]] {
        let after = ran(&dir, &words(args), &[("INBOX_URL", os(b"\xff"))]);
        assert_eq!(after.status, Some(USAGE), "{args:?}");
        assert_eq!(after.stderr(), named("INBOX_URL"), "{args:?}");
    }
}
