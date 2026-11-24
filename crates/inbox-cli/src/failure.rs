//! How a command fails, and the exit status of each kind of failure.

use inbox::server::json::{self, Value};

use crate::output::{self, Done};

/// The exit status of a refusal: the service refused the request (a declared refusal of the
/// model, or the service's own `refused` answer), or the outbox file holds an entry `repost`
/// cannot read.
pub const REFUSED: i32 = 1;
/// The exit status of wrong usage: a flag or a value the command does not take.
pub const USAGE: i32 = 2;
/// The exit status of a transport failure: the service could not be reached, or its answer could
/// not be read or was neither a success nor a refusal.
pub const TRANSPORT: i32 = 3;
/// The exit status of a local failure: a file that cannot be read, written or created, or no
/// `connectors` on `PATH` for `inbox bridge`.
pub const LOCAL: i32 = 4;
/// The exit status of an `inbox wake` or `inbox bridge` that found another wake or bridge of the
/// same inbox with the same outbox file running on this host: it sent nothing.
pub const ANOTHER_WAKE: i32 = 5;

/// Why a command failed.
#[derive(Debug)]
pub enum Failure {
    /// The service refused: `name` is the model's error name for a declared refusal
    /// (`inbox.messaging.UnknownInbox`) and the service's own `refused` text otherwise
    /// (`unauthenticated`).
    Refused {
        /// The error name.
        name: String,
        /// The HTTP status of the answer.
        status: u16,
        /// The answer's body.
        answer: Value,
    },
    /// The outbox file holds entries that do not read as an envelope; `answer` is what the
    /// command did with the rest.
    Unreadable {
        /// Which entries, and why.
        detail: String,
        /// What the command did with the readable entries.
        answer: Value,
    },
    /// A flag or a value the command does not take.
    Usage(String),
    /// The service could not be reached, or its answer could not be read whole (no answer, or one
    /// cut before the end its head names).
    Transport(String),
    /// A whole answer that is neither a success nor a refusal of the service (a `3xx`, an error
    /// without a `refused` text, a body that is not JSON). Said and exited as
    /// [`Failure::Transport`]; apart from it so that `inbox wake` tries again only what its
    /// status says may pass (`408`, from `500` on).
    Unexpected {
        /// The status of the answer.
        status: u16,
        /// What it was.
        detail: String,
    },
    /// The TLS handshake was refused by the client: the service's certificate is not the one the
    /// pin names, or the peer speaks no TLS. Said and exited as [`Failure::Transport`]; apart
    /// from it so that `inbox wake` does not try again against such a service.
    Certificate(String),
    /// A local file could not be read, written or created; for `inbox bridge` also: no
    /// `connectors` on `PATH`, or a post it made that cannot be recorded.
    Local(String),
    /// `inbox wake` and `inbox bridge`: another wake or bridge of the caller's inbox with the same
    /// outbox file runs on this host (it holds the lock of the wake).
    AnotherWake(String),
    /// `inbox wake`: the service answered that the caller's inbox is not open, and the wake does
    /// not open it: this host has no record of an `inbox open` of it (never opened here, or closed
    /// by `inbox close`), the service refused the reconnect's open, or it is still not open right
    /// after an open that succeeded. Exits as a refusal.
    NotOpen(String),
    /// `inbox wake --follow` and `inbox bridge`: this host's record of the caller's `inbox open`
    /// names another adapter than the mode's own (`terminal`, `slack`). Exits as a refusal,
    /// before the lock and before any request.
    OtherAdapter(String),
    /// `inbox bridge --for`: the service answered a request `401`, which it gives alike for a key
    /// entry and for a delegation that is missing or lapsed in its key file. Exits as a refusal;
    /// the line names the key id, the delegation id and the 2 commands that add them.
    KeyRefused(String),
    /// The command did its work at the service, and what it took from there is printed, but a
    /// step after it failed: `done` is printed, then `cause` is reported and decides the exit.
    After {
        /// What the command printed.
        done: Done,
        /// The step that failed after it.
        cause: Box<Failure>,
    },
}

impl Failure {
    /// The exit status.
    pub fn status(&self) -> i32 {
        match self {
            Self::Refused { .. }
            | Self::Unreadable { .. }
            | Self::NotOpen(_)
            | Self::OtherAdapter(_)
            | Self::KeyRefused(_) => REFUSED,
            Self::Usage(_) => USAGE,
            Self::Transport(_) | Self::Certificate(_) | Self::Unexpected { .. } => TRANSPORT,
            Self::Local(_) => LOCAL,
            Self::AnotherWake(_) => ANOTHER_WAKE,
            Self::After { cause, .. } => cause.status(),
        }
    }

    /// The line written to standard error.
    pub fn message(&self) -> String {
        match self {
            Self::Refused { name, status, .. } => format!("refused: {name} ({status})"),
            Self::Unreadable { detail, .. } => format!("unreadable outbox entry: {detail}"),
            Self::Usage(detail) => format!("usage: {detail}"),
            Self::Transport(detail)
            | Self::Certificate(detail)
            | Self::Unexpected { detail, .. } => format!("transport: {detail}"),
            Self::Local(detail)
            | Self::AnotherWake(detail)
            | Self::NotOpen(detail)
            | Self::OtherAdapter(detail)
            | Self::KeyRefused(detail) => detail.clone(),
            Self::After { cause, .. } => cause.message(),
        }
    }

    /// The text written to standard output without `--json`, before the message.
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::After { done, .. } => Some(&done.text),
            _ => None,
        }
    }

    /// The JSON document written to standard output with `--json`.
    pub fn document(&self) -> String {
        let mut out = String::from("{");
        self.members(&mut out);
        out.push('}');
        out
    }

    fn members(&self, out: &mut String) {
        match self {
            Self::Refused {
                name,
                status,
                answer,
            } => {
                json::member(out, "error");
                json::push_text(out, name);
                json::member(out, "status");
                json::push_integer(out, i64::from(*status));
                json::member(out, "answer");
                output::write(answer, out);
            }
            Self::Unreadable { detail, answer } => {
                json::member(out, "error");
                json::push_text(out, "unreadable outbox entry");
                json::member(out, "detail");
                json::push_text(out, detail);
                json::member(out, "answer");
                output::write(answer, out);
            }
            Self::Usage(detail)
            | Self::Transport(detail)
            | Self::Certificate(detail)
            | Self::Unexpected { detail, .. }
            | Self::Local(detail)
            | Self::AnotherWake(detail)
            | Self::NotOpen(detail)
            | Self::OtherAdapter(detail)
            | Self::KeyRefused(detail) => {
                json::member(out, "error");
                json::push_text(
                    out,
                    match self {
                        Self::Usage(_) => "usage",
                        Self::Transport(_) | Self::Certificate(_) | Self::Unexpected { .. } => {
                            "transport"
                        }
                        Self::AnotherWake(_) => "another wake",
                        Self::NotOpen(_) => "inbox not open",
                        Self::OtherAdapter(_) => "another adapter",
                        Self::KeyRefused(_) => "key refused",
                        _ => "local",
                    },
                );
                json::member(out, "detail");
                json::push_text(out, detail);
            }
            Self::After { done, cause } => {
                cause.members(out);
                json::member(out, "done");
                out.push_str(&done.document);
            }
        }
    }
}
