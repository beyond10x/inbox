//! The cases of story:the-service-speaks-tls-with-a-pinned-certificate against the built binary:
//! `inbox tls new`, `inbox serve --tls-file` and the client's `--tls-pin`. Every certificate and
//! key is made at run time with `inbox tls new`; no pin is written into this file.

use crate::support;

use std::fs;
use std::io::Read;
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Duration;

use inbox::server::json::Value;
use support::{
    BOB, LOCAL, REFUSED, Run, Service, TRANSPORT, USAGE, accept_within, ids, keys_only, run, text,
};

/// `inbox tls new --out <file>` in `dir`, answering the pin it printed.
fn tls_new(dir: &Path, file: &str) -> String {
    run(dir, &["tls", "new", "--out", file], &[])
        .ok()
        .stdout
        .trim()
        .to_owned()
}

/// `inbox serve` over `keys.json` in `dir` with the certificate of `tls_file`.
fn serve_tls(dir: &Path, operator_public_key: &str, tls_file: &str) -> Service {
    Service::start(
        dir,
        &[
            "--listen",
            "127.0.0.1:0",
            "--key-file",
            "keys.json",
            "--operator-public-key",
            operator_public_key,
            "--tls-file",
            tls_file,
        ],
        &[],
    )
}

/// A client command of `who` (`alice` or `bob`) at `url`, with `pin` as `INBOX_TLS_PIN` when
/// there is one.
fn client(dir: &Path, url: &str, pin: Option<&str>, who: &str, args: &[&str]) -> Run {
    let key = format!("{who}.key");
    let outbox = dir.join(format!("{who}.outbox"));
    let outbox = outbox.to_str().expect("a UTF-8 path");
    let mut env = vec![
        ("INBOX_URL", url),
        ("INBOX_SECRET_KEY_FILE", key.as_str()),
        ("INBOX_OUTBOX", outbox),
    ];
    if let Some(pin) = pin {
        env.push(("INBOX_TLS_PIN", pin));
    }
    run(dir, args, &env)
}

/// The acceptance of the story: a client given the pin of the certificate gets `posted` from an
/// `inbox serve` started with it, and a client given another pin exits 3 while the recipient's
/// `Pending` stays empty.
#[test]
fn a_client_with_the_pin_posts_over_tls_and_one_with_another_pin_exits_3() {
    let (dir, operator) = keys_only("tls-acceptance");
    let pin = tls_new(&dir.path, "tls.json");
    let other = tls_new(&dir.path, "other.json");
    assert_ne!(pin, other, "two certificates, two pins");
    let service = serve_tls(&dir.path, &operator, "tls.json");
    assert!(
        service.url.starts_with("https://127.0.0.1:"),
        "{}",
        service.url
    );
    let url = service.url.clone();
    client(&dir.path, &url, Some(&pin), "bob", &["open"]).ok();

    let refused = client(
        &dir.path,
        &url,
        Some(&other),
        "alice",
        &[
            "post",
            "--to",
            BOB,
            "--body",
            "over a certificate I did not pin",
        ],
    );
    assert_eq!(refused.status, TRANSPORT, "{refused:#?}");
    assert!(refused.stderr.contains("pin"), "{refused:#?}");
    let pending = client(&dir.path, &url, Some(&pin), "bob", &["pending", "--json"]).ok();
    assert!(ids(&pending.json()).is_empty(), "{pending:#?}");

    let posted = client(
        &dir.path,
        &url,
        Some(&pin),
        "alice",
        &[
            "post",
            "--to",
            BOB,
            "--body",
            "over the pinned certificate",
            "--json",
        ],
    )
    .ok();
    let answer = posted.json();
    let outcome = text(answer.member("answer").expect("an answer"), "outcome");
    assert_eq!(outcome, "posted", "{posted:#?}");
    let message = text(&answer, "message_id");
    // The post that never reached the service is kept marked unsent: the reconnect of this
    // command posts it, over the pinned certificate (story:the-client-reconnects-after-a-restart).
    let reposted = match answer
        .member("reconnected")
        .and_then(|reconnect| reconnect.member("repost"))
        .and_then(|repost| repost.member("reposted"))
    {
        Some(Value::Array(entries)) => entries.clone(),
        other => panic!("the reconnect reposted {other:?}: {posted:#?}"),
    };
    assert_eq!(reposted.len(), 1, "{posted:#?}");
    assert_eq!(reposted[0].member("arrived"), Some(&Value::Bool(true)));
    let kept = text(&reposted[0], "message_id");
    let pending = client(&dir.path, &url, Some(&pin), "bob", &["pending", "--json"]).ok();
    let mut held = ids(&pending.json());
    held.sort();
    let mut both = vec![message, kept];
    both.sort();
    assert_eq!(held, both);
}

/// The `413` of a post over `--max-request-bytes` arrives over `https://` as over `http://`: exit
/// 1 with the same output, not a transport failure. The service checks that limit once it has
/// read the whole body, so the client has sent everything when the answer comes.
#[test]
fn the_413_of_a_post_over_max_request_bytes_arrives_over_https_as_over_http() {
    let (dir, operator) = keys_only("tls-413");
    let pin = tls_new(&dir.path, "tls.json");
    // The longest body a message takes, of a character JSON writes as six bytes: a request of
    // about 393 KB, against a limit of 1 KiB.
    fs::write(dir.join("big.txt"), [1_u8; 65_536]).expect("the body is written");
    let serve = |tls: &[&str]| {
        let mut args = vec![
            "--listen",
            "127.0.0.1:0",
            "--key-file",
            "keys.json",
            "--operator-public-key",
            &operator,
            "--max-request-bytes",
            "1024",
        ];
        args.extend_from_slice(tls);
        Service::start(&dir.path, &args, &[])
    };
    let plain = serve(&[]);
    let over_tls = serve(&["--tls-file", "tls.json"]);
    let post = ["post", "--to", BOB, "--body-file", "big.txt", "--json"];
    let over_http = client(&dir.path, &plain.url, None, "alice", &post);
    assert_eq!(over_http.status, REFUSED, "{over_http:#?}");
    let over_https = client(&dir.path, &over_tls.url, Some(&pin), "alice", &post);
    assert_eq!(over_https.status, REFUSED, "{over_https:#?}");
    assert_eq!(over_https.stdout, over_http.stdout);
}

/// A plain TCP stand-in on 127.0.0.1:0 that keeps every byte a client sends over at most `most`
/// connections, each of which it waits `wait` for, and writes `answer` back once a connection
/// has sent a few bytes; answers the bytes of each connection.
fn recording_stand_in(
    answer: &'static [u8],
    most: usize,
    wait: Duration,
) -> (String, std::thread::JoinHandle<Vec<Vec<u8>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
    let address = listener.local_addr().expect("an address");
    let handle = std::thread::spawn(move || {
        let mut received = Vec::new();
        while received.len() < most
            && let Some(mut stream) = accept_within(&listener, wait)
        {
            use std::io::Write;
            let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
            let mut bytes = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => bytes.extend_from_slice(&buffer[..read]),
                }
                if bytes.len() > 4 && !answer.is_empty() {
                    let _ = stream.write_all(answer);
                }
            }
            received.push(bytes);
        }
        received
    });
    (address.to_string(), handle)
}

/// A client given a pin never falls back to plain HTTP: an `http://` URL beside a pin is wrong
/// usage, and the stand-in at that URL receives no byte.
#[test]
fn a_client_given_a_pin_sends_nothing_to_an_http_url() {
    let (dir, _operator) = keys_only("tls-pin-http");
    let pin = tls_new(&dir.path, "tls.json");
    let (address, stand_in) = recording_stand_in(b"", 1, Duration::from_secs(1));
    let url = format!("http://{address}");
    let refused = client(
        &dir.path,
        &url,
        Some(&pin),
        "alice",
        &["post", "--to", BOB, "--body", "never sent"],
    );
    assert_eq!(refused.status, USAGE, "{refused:#?}");
    let received = stand_in.join().expect("the stand-in ends");
    assert!(
        received.is_empty(),
        "the stand-in was reached: {received:?}"
    );
    assert!(
        fs::read_to_string(dir.join("alice.outbox"))
            .unwrap_or_default()
            .is_empty(),
        "nothing was posted, so nothing is kept"
    );
}

/// A server that speaks no TLS at an `https://` URL: the client sends its TLS hello, gets no
/// certificate, exits 3, and no request line, header or body reached the stand-in.
#[test]
fn a_client_given_a_pin_sends_no_request_to_a_server_without_tls() {
    let (dir, _operator) = keys_only("tls-pin-plain");
    let pin = tls_new(&dir.path, "tls.json");
    let (address, stand_in) = recording_stand_in(
        b"HTTP/1.1 202 Accepted\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
        1,
        Duration::from_secs(20),
    );
    let url = format!("https://{address}");
    let refused = client(
        &dir.path,
        &url,
        Some(&pin),
        "alice",
        &["post", "--to", BOB, "--body", "never sent in the clear"],
    );
    assert_eq!(refused.status, TRANSPORT, "{refused:#?}");
    let received = stand_in.join().expect("the stand-in ends");
    assert_eq!(received.len(), 1, "one connection: {received:?}");
    let bytes = &received[0];
    for needle in [
        &b"POST"[..],
        b"HTTP/1.1",
        b"Inbox-Signature",
        b"Inbox-Key-Id",
        b"never sent in the clear",
    ] {
        assert!(
            !bytes.windows(needle.len()).any(|window| window == needle),
            "the stand-in received {:?}",
            String::from_utf8_lossy(needle)
        );
    }
}

/// An `https://` URL needs a pin: without one the client refuses before it connects.
#[test]
fn an_https_url_without_a_pin_is_wrong_usage() {
    let (dir, _operator) = keys_only("tls-no-pin");
    let (address, stand_in) = recording_stand_in(b"", 1, Duration::from_secs(1));
    let url = format!("https://{address}");
    let refused = client(&dir.path, &url, None, "alice", &["pending"]);
    assert_eq!(refused.status, USAGE, "{refused:#?}");
    assert!(refused.stderr.contains("INBOX_TLS_PIN"), "{refused:#?}");
    assert!(stand_in.join().expect("the stand-in ends").is_empty());
}

/// A pin that is not 44 characters of canonical base64 is wrong usage.
#[test]
fn a_pin_that_is_not_a_sha_256_in_base64_is_wrong_usage() {
    let (dir, _operator) = keys_only("tls-bad-pin");
    let pin = tls_new(&dir.path, "tls.json");
    let short = &pin[..43];
    let url_safe = pin.replace('+', "-").replace('/', "_");
    for wrong in [short, "", &format!("{pin}A"), "not a pin"] {
        let refused = client(
            &dir.path,
            "https://127.0.0.1:9",
            Some(wrong),
            "alice",
            &["pending"],
        );
        assert_eq!(refused.status, USAGE, "{wrong:?}: {refused:#?}");
    }
    if url_safe != pin {
        let refused = client(
            &dir.path,
            "https://127.0.0.1:9",
            Some(&url_safe),
            "alice",
            &["pending"],
        );
        assert_eq!(refused.status, USAGE, "{refused:#?}");
    }
}

/// `inbox tls new` writes its secret with mode 0600, never overwrites a file, prints the pin
/// (44 characters of base64, as `--json` names it) and nothing of the secret.
#[test]
fn tls_new_writes_a_0600_file_never_overwrites_it_and_prints_only_the_pin() {
    let (dir, _operator) = keys_only("tls-new");
    let done = run(
        &dir.path,
        &["tls", "new", "--out", "tls.json", "--json"],
        &[],
    )
    .ok();
    let document = done.json();
    let pin = text(&document, "pin");
    assert_eq!(pin.len(), 44, "{pin}");
    assert!(pin.ends_with('='), "{pin}");
    assert_eq!(text(&document, "file"), "tls.json");
    let path = dir.join("tls.json");
    let mode = fs::metadata(&path).expect("written").permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let secret = fs::read_to_string(&path).expect("readable");
    let seed = text(
        &inbox::server::json::parse(secret.trim()).expect("JSON"),
        "seed",
    );
    assert!(!done.stdout.contains(&seed) && !done.stderr.contains(&seed));
    let before = fs::read(&path).expect("readable");
    let again = run(&dir.path, &["tls", "new", "--out", "tls.json"], &[]);
    assert_eq!(again.status, LOCAL, "{again:#?}");
    assert!(again.stderr.contains("never overwrites"), "{again:#?}");
    assert_eq!(fs::read(&path).expect("readable"), before, "unchanged");
}

/// `inbox serve` refuses a TLS file that grants a permission to its group or to others, and one
/// that is not a TLS file, before it binds.
#[test]
fn serve_refuses_a_tls_file_others_can_read_or_that_is_no_tls_file() {
    let (dir, operator) = keys_only("tls-serve-refused");
    tls_new(&dir.path, "tls.json");
    let path = dir.join("tls.json");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("chmod");
    let args = |file: &'static str| {
        vec![
            "--listen".to_owned(),
            "127.0.0.1:0".to_owned(),
            "--key-file".to_owned(),
            "keys.json".to_owned(),
            "--operator-public-key".to_owned(),
            operator.clone(),
            "--tls-file".to_owned(),
            file.to_owned(),
        ]
    };
    let open = args("tls.json");
    let open: Vec<&str> = open.iter().map(String::as_str).collect();
    let refused = support::serve_refused(&dir.path, &open, &[]);
    assert_eq!(refused.status, LOCAL, "{refused:#?}");
    assert!(refused.stderr.contains("0600"), "{refused:#?}");
    let other = args("alice.key");
    let other: Vec<&str> = other.iter().map(String::as_str).collect();
    let refused = support::serve_refused(&dir.path, &other, &[]);
    assert_eq!(refused.status, LOCAL, "{refused:#?}");
    assert!(refused.stderr.contains("not a TLS file"), "{refused:#?}");
}
