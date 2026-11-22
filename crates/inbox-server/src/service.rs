//! One request, from what the reader read to the answer (`docs/design/service.md`, one request).

use std::sync::Arc;
use std::time::Duration;

use inbox::actor::{Actor, Caller};
use inbox::behaviour::{InboxStorage, MessageStorage};
use inbox::messaging::Address;
use inbox::primitives::Timestamp;
use inbox::server::json::{self, Value};
use inbox::server::{http, inbox_service};
use inbox_core::{CallerAttributes, CommandClock, Exclusive, ServiceContext};
use inbox_verify::{Instant, duplicate_member};

use crate::auth::{RingVerifier, authenticate};
use crate::clock::CommandReading;
use crate::config::Config;
use crate::signals::Signals;

/// What a backend of the service is: both generated storage ports and the core's section, shared
/// by clones across the connection threads. [`inbox_core::MemoryBackend`] is one.
pub trait Backend:
    Exclusive + InboxStorage + MessageStorage + Clone + Send + Sync + 'static
{
}

impl<B> Backend for B where
    B: Exclusive + InboxStorage + MessageStorage + Clone + Send + Sync + 'static
{
}

/// Every request over HTTP is a client: no request is the expiry task.
const CLIENT: Caller = Caller {
    actor: Actor::Client,
};

/// The route a post arrives on: the one the message limit holds.
const POST: &str = "/messaging/commands/post";

/// The view a `wait` holds open.
const PENDING: &str = "/messaging/views/pending";

/// What every connection thread of one service shares.
#[derive(Debug)]
pub(crate) struct Shared<B> {
    pub(crate) backend: B,
    pub(crate) config: Config,
    pub(crate) signals: Arc<Signals>,
    /// The epoch of this start, written on every answer (`crate::listener::EPOCH_HEADER`).
    pub(crate) epoch: String,
}

/// A refusal of this service: `refused` and, where there is one, the limit it met.
pub(crate) fn refusal(status: u16, refused: &str, limit: Option<usize>) -> http::Response {
    let mut body = String::from("{");
    json::member(&mut body, "refused");
    json::push_text(&mut body, refused);
    if let Some(limit) = limit {
        json::member(&mut body, "limit");
        body.push_str(&limit.to_string());
    }
    body.push('}');
    http::Response::new(status, http::JSON, body)
}

/// The answer to a request the reader read: the steps of `service.md` in order, the first refusal
/// wins.
pub(crate) fn answer<B: Backend>(shared: &Shared<B>, request: &http::Request) -> http::Response {
    let config = &shared.config;
    let max_body = config.max_body.min(http::MAX_BODY);
    if request.body.len() > max_body {
        return refusal(413, "body too large", Some(max_body));
    }
    let at = config.clock.now();
    let Some(principal) = authenticate(&config.keys, request, at) else {
        return refusal(401, "unauthenticated", None);
    };
    if let Some(member) = duplicate(&request.body) {
        let mut body = String::from("{");
        json::member(&mut body, "refused");
        json::push_text(&mut body, "duplicate member");
        json::member(&mut body, "member");
        json::push_text(&mut body, &member);
        body.push('}');
        return http::Response::new(400, http::JSON, body);
    }
    let inbox = Address(principal.0.clone());
    let caller = CallerAttributes::new(principal, inbox);
    if request.method == "GET" && request.path == PENDING {
        match wait(&request.query, config.max_wait) {
            Ok(Some(wait)) => return waited(shared, request, &caller, at, wait),
            Ok(None) => {}
            Err(refused) => return refused,
        }
    }
    once(shared, request, &caller, at)
}

/// The dotted path of a member name the body carries twice, when the body is JSON. A body that is
/// not JSON is the generated reader's to refuse.
fn duplicate(body: &[u8]) -> Option<String> {
    if body.is_empty() {
        return None;
    }
    let value = json::parse(std::str::from_utf8(body).ok()?).ok()?;
    duplicate_member(&value)
}

/// The `wait` of a query, cut to `most`; `None` when the query names none.
fn wait(query: &str, most: Duration) -> Result<Option<Duration>, http::Response> {
    let value = http::query_object(query, &[("wait", http::Scalar::Integer)], "query")
        .map_err(|refused| http::Response::refusal(400, &refused))?;
    match value.member("wait") {
        None => Ok(None),
        Some(Value::Number(seconds)) if !seconds.starts_with('-') => {
            let seconds = seconds.parse::<u64>().unwrap_or(u64::MAX);
            Ok(Some(Duration::from_secs(seconds).min(most)))
        }
        Some(_) => Err(refusal(400, "wait is not a whole number of seconds", None)),
    }
}

/// The request through the generated `dispatch`, inside the backend's section, over a system built
/// for this request's caller; a view answer is then held to the read rule.
fn once<B: Backend>(
    shared: &Shared<B>,
    request: &http::Request,
    caller: &CallerAttributes,
    at: Instant,
) -> http::Response {
    let backend = &shared.backend;
    let config = &shared.config;
    let verifier = RingVerifier::new(Arc::clone(&config.keys), at);
    let context = ServiceContext::new(
        caller.clone(),
        verifier,
        backend.clone(),
        CommandReading(Arc::clone(&config.clock)),
    );
    let answered = backend.exclusive(|| {
        if request.method == "POST"
            && request.path == POST
            && MessageStorage::list(backend).len() >= config.max_messages
        {
            return refusal(503, "message limit", Some(config.max_messages));
        }
        let mut system = inbox_core::system(backend.clone(), context);
        inbox_service::dispatch(&mut system, Some(&CLIENT), request)
    });
    if request.method != "GET" {
        shared.signals.command_answered();
        return readable(as_unknown(answered), caller);
    }
    readable(answered, caller)
}

/// The status the generated writer gives the outcome `unknown-message`.
const UNKNOWN_STATUS: u16 = 404;

/// A command answer held to the core's rename ([`inbox_core::as_unknown`], the one copy of it):
/// an answer it renames is written back as the generated writers write one, with the status of
/// `unknown-message`. Any other answer is handed on as it is.
fn as_unknown(answer: http::Response) -> http::Response {
    if answer.content_type != http::JSON {
        return answer;
    }
    let Ok(value) = json::parse_written(&answer.body) else {
        return answer;
    };
    let renamed = inbox_core::as_unknown(value.clone());
    if renamed == value {
        return answer;
    }
    let mut body = String::new();
    write_value(&mut body, &renamed);
    http::Response::new(UNKNOWN_STATUS, http::JSON, body)
}

/// `Pending` held open: read, and while it has no row for the caller that is not due, wait for the
/// next command answered on another connection, without the section, until `wait` ends or the
/// service stops. A due row (its `expires` at or before the clock reading taken after the view
/// was read, compared as the marks compare it: [`inbox_core::due`]) does not end the wait: its
/// owner can neither read nor mark it, and only the expiry task takes it out of `Pending`. The
/// answer the wait ends with is the whole view, due rows included. A wait with only due rows
/// sleeps on the next command as one with no row does, and reads the view again only after one:
/// a message that becomes due while it sleeps needs no wake. A sweep that sent an `Expire` counts
/// as a command (`crate::listener`), so a wait reads again after the expiry task removed a row.
/// A wait that sleeps to its deadline or to the stop reads the view once more and answers that
/// read, never the one it slept on: what `pending` lists then, with no second clock reading.
fn waited<B: Backend>(
    shared: &Shared<B>,
    request: &http::Request,
    caller: &CallerAttributes,
    at: Instant,
    wait: Duration,
) -> http::Response {
    // A wait too long to add to the clock has no deadline: it ends with a row or the stop.
    let deadline = std::time::Instant::now().checked_add(wait);
    loop {
        // Read before the view, never after: a command answered between the view and this read
        // would move the count unseen, and the wait would sleep past the row it brought.
        let seen = shared.signals.generation();
        let answered = once(shared, request, caller, at);
        let now = CommandReading(Arc::clone(&shared.config.clock)).now();
        if answered.status != 200 || has_rows_not_due(&answered, &now) {
            return answered;
        }
        if !shared.signals.wait_for_command(seen, deadline) {
            return once(shared, request, caller, at);
        }
    }
}

/// Whether a view answer has a row whose `expires` is not at or before `now`. A row without a
/// readable `expires` counts as not due.
fn has_rows_not_due(answer: &http::Response, now: &Timestamp) -> bool {
    let Ok(value) = json::parse_written(&answer.body) else {
        return false;
    };
    let Some(Value::Array(rows)) = value.member("rows") else {
        return false;
    };
    rows.iter().any(|row| match row.member("expires") {
        Some(Value::Text(expires)) => {
            inbox_core::due(&Timestamp(expires.clone()), now) != Some(true)
        }
        _ => true,
    })
}

/// A view answer held to the core's read rule ([`inbox_core::readable`], the one copy of it) for
/// this client, and written back as the generated writers write it. Any other answer is handed on
/// as it is.
fn readable(answer: http::Response, caller: &CallerAttributes) -> http::Response {
    if answer.status != 200 || answer.content_type != http::JSON {
        return answer;
    }
    let Ok(value) = json::parse_written(&answer.body) else {
        return answer;
    };
    if !matches!(value.member("rows"), Some(Value::Array(_))) {
        return answer;
    }
    let mut body = String::new();
    write_value(
        &mut body,
        &inbox_core::readable(Some(&CLIENT), caller, value),
    );
    http::Response::new(200, http::JSON, body)
}

/// Writes a value back as the generated writers write one: no whitespace, members in the order
/// they were read, text escaped by `json::push_text`, a number in the spelling it was read in.
fn write_value(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(flag) => json::push_bool(out, *flag),
        Value::Number(spelling) => out.push_str(spelling),
        Value::Text(text) => json::push_text(out, text),
        Value::Array(items) => {
            out.push('[');
            for (position, item) in items.iter().enumerate() {
                if position > 0 {
                    out.push(',');
                }
                write_value(out, item);
            }
            out.push(']');
        }
        Value::Object(members) => {
            out.push('{');
            for (name, item) in members {
                json::member(out, name);
                write_value(out, item);
            }
            out.push('}');
        }
    }
}

#[cfg(test)]
mod tests {
    use inbox::server::json;

    use super::write_value;

    #[test]
    fn a_generated_answer_is_written_back_byte_for_byte() {
        let text =
            r#"{"rows":[{"a":"x\"y\n","b":[1,-2.50,true,null,{}],"c":{"d":[]}}],"e":"\u0001"}"#;
        let mut out = String::new();
        write_value(&mut out, &json::parse_written(text).unwrap());
        assert_eq!(out, text);
    }
}
