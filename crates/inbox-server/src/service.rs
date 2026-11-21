//! One request, from what the reader read to the answer (`docs/design/service.md`, one request).

use std::sync::Arc;
use std::time::Duration;

use inbox::actor::{Actor, Caller};
use inbox::behaviour::{InboxStorage, MessageStorage};
use inbox::messaging::Address;
use inbox::server::json::{self, Value};
use inbox::server::{http, inbox_service};
use inbox_core::{CallerAttributes, Exclusive, ServiceContext};
use inbox_verify::{Instant, duplicate_member};

use crate::auth::{RingVerifier, authenticate};
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
    let context = ServiceContext::new(caller.clone(), verifier, backend.clone());
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

/// The outcome the model names for a caller that is neither party to the stored message.
const FOREIGN: &str = "foreign-message";

/// The outcome of an id no message carries, and its status.
const UNKNOWN: (&str, u16) = ("unknown-message", 404);

/// The one answer of this service that differs from the generated one: an answer of the outcome
/// `foreign-message` is given the status and the body of `unknown-message`, its `outcome` member
/// renamed and every other member written back as it was. The model decides who is a party; this
/// compares no caller with no message.
fn as_unknown(answer: http::Response) -> http::Response {
    if answer.content_type != http::JSON {
        return answer;
    }
    let Ok(Value::Object(members)) = json::parse_written(&answer.body) else {
        return answer;
    };
    if !members.iter().any(|(name, value)| {
        name == "outcome" && matches!(value, Value::Text(text) if text == FOREIGN)
    }) {
        return answer;
    }
    let mut body = String::from("{");
    for (name, value) in &members {
        json::member(&mut body, name);
        match name.as_str() {
            "outcome" => json::push_text(&mut body, UNKNOWN.0),
            _ => write_value(&mut body, value),
        }
    }
    body.push('}');
    http::Response::new(UNKNOWN.1, http::JSON, body)
}

/// `Pending` held open: read, and while it has no row for the caller, wait for the next command
/// answered on another connection, without the section, until `wait` ends or the service stops.
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
        if answered.status != 200
            || has_rows(&answered)
            || !shared.signals.wait_for_command(seen, deadline)
        {
            return answered;
        }
    }
}

fn has_rows(answer: &http::Response) -> bool {
    json::parse_written(&answer.body)
        .ok()
        .and_then(|value| match value.member("rows") {
            Some(Value::Array(rows)) => Some(!rows.is_empty()),
            _ => None,
        })
        .unwrap_or(false)
}

/// The read rule: of a view answer, the message rows the caller is neither the owner of the inbox
/// in `to` of nor the sender in `from` of are removed. A row that carries neither member
/// (`Inboxes`) stays. Any other answer is handed on as it is.
fn readable(answer: http::Response, caller: &CallerAttributes) -> http::Response {
    if answer.status != 200 || answer.content_type != http::JSON {
        return answer;
    }
    let Ok(Value::Object(members)) = json::parse_written(&answer.body) else {
        return answer;
    };
    if !matches!(answer_rows(&members), Some(Value::Array(_))) {
        return answer;
    }
    let mut body = String::from("{");
    for (name, value) in &members {
        json::member(&mut body, name);
        match (name.as_str(), value) {
            ("rows", Value::Array(rows)) => {
                let kept: Vec<Value> = rows
                    .iter()
                    .filter(|row| visible(row, caller))
                    .cloned()
                    .collect();
                write_value(&mut body, &Value::Array(kept));
            }
            _ => write_value(&mut body, value),
        }
    }
    body.push('}');
    http::Response::new(200, http::JSON, body)
}

fn answer_rows(members: &[(String, Value)]) -> Option<&Value> {
    members
        .iter()
        .find(|(name, _)| name == "rows")
        .map(|(_, value)| value)
}

fn visible(row: &Value, caller: &CallerAttributes) -> bool {
    let (to, from) = (row.member("to"), row.member("from"));
    if to.is_none() && from.is_none() {
        return true;
    }
    let is = |member: Option<&Value>, mine: Option<&str>| matches!((member, mine), (Some(Value::Text(text)), Some(mine)) if text == mine);
    is(to, caller.inbox.as_ref().map(|inbox| inbox.0.as_str()))
        || is(
            from,
            caller
                .principal
                .as_ref()
                .map(|principal| principal.0.as_str()),
        )
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
