//! Adversary cases, pass 1, for story:the-service-speaks-tls-with-a-pinned-certificate, against
//! the built binary. Every certificate and key is made at run time; no pin is written into this
//! file.

use crate::support;

use std::io::Write;
use std::net::TcpStream;
use std::path::Path;
use std::time::{Duration, Instant};

use support::{BOB, Run, Service, TRANSPORT, USAGE, ids, keys_only, run, text};

fn tls_new(dir: &Path, file: &str) -> String {
    run(dir, &["tls", "new", "--out", file], &[])
        .ok()
        .stdout
        .trim()
        .to_owned()
}

fn serve_tls(dir: &Path, operator_public_key: &str, tls_file: &str, extra: &[&str]) -> Service {
    let mut args = vec![
        "--listen",
        "127.0.0.1:0",
        "--key-file",
        "keys.json",
        "--operator-public-key",
        operator_public_key,
        "--tls-file",
        tls_file,
    ];
    args.extend_from_slice(extra);
    Service::start(dir, &args, &[])
}

fn client(dir: &Path, url: &str, env: &[(&str, &str)], who: &str, args: &[&str]) -> Run {
    let key = format!("{who}.key");
    let outbox = dir.join(format!("{who}.outbox"));
    let outbox = outbox.to_str().expect("a UTF-8 path");
    let mut all = vec![
        ("INBOX_URL", url),
        ("INBOX_SECRET_KEY_FILE", key.as_str()),
        ("INBOX_OUTBOX", outbox),
    ];
    all.extend_from_slice(env);
    run(dir, args, &all)
}

/// After a wrong pin: exit 3 naming the pin, the envelope kept, and a later `repost` with the
/// right pin delivers it.
#[test]
fn adv_tls_a_post_refused_by_the_pin_is_kept_and_reposted_with_the_right_pin() {
    let (dir, operator) = keys_only("adv-tls-repost");
    let pin = tls_new(&dir.path, "tls.json");
    let other = tls_new(&dir.path, "other.json");
    let service = serve_tls(&dir.path, &operator, "tls.json", &[]);
    let url = service.url.clone();
    client(
        &dir.path,
        &url,
        &[("INBOX_TLS_PIN", &pin)],
        "bob",
        &["open"],
    )
    .ok();
    let refused = client(
        &dir.path,
        &url,
        &[("INBOX_TLS_PIN", &other)],
        "alice",
        &["post", "--to", BOB, "--body", "kept"],
    );
    assert_eq!(refused.status, TRANSPORT, "{refused:#?}");
    assert!(
        refused.stderr.contains("does not match the pin"),
        "{refused:#?}"
    );
    let kept = std::fs::read_to_string(dir.join("alice.outbox")).unwrap_or_default();
    assert_eq!(kept.lines().count(), 1, "the envelope is kept: {kept:?}");
    let reposted = client(
        &dir.path,
        &url,
        &[("INBOX_TLS_PIN", &pin)],
        "alice",
        &["repost", "--json"],
    )
    .ok();
    assert!(
        reposted.stdout.contains("\"arrived\":true"),
        "{reposted:#?}"
    );
    let pending = client(
        &dir.path,
        &url,
        &[("INBOX_TLS_PIN", &pin)],
        "bob",
        &["pending", "--json"],
    )
    .ok();
    assert_eq!(ids(&pending.json()).len(), 1, "{pending:#?}");
}

/// The flag wins over the environment, as for every flag with an environment variable.
#[test]
fn adv_tls_the_pin_flag_wins_over_the_environment() {
    let (dir, operator) = keys_only("adv-tls-flag");
    let pin = tls_new(&dir.path, "tls.json");
    let other = tls_new(&dir.path, "other.json");
    let service = serve_tls(&dir.path, &operator, "tls.json", &[]);
    let url = service.url.clone();
    let wrong_flag = client(
        &dir.path,
        &url,
        &[("INBOX_TLS_PIN", &pin)],
        "bob",
        &["open", "--tls-pin", &other],
    );
    assert_eq!(wrong_flag.status, TRANSPORT, "{wrong_flag:#?}");
    let right_flag = client(
        &dir.path,
        &url,
        &[("INBOX_TLS_PIN", &other)],
        "bob",
        &["open", "--tls-pin", &pin],
    );
    assert_eq!(right_flag.status, 0, "{right_flag:#?}");
    for spaced in [format!(" {pin}"), format!("{pin} "), format!("{pin}\n")] {
        let refused = client(
            &dir.path,
            &url,
            &[("INBOX_TLS_PIN", &spaced)],
            "bob",
            &["pending"],
        );
        assert_eq!(refused.status, USAGE, "{spaced:?}: {refused:#?}");
    }
    let upper = url.replacen("https://", "HTTPS://", 1);
    let refused = client(
        &dir.path,
        &upper,
        &[("INBOX_TLS_PIN", &pin)],
        "bob",
        &["pending"],
    );
    assert_eq!(refused.status, USAGE, "{refused:#?}");
    let empty = client(
        &dir.path,
        &url,
        &[("INBOX_TLS_PIN", "")],
        "bob",
        &["pending"],
    );
    assert_eq!(empty.status, USAGE, "{empty:#?}");
}

/// SIGTERM ends `inbox serve --tls-file` within its bound while one caller stalls in the
/// handshake and another has completed it and sent nothing.
#[test]
fn adv_tls_sigterm_ends_the_service_while_callers_stall_in_and_after_the_handshake() {
    let (dir, operator) = keys_only("adv-tls-stop");
    let pin = tls_new(&dir.path, "tls.json");
    let service = serve_tls(
        &dir.path,
        &operator,
        "tls.json",
        &["--request-timeout", "3600"],
    );
    let address: std::net::SocketAddr = service
        .url
        .strip_prefix("https://")
        .expect("an https URL")
        .parse()
        .expect("an address");
    let mut stalled = TcpStream::connect(address).expect("accepted");
    stalled
        .write_all(&[0x16, 0x03, 0x01, 0x02, 0x00, 0x01])
        .expect("the start of a hello");
    let config =
        inbox_server::tls::client_config(inbox_server::tls::Pin::parse(&pin).expect("a pin"))
            .expect("a client configuration");
    let quiet = inbox_server::tls::connect(&config, TcpStream::connect(address).expect("accepted"))
        .expect("the handshake completes");
    std::thread::sleep(Duration::from_millis(200));
    let started = Instant::now();
    let status = service.terminate();
    let took = started.elapsed();
    drop((stalled, quiet));
    assert!(status.success(), "{status:?}");
    assert!(took < Duration::from_secs(5), "SIGTERM took {took:?}");
}

/// `wait` over TLS ends on a post.
#[test]
fn adv_tls_wait_over_tls_returns_on_a_post() {
    let (dir, operator) = keys_only("adv-tls-wait");
    let pin = tls_new(&dir.path, "tls.json");
    let service = serve_tls(&dir.path, &operator, "tls.json", &["--max-wait", "20"]);
    let url = service.url.clone();
    client(
        &dir.path,
        &url,
        &[("INBOX_TLS_PIN", &pin)],
        "bob",
        &["open"],
    )
    .ok();
    let mut waiting = support::command(&dir.path);
    waiting
        .args(["wait", "--json"])
        .env("INBOX_URL", &url)
        .env("INBOX_TLS_PIN", &pin)
        .env("INBOX_SECRET_KEY_FILE", "bob.key")
        .env("INBOX_OUTBOX", dir.join("bob.outbox"))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let child = waiting.spawn().expect("wait starts");
    std::thread::sleep(Duration::from_millis(500));
    let posted = client(
        &dir.path,
        &url,
        &[("INBOX_TLS_PIN", &pin)],
        "alice",
        &["post", "--to", BOB, "--body", "wake", "--json"],
    )
    .ok();
    let message = text(&posted.json(), "message_id");
    let started = Instant::now();
    let output = child.wait_with_output().expect("wait ends");
    assert!(started.elapsed() < Duration::from_secs(10));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{stdout} {:?}",
        output.stderr
    );
    assert!(stdout.contains(&message), "{stdout}");
}
