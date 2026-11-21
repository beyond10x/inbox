//! What the embedding sets: the address, the key file, the clock and the limits.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use inbox::server::http;
use inbox_verify::KeyRing;

use crate::clock::{Clock, SystemClock};

/// The configuration of one service. Every field is the embedding's to set; [`Config::new`] fills
/// in this crate's defaults (`docs/design/service.md`, the limits).
#[derive(Debug, Clone)]
pub struct Config {
    /// The address to bind: a loopback address only in this stage. Port 0 binds an ephemeral port.
    pub address: SocketAddr,
    /// The key file, as the operator key signed it: who each request's key id is.
    pub keys: Arc<KeyRing>,
    /// Where the service reads the time.
    pub clock: Arc<dyn Clock>,
    /// The most connections served at once; the listener accepts no further one until one ends.
    /// 0 is read as 1.
    pub max_connections: usize,
    /// The longest a request may take to arrive in full, counted from the accept. A connection
    /// whose request has not arrived by then is answered `408` and closed, and gives its slot
    /// back. Beside it the generated reader drops a connection 1 s without a byte.
    pub request_timeout: Duration,
    /// The longest body read, in bytes; a longer one is answered `413`. At most the generated
    /// reader's own bound, [`http::MAX_BODY`], which refuses a longer one first.
    pub max_body: usize,
    /// The message count at which a post is answered `503` before the command runs.
    pub max_messages: usize,
    /// The longest a `wait` on `Pending` is held open.
    pub max_wait: Duration,
    /// How often the expiry task reads the clock and expires the due messages; at least 1 ms.
    pub sweep_interval: Duration,
    /// How long an answer by silence waits for its sender: `keep_until` is the sweep's reading
    /// plus this, to the second.
    pub answer_retention: Duration,
}

impl Config {
    /// The default connections at once.
    pub const MAX_CONNECTIONS: usize = 64;
    /// The default time a request may take to arrive.
    pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
    /// The default message count.
    pub const MAX_MESSAGES: usize = 10_000;
    /// The default longest wait.
    pub const MAX_WAIT: Duration = Duration::from_secs(60);
    /// The default sweep interval.
    pub const SWEEP_INTERVAL: Duration = Duration::from_secs(10);
    /// The default answer retention.
    pub const ANSWER_RETENTION: Duration = Duration::from_secs(24 * 3600);

    /// A service at `address` over `keys`, reading the system clock, with the default limits.
    #[must_use]
    pub fn new(address: SocketAddr, keys: KeyRing) -> Self {
        Self {
            address,
            keys: Arc::new(keys),
            clock: Arc::new(SystemClock),
            max_connections: Self::MAX_CONNECTIONS,
            request_timeout: Self::REQUEST_TIMEOUT,
            max_body: http::MAX_BODY,
            max_messages: Self::MAX_MESSAGES,
            max_wait: Self::MAX_WAIT,
            sweep_interval: Self::SWEEP_INTERVAL,
            answer_retention: Self::ANSWER_RETENTION,
        }
    }
}
