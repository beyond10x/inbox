//! The HTTP service of the inbox: a listener in front of the `server` feature of
//! `generated/rust/inbox` (`docs/design/service.md`).
//!
//! The generated route table, JSON codecs and `dispatch` answer every command and view; this crate
//! writes no second route table and no second codec. It adds what the generated surface leaves
//! out by decision and what the specification leaves to the service:
//!
//! - concurrency: one thread per connection, at most [`Config::max_connections`] at once, each
//!   request run inside the backend's [`inbox_core::Exclusive`] section over a system built for
//!   that request's caller;
//! - the caller: every request carries a request signature ([`KEY_ID_HEADER`],
//!   [`TIMESTAMP_HEADER`], [`SIGNATURE_HEADER`]) checked at the [`Clock`]'s reading; its key id
//!   resolves to the principal that is the caller's `principal` and `inbox`; a request without a
//!   valid one is answered `401 {"refused": "unauthenticated"}`;
//! - a body in which an object carries a member name twice is refused before any command runs;
//! - the verifier port: [`RingVerifier`] over `inbox-verify`'s key ring;
//! - the read rule, the wait on `Pending`, the expiry task ([`sweep`]) and the limits;
//! - the epoch: every answer carries [`EPOCH_HEADER`], a value made once per bound listener, so a
//!   caller tells an answer of this start from one of an earlier start.
//!
//! [`Server::bind`] binds a loopback address (port 0 for an ephemeral one, [`Server::local_addr`]
//! names it), and [`Server::run`] serves until a [`Stopper`] tells it to stop.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod auth;
mod clock;
mod config;
mod expiry;
mod listener;
mod service;
mod signals;

pub use auth::{KEY_ID_HEADER, RingVerifier, SIGNATURE_HEADER, TIMESTAMP_HEADER, request_headers};
pub use clock::{Clock, ManualClock, SystemClock, format_instant};
pub use config::Config;
pub use expiry::sweep;
pub use listener::{EPOCH_HEADER, Running, Server, Stopper};
pub use service::Backend;
