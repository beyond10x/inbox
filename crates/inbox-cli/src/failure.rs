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
/// The exit status of a local failure: a file that cannot be read, written or created.
pub const LOCAL: i32 = 4;

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
    /// The service could not be reached, or its answer could not be read.
    Transport(String),
    /// A local file could not be read, written or created.
    Local(String),
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
            Self::Refused { .. } | Self::Unreadable { .. } => REFUSED,
            Self::Usage(_) => USAGE,
            Self::Transport(_) => TRANSPORT,
            Self::Local(_) => LOCAL,
            Self::After { cause, .. } => cause.status(),
        }
    }

    /// The line written to standard error.
    pub fn message(&self) -> String {
        match self {
            Self::Refused { name, status, .. } => format!("refused: {name} ({status})"),
            Self::Unreadable { detail, .. } => format!("unreadable outbox entry: {detail}"),
            Self::Usage(detail) => format!("usage: {detail}"),
            Self::Transport(detail) => format!("transport: {detail}"),
            Self::Local(detail) => detail.clone(),
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
            Self::Usage(detail) | Self::Transport(detail) | Self::Local(detail) => {
                json::member(out, "error");
                json::push_text(
                    out,
                    match self {
                        Self::Usage(_) => "usage",
                        Self::Transport(_) => "transport",
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
