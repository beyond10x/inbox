//! `inbox serve`: the service of `crates/inbox-server` over `inbox_core::MemoryBackend`, until
//! SIGINT or SIGTERM.

use std::io::ErrorKind;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use inbox::server::{http, json};
use inbox_core::MemoryBackend;
use inbox_server::{Config, Server, StartedClock};
use inbox_verify::{Instant, KeyRing, PublicKey};
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

use crate::failure::Failure;
use crate::output::{print_document, print_text, warn};

/// The backends `INBOX_BACKEND` may name in this stage.
const BACKENDS: &[&str] = &["memory"];

/// The settings of `Config`, as the flags give them.
#[derive(Debug, Clone)]
pub struct Settings<'a> {
    /// `--listen`.
    pub listen: SocketAddr,
    /// `--key-file`.
    pub key_file: &'a Path,
    /// `--operator-public-key`.
    pub operator_public_key: &'a str,
    /// `--max-connections`.
    pub max_connections: usize,
    /// `--request-timeout`, seconds.
    pub request_timeout: u64,
    /// `--max-request-bytes`.
    pub max_request_bytes: usize,
    /// `--max-messages`.
    pub max_messages: usize,
    /// `--max-wait`, seconds.
    pub max_wait: u64,
    /// `--sweep-interval`, seconds.
    pub sweep_interval: u64,
    /// `--answer-retention`, seconds.
    pub answer_retention: u64,
    /// `--clock-start`, RFC 3339; the system clock where absent.
    pub clock_start: Option<&'a str>,
}

/// Serves until SIGINT or SIGTERM. The first line on standard output is the URL the service
/// answers at (with `json`, one JSON document naming the bound address and that URL); nothing
/// else is written to standard output.
///
/// # Errors
///
/// [`Failure::Usage`] for an `INBOX_BACKEND` other than `memory`, an operator public key that is
/// not one, a body limit over the generated reader's, a `--clock-start` that is no RFC 3339
/// instant, or an address that is not loopback;
/// [`Failure::Local`] for a key file that cannot be read or does not read as one, and an address
/// that cannot be bound.
pub fn serve(settings: &Settings<'_>, backend: Option<&str>, json: bool) -> Result<(), Failure> {
    if let Some(backend) = backend.filter(|backend| !BACKENDS.contains(backend)) {
        return Err(Failure::Usage(format!(
            "INBOX_BACKEND is `{backend}`: this stage has the backend `memory` only"
        )));
    }
    let operator = PublicKey::from_base64(settings.operator_public_key)
        .map_err(|error| Failure::Usage(format!("--operator-public-key: {error}")))?;
    if settings.max_request_bytes > http::MAX_BODY {
        return Err(Failure::Usage(format!(
            "--max-request-bytes {} is over {}, the most the generated reader reads",
            settings.max_request_bytes,
            http::MAX_BODY
        )));
    }
    let shown = settings.key_file.display();
    let text = std::fs::read_to_string(settings.key_file)
        .map_err(|error| Failure::Local(format!("the key file {shown}: {error}")))?;
    let keys = KeyRing::parse(&text, &operator)
        .map_err(|error| Failure::Local(format!("the key file {shown}: {error}")))?;
    for ignored in keys.ignored() {
        warn(&format!(
            "inbox serve: {shown}: the {:?} entry `{}` is ignored: {:?}",
            ignored.kind, ignored.id, ignored.reason
        ));
    }
    let mut config = Config::new(settings.listen, keys);
    config.max_connections = settings.max_connections;
    config.request_timeout = Duration::from_secs(settings.request_timeout);
    config.max_body = settings.max_request_bytes;
    config.max_messages = settings.max_messages;
    config.max_wait = Duration::from_secs(settings.max_wait);
    config.sweep_interval = Duration::from_secs(settings.sweep_interval);
    config.answer_retention = Duration::from_secs(settings.answer_retention);
    if let Some(start) = settings.clock_start {
        let start = Instant::parse(start)
            .map_err(|error| Failure::Usage(format!("--clock-start: {error}")))?;
        config.clock = Arc::new(StartedClock::new(start));
        warn(&format!(
            "inbox serve: the service's clock starts at {}, not at the system clock",
            inbox_server::format_reading(start)
        ));
    }
    let server =
        Server::bind(config, MemoryBackend::new()).map_err(|error| match error.kind() {
            ErrorKind::InvalidInput => Failure::Usage(format!("--listen: {error}")),
            _ => Failure::Local(format!("--listen {}: {error}", settings.listen)),
        })?;
    // The handlers are in place before the first line is printed: a signal sent once the
    // address is known stops the service instead of killing the process.
    let mut signals = Signals::new([SIGINT, SIGTERM])
        .map_err(|error| Failure::Local(format!("the signal handlers: {error}")))?;
    let address = server.local_addr();
    let url = format!("http://{address}");
    if json {
        let mut out = String::from("{");
        json::member(&mut out, "address");
        json::push_text(&mut out, &address.to_string());
        json::member(&mut out, "url");
        json::push_text(&mut out, &url);
        out.push('}');
        print_document(&out);
    } else {
        print_text(&url);
    }
    let stopper = server.stopper();
    let handle = signals.handle();
    // The signal thread stops the service first, then writes its line. It is never joined: the
    // process ends once `run` returns, whether that write has finished, failed (a reader that is
    // gone) or is still blocked (a full pipe whose reader is alive), and the line is then lost.
    std::thread::Builder::new()
        .name("inbox-signals".to_owned())
        .spawn(move || {
            if let Some(signal) = signals.forever().next() {
                stopper.stop();
                warn(&format!("inbox serve: signal {signal}: stopping"));
            }
        })
        .map_err(|error| Failure::Local(format!("the signal thread: {error}")))?;
    let served = server.run();
    handle.close();
    served.map_err(|error| Failure::Local(format!("serving: {error}")))
}
