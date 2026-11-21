//! How a command fails, and the exit status of each kind of failure.

use inbox::server::json::{self, Value};

use crate::output;

/// The exit status of a refusal: the service refused the request (a declared refusal of the
/// model, or the service's own `refused` answer).
pub const REFUSED: i32 = 1;
/// The exit status of wrong usage: a flag or a value the command does not take.
pub const USAGE: i32 = 2;
/// The exit status of a transport failure: the service could not be reached, or its answer could
/// not be read.
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
    /// A flag or a value the command does not take.
    Usage(String),
    /// The service could not be reached, or its answer could not be read.
    Transport(String),
    /// A local file could not be read, written or created.
    Local(String),
}

impl Failure {
    /// The exit status.
    pub fn status(&self) -> i32 {
        match self {
            Self::Refused { .. } => REFUSED,
            Self::Usage(_) => USAGE,
            Self::Transport(_) => TRANSPORT,
            Self::Local(_) => LOCAL,
        }
    }

    /// The line written to standard error.
    pub fn message(&self) -> String {
        match self {
            Self::Refused { name, status, .. } => format!("refused: {name} ({status})"),
            Self::Usage(detail) => format!("usage: {detail}"),
            Self::Transport(detail) => format!("transport: {detail}"),
            Self::Local(detail) => detail.clone(),
        }
    }

    /// The JSON document written to standard output with `--json`.
    pub fn document(&self) -> String {
        let mut out = String::from("{");
        match self {
            Self::Refused {
                name,
                status,
                answer,
            } => {
                json::member(&mut out, "error");
                json::push_text(&mut out, name);
                json::member(&mut out, "status");
                json::push_integer(&mut out, i64::from(*status));
                json::member(&mut out, "answer");
                output::write(answer, &mut out);
            }
            Self::Usage(detail) | Self::Transport(detail) | Self::Local(detail) => {
                json::member(&mut out, "error");
                json::push_text(
                    &mut out,
                    match self {
                        Self::Usage(_) => "usage",
                        Self::Transport(_) => "transport",
                        _ => "local",
                    },
                );
                json::member(&mut out, "detail");
                json::push_text(&mut out, detail);
            }
        }
        out.push('}');
        out
    }
}
