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

/// The query parameter of a read of `Pending` for `inbox wake` (`undelivered=1`): its answer
/// holds only the caller's rows in `Sent` that are not due, so only such a row ends its wait, and
/// it is answered `409` at once for a caller whose own inbox is not open.
const UNDELIVERED: &str = "undelivered";

/// The refusal of a read with `undelivered=1` while the caller's own inbox is not open.
const NOT_OPEN: &str = "inbox not open";

/// A wait on `Pending` a query asks for.
#[derive(Debug, Clone, Copy)]
struct Wait {
    /// How long, cut to `max_wait`.
    long: Duration,
    /// `undelivered=1`: the answer holds only the rows in `Sent` that are not due.
    undelivered: bool,
}

/// The `wait` of a query, cut to `most`, and its `undelivered`; `None` when the query names
/// neither. `undelivered=1` without a `wait` is a wait of no time. An `undelivered` other than `1`
/// is refused, also without a `wait`.
fn wait(query: &str, most: Duration) -> Result<Option<Wait>, http::Response> {
    let value = http::query_object(
        query,
        &[
            ("wait", http::Scalar::Integer),
            (UNDELIVERED, http::Scalar::Integer),
        ],
        "query",
    )
    .map_err(|refused| http::Response::refusal(400, &refused))?;
    let undelivered = match value.member(UNDELIVERED) {
        None => false,
        Some(Value::Number(one)) if one == "1" => true,
        Some(_) => return Err(refusal(400, "undelivered is not 1", None)),
    };
    match value.member("wait") {
        None if undelivered => Ok(Some(Wait {
            long: Duration::ZERO,
            undelivered,
        })),
        None => Ok(None),
        Some(Value::Number(seconds)) if !seconds.starts_with('-') => {
            let seconds = seconds.parse::<u64>().unwrap_or(u64::MAX);
            Ok(Some(Wait {
                long: Duration::from_secs(seconds).min(most),
                undelivered,
            }))
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
/// With `undelivered=1` (`docs/design/service.md`, the wait) every read, the one at the deadline
/// included, is first held to the caller's own inbox: while it is not open the answer is `409
/// {"refused":"inbox not open"}` at once, so a close, which is a command and wakes the wait, ends
/// it. The answer then holds only the rows in `Sent` that are not due at the clock reading after
/// that read (the other members as they were), so a row in `Delivered` sleeps as a due one does.
fn waited<B: Backend>(
    shared: &Shared<B>,
    request: &http::Request,
    caller: &CallerAttributes,
    at: Instant,
    wait: Wait,
) -> http::Response {
    // A wait too long to add to the clock has no deadline: it ends with a row or the stop.
    let deadline = std::time::Instant::now().checked_add(wait.long);
    loop {
        // Read before the view, never after: a command answered between the view and this read
        // would move the count unseen, and the wait would sleep past the row it brought.
        let seen = shared.signals.generation();
        if wait.undelivered && !own_inbox_open(shared, caller) {
            return refusal(409, NOT_OPEN, None);
        }
        let answered = once(shared, request, caller, at);
        let now = CommandReading(Arc::clone(&shared.config.clock)).now();
        if answered.status != 200 {
            return answered;
        }
        if wait.undelivered {
            if let Some(kept) = undelivered_rows(&answered, &now) {
                return kept;
            }
        } else if has_rows_not_due(&answered, &now) {
            return answered;
        }
        if !shared.signals.wait_for_command(seen, deadline) {
            if !wait.undelivered {
                return once(shared, request, caller, at);
            }
            if !own_inbox_open(shared, caller) {
                return refusal(409, NOT_OPEN, None);
            }
            let answered = once(shared, request, caller, at);
            let now = CommandReading(Arc::clone(&shared.config.clock)).now();
            return match undelivered_rows(&answered, &now) {
                Some(kept) => kept,
                None if answered.status == 200 => without_rows(answered),
                None => answered,
            };
        }
    }
}

/// Whether the caller's own inbox is stored and `Open`.
fn own_inbox_open<B: Backend>(shared: &Shared<B>, caller: &CallerAttributes) -> bool {
    let backend = &shared.backend;
    backend.exclusive(|| {
        caller
            .inbox
            .as_ref()
            .and_then(|inbox| InboxStorage::get(backend, inbox))
            .is_some_and(|inbox| inbox.state == inbox::messaging::InboxState::Open)
    })
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
    rows.iter().any(|row| not_due(row, now))
}

/// Whether a row's `expires` is not at or before `now`; a row without a readable `expires` counts
/// as not due.
fn not_due(row: &Value, now: &Timestamp) -> bool {
    match row.member("expires") {
        Some(Value::Text(expires)) => {
            inbox_core::due(&Timestamp(expires.clone()), now) != Some(true)
        }
        _ => true,
    }
}

/// A view answer of `undelivered=1`: its rows cut to those in `Sent` that are not due at `now`,
/// written back as the generated writers write it; `None` when no such row is left or the answer
/// is not a view.
fn undelivered_rows(answer: &http::Response, now: &Timestamp) -> Option<http::Response> {
    let kept = kept_rows(answer, |row| {
        matches!(row.member("state"), Some(Value::Text(state)) if state == "Sent")
            && not_due(row, now)
    })?;
    let has_rows = matches!(kept.member("rows"), Some(Value::Array(rows)) if !rows.is_empty());
    has_rows.then(|| written(&kept))
}

/// A view answer with no row, the other members as they were: the answer of `undelivered=1` that
/// ends at its bound or at the stop with no row in `Sent` that is not due.
fn without_rows(answer: http::Response) -> http::Response {
    match kept_rows(&answer, |_| false) {
        Some(kept) => written(&kept),
        None => answer,
    }
}

/// The view of `answer` with only the rows `keep` keeps; `None` when it is no view.
fn kept_rows(answer: &http::Response, keep: impl Fn(&Value) -> bool) -> Option<Value> {
    let Value::Object(members) = json::parse_written(&answer.body).ok()? else {
        return None;
    };
    if !matches!(
        members.iter().find(|(name, _)| name == "rows"),
        Some((_, Value::Array(_)))
    ) {
        return None;
    }
    Some(Value::Object(
        members
            .into_iter()
            .map(|(name, value)| match (name.as_str(), value) {
                ("rows", Value::Array(rows)) => {
                    (name, Value::Array(rows.into_iter().filter(&keep).collect()))
                }
                (_, value) => (name, value),
            })
            .collect(),
    ))
}

/// A view answer `200` with the body `value`, written as the generated writers write it.
fn written(value: &Value) -> http::Response {
    let mut body = String::new();
    write_value(&mut body, value);
    http::Response::new(200, http::JSON, body)
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
