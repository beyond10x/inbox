//! The client commands: each signs its requests with the caller's key and talks to the service
//! only.

use std::cell::RefCell;
use std::path::Path;
use std::time::Duration;

use inbox::messaging::{
    Address, Answer, Body, CloseInbox, Collect, DecisionAsk, DecisionOption, DedupKey,
    DelegationId, MarkDelivered, MarkRead, MessageId, OpenInbox, OptionLetter, PostMessage,
    Principal, Refuse, Signature, ThreadId,
};
use inbox::primitives::Timestamp;
use inbox::server::json::{self, Value};
use inbox::server::wire;
use inbox_server::{Clock, SystemClock, format_instant};
use inbox_verify::{Instant, sign_envelope};

use crate::client::{Epoch, Reply, Service, declared_error, query_value, route};
use crate::collected;
use crate::failure::Failure;
use crate::ids::ulid;
use crate::keys::Caller;
use crate::outbox::{Opened, Outbox, Recorded, Unreadable};
use crate::output::{self, Done, document, rows_text, warn};

/// The model's error a repeated `message_id` or `dedup_key` is answered with (the outcomes
/// `duplicate` and `duplicate-key` of `PostMessage`): the message is there, so it arrived.
const DUPLICATE: &str = "inbox.messaging.Duplicate";
/// The model's error for a post to an inbox the service does not hold open: after a restart the
/// recipient opens it again, so the envelope stays in the outbox.
const UNKNOWN_INBOX: &str = "inbox.messaging.UnknownInbox";

/// How long an envelope lives when `--expires` is not given: `created` plus 24 hours.
const DEFAULT_LIFETIME_SECONDS: i64 = 24 * 3600;

/// A client: the service, the caller, and the outbox file.
pub struct Client {
    /// The service.
    pub service: Service,
    /// The caller.
    pub caller: Caller,
    /// The outbox file.
    pub outbox: Outbox,
    /// The epoch the answer to the last request named; `None` before the first answer, after an
    /// answer without one, and after a request that got no answer.
    seen: RefCell<Option<Epoch>>,
    /// The message id of the envelope `inbox post` kept and posted in this command: the
    /// reconnect after it does not post it a second time.
    posted: RefCell<Option<String>>,
}

impl Client {
    /// A client of `service` for `caller`, with its outbox file.
    pub fn new(service: Service, caller: Caller, outbox: Outbox) -> Self {
        Self {
            service,
            caller,
            outbox,
            seen: RefCell::new(None),
            posted: RefCell::new(None),
        }
    }

    /// Sends one request and reads its answer, keeping the epoch it named ([`Client::seen`]).
    /// Every request of the client goes through here.
    fn exchange(
        &self,
        method: &str,
        target: &str,
        body: &[u8],
        wait: Duration,
    ) -> Result<Reply, Failure> {
        let sent = self.service.send(&self.caller, method, target, body, wait);
        *self.seen.borrow_mut() = sent.as_ref().ok().and_then(|reply| reply.epoch.clone());
        sent
    }

    /// The epoch the answer to the last request named.
    fn seen(&self) -> Option<Epoch> {
        self.seen.borrow().clone()
    }

    fn send(&self, method: &str, target: &str, body: &[u8]) -> Result<Value, Failure> {
        self.exchange(method, target, body, Duration::ZERO)?
            .accepted()
    }

    fn command(&self, wire: &str, body: &str) -> Result<Value, Failure> {
        self.send("POST", route("POST", wire), body.as_bytes())
    }

    fn view(&self, wire: &str, query: &str) -> Result<Value, Failure> {
        let target = format!("{}?{query}", route("GET", wire));
        self.send("GET", &target, b"")
    }

    /// [`Client::command`], with the epoch its answer named, whatever the answer was; `None`
    /// beside a transport failure.
    fn command_in_epoch(&self, wire: &str, body: &str) -> (Result<Value, Failure>, Option<Epoch>) {
        let sent = self.exchange("POST", route("POST", wire), body.as_bytes(), Duration::ZERO);
        match sent {
            Ok(reply) => {
                let epoch = reply.epoch.clone();
                (reply.accepted(), epoch)
            }
            Err(failure) => (Err(failure), None),
        }
    }

    /// The epoch of the service's start, asked with one signed request that changes nothing (the
    /// caller's own `Outbox` view) and read from its answer, whatever its status.
    fn epoch(&self) -> Result<Option<Epoch>, Failure> {
        let target = format!(
            "{}?sender={}",
            route("GET", "outbox"),
            query_value(&self.caller.principal)
        );
        Ok(self.exchange("GET", &target, b"", Duration::ZERO)?.epoch)
    }

    /// The caller's own inbox: the address with the text of its principal (decision 5 of
    /// `docs/design/inbox.md`).
    fn own_inbox(&self) -> Address {
        Address(self.caller.principal.clone())
    }
}

fn now() -> Instant {
    SystemClock.now()
}

/// A value of a flag read by the generated decoder of its type, refused as wrong usage.
fn decoded<T>(
    flag: &str,
    value: Value,
    decode: fn(&Value, &str) -> Result<T, json::DecodeError>,
) -> Result<T, Failure> {
    decode(&value, flag).map_err(|error| Failure::Usage(error.to_string()))
}

/// The answer of a command, for a person: its outcome and what it was about.
fn outcome(answer: &Value, about: &str) -> Done {
    Done {
        document: document(answer),
        text: format!("{} {about}", output::text(answer, "outcome")),
    }
}

/// The answer of a view: the service's body as it is, or its rows for a person.
fn view_done(answer: &Value) -> Done {
    Done {
        document: document(answer),
        text: rows_text(answer),
    }
}

/// `inbox open`: opens the caller's own inbox with `adapter`, then records in the file of the
/// opened inbox beside the outbox file the adapter and the epoch of the answer, for a reconnect
/// to open it again with ([`reconnect`]). The outcome is printed also when that file cannot be
/// written, and the local failure is reported beside it.
pub fn open(client: &Client, adapter: &str) -> Result<Done, Failure> {
    opened_with(client, adapter, None).map(|(done, _)| done)
}

/// `inbox open`, or the open of a reconnect from the record `from`: then the record is replaced
/// only while it is still `from` ([`Outbox::record_opened`]). Answers what it did with the record.
fn opened_with(
    client: &Client,
    adapter: &str,
    from: Option<&Opened>,
) -> Result<(Done, Recorded), Failure> {
    let (done, epoch) = sent_open(client, adapter)?;
    match client
        .outbox
        .record_opened(&client.own_inbox().0, adapter, epoch.as_ref(), from)
    {
        Ok(recorded) => Ok((done, recorded)),
        Err(cause) => Err(Failure::After {
            done,
            cause: Box::new(cause),
        }),
    }
}

/// Sends `OpenInbox` for the caller's own inbox with `adapter` and records nothing: the outcome,
/// and the epoch the answer names.
fn sent_open(client: &Client, adapter: &str) -> Result<(Done, Option<Epoch>), Failure> {
    let input = OpenInbox {
        address: client.own_inbox(),
        adapter: decoded(
            "--adapter",
            Value::Text(adapter.to_owned()),
            wire::decode_inbox_messaging_adapter,
        )?,
    };
    let mut body = String::new();
    wire::encode_command_inbox_messaging_open_inbox(&input, &mut body);
    let (answer, epoch) = client.command_in_epoch("open-inbox", &body);
    Ok((outcome(&answer?, &input.address.0), epoch))
}

/// `inbox close`. The record of the inbox is removed from the file of the opened inbox before
/// `CloseInbox` is sent, so a close the service did is never followed by a reconnect that opens
/// the inbox again; a record that cannot be removed fails the command before the request (exit
/// 4, nothing sent). The record is put back when the close did not happen: a refusal other than
/// `UnknownInbox` (which says the service holds no open inbox there, as after a restart) or a
/// transport failure. A record that cannot be put back is named in one line on standard error;
/// the close's own failure decides the exit.
pub fn close(client: &Client) -> Result<Done, Failure> {
    let input = CloseInbox {
        address: client.own_inbox(),
    };
    let taken = client.outbox.take_opened(&input.address.0)?;
    let mut body = String::new();
    wire::encode_command_inbox_messaging_close_inbox(&input, &mut body);
    let failure = match client.command("close-inbox", &body) {
        Ok(answer) => return Ok(outcome(&answer, &input.address.0)),
        Err(Failure::Refused {
            name,
            status,
            answer,
        }) if declared_error(&answer).is_some() && name == UNKNOWN_INBOX => {
            return Err(Failure::Refused {
                name,
                status,
                answer,
            });
        }
        Err(failure) => failure,
    };
    if let Some(record) = taken
        && let Err(cause) = client.outbox.put_back_opened(&input.address.0, record)
    {
        warn(&format!(
            "inbox close: the record of the opened inbox could not be put back: {}",
            cause.message()
        ));
    }
    Err(failure)
}

/// `inbox read`.
pub fn read(client: &Client, message_id: &str) -> Result<Done, Failure> {
    let mut body = String::new();
    wire::encode_command_inbox_messaging_mark_read(
        &MarkRead {
            message_id: MessageId(message_id.to_owned()),
        },
        &mut body,
    );
    let answer = client.command("read", &body)?;
    Ok(outcome(&answer, message_id))
}

/// `MarkDelivered` of `message_id`, which `inbox wake` sends once the rows are on its standard
/// output: `delivered` moves it from `Sent` to `Delivered`, `already-delivered` changes nothing.
pub fn mark_delivered(client: &Client, message_id: &str) -> Result<Value, Failure> {
    let mut body = String::new();
    wire::encode_command_inbox_messaging_mark_delivered(
        &MarkDelivered {
            message_id: MessageId(message_id.to_owned()),
        },
        &mut body,
    );
    client.command("mark-delivered", &body)
}

/// `inbox refuse`.
pub fn refuse(client: &Client, message_id: &str, reason: &str) -> Result<Done, Failure> {
    let mut body = String::new();
    wire::encode_command_inbox_messaging_refuse(
        &Refuse {
            message_id: MessageId(message_id.to_owned()),
            reason: reason.to_owned(),
        },
        &mut body,
    );
    let answer = client.command("refuse", &body)?;
    Ok(outcome(&answer, message_id))
}

/// `inbox collect`: sends `Collect` and prints the settled record its answer carries (the
/// `response` of the outcome `collected`: the members of the message's `Outbox` row as the
/// service held them when it removed the message, in the form `outbox` prints a row; `--json`:
/// under `message`), then drops the envelope from the outbox file. Nothing is read before the
/// `Collect`, so an answer that arrived just before it is the one printed. A refusal
/// (`not-settled`, `unknown-message`) removes nothing. A success answer that is not `collected`
/// with the settled record of this message ([`collected::record`]: no `response`, as a 0.2.0
/// service answers, one outside the declared response of `Collect`, of another `message_id`, not
/// settled, or with a member stated twice) is a transport failure: nothing is printed as
/// collected and the outbox line stays. The service removed the message
/// once it answered `collected`: what it handed over is printed also when the outbox file cannot
/// be changed after that, and the local failure is reported beside it.
pub fn collect(client: &Client, message_id: &str) -> Result<Done, Failure> {
    let mut body = String::new();
    wire::encode_command_inbox_messaging_collect(
        &Collect {
            message_id: MessageId(message_id.to_owned()),
        },
        &mut body,
    );
    let answer = client.command("collect", &body)?;
    let record = match collected::record(&answer, message_id) {
        Ok(record) => record.clone(),
        Err(why) => {
            return Err(Failure::Transport(format!(
                "the service answered `collected` without the record of {message_id} ({why}): \
                 its reply is not shown, and its outbox line is kept"
            )));
        }
    };
    let shown = Value::Object(vec![(
        "rows".to_owned(),
        Value::Array(vec![record.clone()]),
    )]);
    let done = Done {
        document: document(&Value::Object(vec![
            ("message".to_owned(), record),
            ("answer".to_owned(), answer.clone()),
        ])),
        text: format!(
            "{} {message_id}\n{}",
            output::text(&answer, "outcome"),
            rows_text(&shown)
        ),
    };
    match client.outbox.remove(message_id, now()) {
        Ok(()) => Ok(done),
        Err(cause) => Err(Failure::After {
            done,
            cause: Box::new(cause),
        }),
    }
}

/// `inbox pending`, and `inbox wait` with `wait` seconds: the `Pending` view of `inbox`
/// (the caller's own by default), held open by the service until it has a row or the wait ends.
pub fn pending(client: &Client, inbox: Option<&str>, wait: Option<u64>) -> Result<Done, Failure> {
    Ok(view_done(&pending_view(client, inbox, wait, false)?))
}

/// The read of `inbox wake`: the caller's own `Pending` with `undelivered=1` and `wait` seconds,
/// which only a row in `Sent` ends (`docs/design/service.md`, the wait); the view as answered.
pub fn undelivered(client: &Client, wait: u64) -> Result<Value, Failure> {
    pending_view(client, None, Some(wait), true)
}

/// What `inbox pending` prints of a view answer: the document as it is, or its rows for a person.
pub fn pending_done(answer: &Value) -> Done {
    view_done(answer)
}

fn pending_view(
    client: &Client,
    inbox: Option<&str>,
    wait: Option<u64>,
    undelivered: bool,
) -> Result<Value, Failure> {
    let inbox = inbox.map_or_else(|| client.own_inbox().0, str::to_owned);
    let mut target = format!("{}?inbox={}", route("GET", "pending"), query_value(&inbox));
    if let Some(seconds) = wait {
        target.push_str(&format!("&wait={seconds}"));
    }
    if undelivered {
        target.push_str("&undelivered=1");
    }
    client
        .exchange("GET", &target, b"", Duration::from_secs(wait.unwrap_or(0)))?
        .accepted()
}

/// `inbox outbox`: the caller's `Outbox` view, every live message it sent, and, as `unreadable`,
/// every line of the outbox file that does not read as an envelope.
pub fn outbox(client: &Client) -> Result<Done, Failure> {
    let answer = client.view(
        "outbox",
        &format!("sender={}", query_value(&client.caller.principal)),
    )?;
    let unreadable = client
        .outbox
        .kept(&client.caller.principal, now())?
        .unreadable;
    let mut done = view_done(&answer);
    if let Value::Object(mut members) = answer {
        members.push(("unreadable".to_owned(), unreadable_value(&unreadable)));
        done.document = document(&Value::Object(members));
    }
    for line in unreadable_lines(&unreadable) {
        done.text.push('\n');
        done.text.push_str(&line);
    }
    Ok(done)
}

/// The unreadable lines of the outbox file, as JSON.
fn unreadable_value(unreadable: &[Unreadable]) -> Value {
    Value::Array(
        unreadable
            .iter()
            .map(|entry| {
                Value::Object(vec![
                    ("line".to_owned(), Value::Number(entry.line.to_string())),
                    ("reason".to_owned(), Value::Text(entry.reason.clone())),
                ])
            })
            .collect(),
    )
}

/// The unreadable lines of the outbox file, for a person.
fn unreadable_lines(unreadable: &[Unreadable]) -> Vec<String> {
    unreadable
        .iter()
        .map(|entry| {
            format!(
                "unreadable outbox line {}: {} (never reposted; remove it by hand)",
                entry.line, entry.reason
            )
        })
        .collect()
}

/// `inbox thread` and `inbox replies`: the view `wire` of one thread.
pub fn thread(client: &Client, wire: &str, thread: &str) -> Result<Done, Failure> {
    let answer = client.view(wire, &format!("thread={}", query_value(thread)))?;
    Ok(view_done(&answer))
}

/// The body of an envelope: `--body`, or `--body-file` (`-` for standard input).
///
/// # Errors
///
/// [`Failure::Local`] when the file or standard input cannot be read.
pub fn body(text: Option<&str>, file: Option<&Path>) -> Result<String, Failure> {
    match (text, file) {
        (Some(text), _) => Ok(text.to_owned()),
        (None, Some(path)) if path == Path::new("-") => std::io::read_to_string(std::io::stdin())
            .map_err(|error| Failure::Local(format!("standard input: {error}"))),
        (None, Some(path)) => std::fs::read_to_string(path)
            .map_err(|error| Failure::Local(format!("{}: {error}", path.display()))),
        (None, None) => Err(Failure::Usage(
            "the body: pass --body <text> or --body-file <path> (`-` for standard input)"
                .to_owned(),
        )),
    }
}

/// `expires`: the text given, held to RFC 3339, or `created` plus 24 hours.
fn expires(given: Option<&str>, created: Instant) -> Result<String, Failure> {
    match given {
        Some(text) => {
            Instant::parse(text)
                .map_err(|error| Failure::Usage(format!("--expires `{text}`: {error}")))?;
            Ok(text.to_owned())
        }
        None => Ok(format_instant(
            created.plus_seconds(DEFAULT_LIFETIME_SECONDS),
        )),
    }
}

/// What `inbox post` writes into the envelope beside what the client draws.
#[derive(Debug, Default)]
pub struct PostInput {
    /// `--to`.
    pub to: String,
    /// `--kind`: note, request or decision.
    pub kind: String,
    /// `--thread`; the message id when absent.
    pub thread: Option<String>,
    /// `--dedup-key`; the message id when absent.
    pub dedup_key: Option<String>,
    /// `--on-behalf-of`; the caller when absent.
    pub on_behalf_of: Option<String>,
    /// `--authority`.
    pub authority: Option<String>,
    /// `--reply-shape`: `free`, `lines=<n>` or `schema=<reference>`.
    pub reply_shape: Option<String>,
    /// `--option <letter>=<text>`, repeated.
    pub options: Vec<(String, String)>,
    /// `--recommended-option`.
    pub recommended_option: Option<String>,
    /// `--default-on-silence`.
    pub default_on_silence: Option<String>,
    /// `--request-ref`.
    pub request_ref: Option<String>,
    /// `--expires`; `created` plus 24 hours when absent.
    pub expires: Option<String>,
    /// The body.
    pub body: String,
}

/// A reply shape from `<shape>` or `<shape>=<value>`, read by the generated decoder: a value of
/// digits is tried as a number first, then as text.
fn reply_shape(text: &str) -> Result<inbox::messaging::ReplyShape, Failure> {
    let (shape, value) = match text.split_once('=') {
        Some((shape, value)) => (shape, Some(value)),
        None => (text, None),
    };
    let tagged = |value: Value| {
        Value::Object(vec![
            ("shape".to_owned(), Value::Text(shape.to_owned())),
            ("value".to_owned(), value),
        ])
    };
    let candidates = match value {
        None => vec![tagged(Value::Object(Vec::new()))],
        Some(value) if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) => {
            vec![
                tagged(Value::Number(value.to_owned())),
                tagged(Value::Text(value.to_owned())),
            ]
        }
        Some(value) => vec![tagged(Value::Text(value.to_owned()))],
    };
    let mut refused = None;
    for candidate in candidates {
        match wire::decode_inbox_messaging_reply_shape(&candidate, "--reply-shape") {
            Ok(shape) => return Ok(shape),
            Err(error) => refused = refused.or(Some(error)),
        }
    }
    Err(Failure::Usage(refused.map_or_else(
        || "--reply-shape".to_owned(),
        |error| error.to_string(),
    )))
}

/// Whether a refusal of a post is final for the envelope, so the outbox file drops it: only a
/// declared refusal of the model other than `UnknownInbox` (a restarted service holds no inbox
/// until its owner opens it again). Every refusal of the service's own (`400`, `401`, `408`,
/// `413`, `503`) keeps it: the generated decoder admitted the text before it was kept, so such an
/// answer comes from this run's settings (`--max-request-bytes`) or the transport (bytes that
/// stalled), not from the envelope.
fn final_refusal(name: &str, answer: &Value) -> bool {
    declared_error(answer).is_some() && name != UNKNOWN_INBOX
}

/// Whether the answer to a post says the service holds the message, so the envelope records
/// the epoch of that answer: an accepted post, or the refusal whose outcome is `duplicate` (the
/// message id is there). `duplicate-key` says another message holds the key; the service stored
/// nothing of this one.
fn holds(answer: &Value) -> bool {
    declared_error(answer).is_none() || output::text(answer, "outcome") == "duplicate"
}

/// Whether a post did not reach a service that holds the recipient's inbox, so the same request
/// sent again may be taken: it failed in transport, the service refused it `UnknownInbox` (after
/// a restart the owner has not opened it again yet), or the listener answered of its own that it
/// did not take the request (`408`, or any `5xx` such as `503` while it stops). Such an envelope
/// keeps its mark unsent. The mark goes for an answer that refuses the same request again: `400`,
/// `401`, `413`, a refusal of the model.
fn unsent(outcome: &Result<Value, Failure>) -> bool {
    match outcome {
        Err(Failure::Transport(_) | Failure::Certificate(_) | Failure::Unexpected { .. }) => true,
        Err(Failure::Refused { name, answer, .. }) if declared_error(answer).is_some() => {
            name == UNKNOWN_INBOX
        }
        Err(Failure::Refused { status, .. }) => *status == 408 || *status >= 500,
        _ => false,
    }
}

/// Whether a refusal of a post is `Duplicate` with the outcome `duplicate`: the service holds a
/// message of the sender under this envelope's own message id, so the envelope arrived.
fn own_duplicate(name: &str, answer: &Value) -> bool {
    name == DUPLICATE && declared_error(answer).is_some() && holds(answer)
}

/// A change of the mark of an envelope after the answer to its post: a file that cannot be
/// written is named in one line on standard error, after `what`, and ends nothing.
fn settle_or_warn(changed: Result<(), Failure>, what: &str) {
    if let Err(cause) = changed {
        warn(&format!("{what}: {}", cause.message()));
    }
}

/// The encoded envelope read back by the generated decoder, as the service reads it: what the
/// decoder refuses (a body over the model's limit, an id of the wrong shape) is wrong usage, and
/// never kept or sent. The decoder holds the model's rules; the client writes none of them.
fn admitted<T>(
    text: &str,
    decode: fn(&Value, &str) -> Result<T, json::DecodeError>,
) -> Result<(), Failure> {
    let value = json::parse(text).map_err(|error| Failure::Usage(error.to_string()))?;
    decode(&value, "body")
        .map(|_| ())
        .map_err(|error| Failure::Usage(error.to_string()))
}

/// `inbox post`: writes the envelope (draws `id`, and `thread` when none is given; `created` is
/// the clock reading and `expires` `created` plus 24 hours when none is given), signs it, reads
/// it back through the generated decoder, keeps it in the outbox marked unsent
/// ([`Outbox::keep`]), and posts it. A final refusal ([`final_refusal`]) drops it from the outbox
/// again; a transport failure and every other refusal keep it for `repost`. A post that arrived
/// (also when answered `duplicate` for its own message id: the reconnect of another command of
/// the sender posted it first) records the epoch of its answer, which ends the mark; a transport
/// failure and every answer that says the request was not taken ([`unsent`]) leave the mark, for
/// the reconnect of a later command to post it again; a refusal of this request for good (`413`
/// and the like) removes it. The message id is
/// printed also when the outbox file cannot be changed after that, and the local failure is
/// reported beside it.
pub fn post(client: &Client, input: &PostInput) -> Result<Done, Failure> {
    let reading = now();
    let message_id = ulid()?;
    let created = format_instant(reading);
    let caller = &client.caller;
    let decision = (!input.options.is_empty()
        || input.recommended_option.is_some()
        || input.request_ref.is_some())
    .then(|| DecisionAsk {
        options: input
            .options
            .iter()
            .map(|(letter, text)| DecisionOption {
                letter: OptionLetter(letter.clone()),
                text: text.clone(),
            })
            .collect(),
        recommended_option: input.recommended_option.clone().map(OptionLetter),
        request_ref: input.request_ref.clone(),
    });
    let mut envelope = PostMessage {
        message_id: MessageId(message_id.clone()),
        from: Principal(caller.principal.clone()),
        on_behalf_of: Principal(
            input
                .on_behalf_of
                .clone()
                .unwrap_or_else(|| caller.principal.clone()),
        ),
        authority: input.authority.clone().map(DelegationId),
        to: Address(input.to.clone()),
        kind: decoded(
            "--kind",
            Value::Text(input.kind.clone()),
            wire::decode_inbox_messaging_kind,
        )?,
        thread: ThreadId(input.thread.clone().unwrap_or_else(|| message_id.clone())),
        reply_shape: input.reply_shape.as_deref().map(reply_shape).transpose()?,
        decision,
        default_on_silence: input.default_on_silence.clone().map(OptionLetter),
        created: Timestamp(created.clone()),
        expires: Timestamp(expires(input.expires.as_deref(), reading)?),
        dedup_key: DedupKey(
            input
                .dedup_key
                .clone()
                .unwrap_or_else(|| message_id.clone()),
        ),
        body: Body(input.body.clone()),
        signature: Signature {
            key_id: String::new(),
            value: String::new(),
        },
    };
    sign_envelope(&mut envelope, &caller.key_id, &caller.key)
        .map_err(|error| Failure::Usage(error.to_string()))?;
    let mut text = String::new();
    wire::encode_command_inbox_messaging_post_message(&envelope, &mut text);
    admitted(&text, wire::decode_command_inbox_messaging_post_message)?;
    client.outbox.keep(&text, reading)?;
    *client.posted.borrow_mut() = Some(message_id.clone());
    let (outcome, epoch) = client.command_in_epoch("post", &text);
    // `duplicate` names this envelope's own message id, drawn fresh above: the reconnect of
    // another command of the sender posted the kept envelope while this request was on its way.
    // It arrived.
    let outcome = match outcome {
        Err(Failure::Refused { name, answer, .. }) if own_duplicate(&name, &answer) => Ok(answer),
        other => other,
    };
    let not_reached = unsent(&outcome);
    match outcome {
        Ok(answer) => {
            let recorded = Some(match &epoch {
                Some(epoch) => client.outbox.record_epoch(&message_id, epoch, now()),
                None => client.outbox.clear_unsent(&message_id, now()),
            });
            let done = Done {
                document: document(&Value::Object(vec![
                    ("message_id".to_owned(), Value::Text(message_id.clone())),
                    ("thread".to_owned(), Value::Text(envelope.thread.0.clone())),
                    ("answer".to_owned(), answer),
                ])),
                text: message_id,
            };
            match recorded {
                Some(Err(cause)) => Err(Failure::After {
                    done,
                    cause: Box::new(cause),
                }),
                _ => Ok(done),
            }
        }
        Err(Failure::Refused {
            name,
            status,
            answer,
        }) => {
            if final_refusal(&name, &answer) {
                client.outbox.remove(&message_id, now())?;
            } else if !not_reached {
                // A refusal that the same request meets again (`400`, `401`, `413`): kept, but
                // no reconnect posts it again. `408`, `5xx` and `UnknownInbox` keep the mark.
                settle_or_warn(client.outbox.clear_unsent(&message_id, now()), "inbox post");
            }
            Err(Failure::Refused {
                name,
                status,
                answer,
            })
        }
        // A transport failure keeps the mark: the post is due for a later command.
        Err(other) => Err(other),
    }
}

/// What `inbox answer` writes into the reply envelope.
#[derive(Debug, Default)]
pub struct AnswerInput {
    /// The message answered.
    pub in_reply_to: String,
    /// `--option`.
    pub option: Option<String>,
    /// `--on-behalf-of`; the caller when absent.
    pub on_behalf_of: Option<String>,
    /// `--authority`.
    pub authority: Option<String>,
    /// `--expires`; `created` plus 24 hours when absent.
    pub expires: Option<String>,
    /// The body.
    pub body: String,
}

/// `inbox answer`: the reply envelope (the model's kind reply), signed and sent as `Answer`.
pub fn answer(client: &Client, input: &AnswerInput) -> Result<Done, Failure> {
    let reading = now();
    let reply_id = ulid()?;
    let caller = &client.caller;
    let mut envelope = Answer {
        in_reply_to: MessageId(input.in_reply_to.clone()),
        reply_id: MessageId(reply_id.clone()),
        from: Principal(caller.principal.clone()),
        on_behalf_of: Principal(
            input
                .on_behalf_of
                .clone()
                .unwrap_or_else(|| caller.principal.clone()),
        ),
        authority: input.authority.clone().map(DelegationId),
        created: Timestamp(format_instant(reading)),
        expires: Timestamp(expires(input.expires.as_deref(), reading)?),
        option: input.option.clone().map(OptionLetter),
        body: Body(input.body.clone()),
        signature: Signature {
            key_id: String::new(),
            value: String::new(),
        },
    };
    sign_envelope(&mut envelope, &caller.key_id, &caller.key)
        .map_err(|error| Failure::Usage(error.to_string()))?;
    let mut text = String::new();
    wire::encode_command_inbox_messaging_answer(&envelope, &mut text);
    admitted(&text, wire::decode_command_inbox_messaging_answer)?;
    let answer = client.command("answer", &text)?;
    Ok(Done {
        document: document(&Value::Object(vec![
            ("reply_id".to_owned(), Value::Text(reply_id.clone())),
            ("answer".to_owned(), answer.clone()),
        ])),
        text: format!(
            "{} {} with reply {reply_id}",
            output::text(&answer, "outcome"),
            input.in_reply_to
        ),
    })
}

/// `inbox repost`: posts again, unchanged, every envelope the outbox keeps for the caller that
/// was not said to arrive at the service's current start; for after a restart of the service.
/// When the epochs file beside the outbox file records, for a kept envelope, the epoch of the
/// start its post arrived at, the service is asked for its epoch first ([`Client::epoch`]); an
/// envelope recorded at that epoch is left alone and not listed. Every other envelope is
/// posted: one recorded at another epoch, one without an epoch, and every one when the service
/// names no epoch (an older service). `posted` and `Duplicate` count as arrived; the epoch of
/// the answer is recorded for an accepted post and for the outcome `duplicate` (the service
/// holds the message), not for `duplicate-key` (it stored nothing), right after that answer, so
/// a run ended between two posts keeps the epochs of the posts that arrived. A final refusal
/// ([`final_refusal`]) drops the envelope; the other refusals keep it. Exits as a refusal,
/// naming the first, when any envelope did not arrive, and with an unreadable entry, naming each
/// line of the outbox file that does not read as an envelope; a transport failure ends the run.
/// A local failure (the outbox file or the epochs file cannot be changed) ends no post:
/// every envelope is posted, the report is printed, and the first local failure decides the
/// exit (4).
///
/// A note its recipient read before a restart is delivered once more by the first `repost`
/// after it: the service keeps no record of a handled note.
pub fn repost(client: &Client) -> Result<Done, Failure> {
    let run = repost_at(client, None, None)?;
    if run.first_refusal.is_some() || !run.unreadable.is_empty() {
        for line in run
            .lines
            .iter()
            .cloned()
            .chain(unreadable_lines(&run.unreadable))
        {
            warn(&format!("inbox repost: {line}"));
        }
    }
    run.finish(client)
}

/// What one run of [`repost_at`] did, before it is reported.
struct Reposted {
    entries: Vec<Value>,
    lines: Vec<String>,
    unreadable: Vec<Unreadable>,
    first_refusal: Option<(String, u16)>,
    local: Option<Failure>,
    /// The envelopes posted.
    posted: usize,
    /// Of them, those that arrived.
    arrived: usize,
}

impl Reposted {
    /// The report, and the failure the run ends with ([`repost`]).
    fn finish(self, client: &Client) -> Result<Done, Failure> {
        let Self {
            entries,
            mut lines,
            unreadable,
            first_refusal,
            local,
            ..
        } = self;
        let report = Value::Object(vec![
            ("reposted".to_owned(), Value::Array(entries)),
            ("unreadable".to_owned(), unreadable_value(&unreadable)),
        ]);
        lines.extend(unreadable_lines(&unreadable));
        let text = if lines.is_empty() {
            "nothing to repost".to_owned()
        } else {
            lines.join("\n")
        };
        if let Some(cause) = local {
            return Err(Failure::After {
                done: Done {
                    document: document(&report),
                    text,
                },
                cause: Box::new(cause),
            });
        }
        if let Some((name, status)) = first_refusal {
            return Err(Failure::Refused {
                name,
                status,
                answer: report,
            });
        }
        if !unreadable.is_empty() {
            let numbers: Vec<String> = unreadable
                .iter()
                .map(|entry| entry.line.to_string())
                .collect();
            return Err(Failure::Unreadable {
                detail: format!(
                    "line {} of {}",
                    numbers.join(", "),
                    client.outbox.path().display()
                ),
                answer: report,
            });
        }
        Ok(Done {
            document: document(&report),
            text,
        })
    }
}

/// The posts of `repost`: at the epoch `known` when the caller knows it (a reconnect, from the
/// answer that started it), else at the epoch the service is asked for when a kept envelope
/// records one. The envelope `skip` (the one the command itself posted) is left alone.
fn repost_at(
    client: &Client,
    known: Option<Epoch>,
    skip: Option<&str>,
) -> Result<Reposted, Failure> {
    let contents = client.outbox.kept(&client.caller.principal, now())?;
    let kept = contents.kept;
    let current = match known {
        Some(epoch) => Some(epoch),
        None if kept.iter().any(|entry| entry.epoch.is_some()) => client.epoch()?,
        None => None,
    };
    let mut entries = Vec::new();
    let mut lines = Vec::new();
    let mut first_refusal: Option<(String, u16)> = None;
    let mut local: Option<Failure> = None;
    let mut arrived_count = 0;
    for entry in kept {
        if (current.is_some() && entry.epoch == current)
            || Some(entry.envelope.message_id.0.as_str()) == skip
        {
            continue;
        }
        let message_id = entry.envelope.message_id.0.clone();
        let to = entry.envelope.to.0.clone();
        let (outcome, epoch) = client.command_in_epoch("post", &entry.text);
        let not_reached = unsent(&outcome);
        if entry.epoch.is_none() && not_reached {
            settle_or_warn(
                client.outbox.mark_unsent(&message_id, now()),
                "inbox repost",
            );
        }
        let (arrived, answer, said) = match outcome {
            Ok(answer) => (true, answer, "arrived".to_owned()),
            Err(Failure::Refused {
                name,
                status,
                answer,
            }) => {
                if declared_error(&answer).is_some() && name == DUPLICATE {
                    (true, answer, format!("arrived ({name})"))
                } else {
                    let changed = if final_refusal(&name, &answer) {
                        client.outbox.remove(&message_id, now())
                    } else if not_reached {
                        Ok(())
                    } else {
                        // A refusal of this request for good (`413` and the like): as after a
                        // typed post, kept and no longer due for a reconnect.
                        client.outbox.clear_unsent(&message_id, now())
                    };
                    if let Err(cause) = changed {
                        local.get_or_insert(cause);
                    }
                    let said = format!("refused: {name} ({status})");
                    first_refusal.get_or_insert((name, status));
                    (false, answer, said)
                }
            }
            Err(other) => return Err(other),
        };
        // An arrival records the answer's epoch where the service holds the message. Otherwise
        // (`duplicate-key`: another message holds the key; or an answer without an epoch) the
        // mark goes and no epoch is written: no reconnect posts the envelope again, and a typed
        // `repost` still does, as once the key is free.
        if arrived {
            let changed = match &epoch {
                Some(epoch) if holds(&answer) => {
                    client.outbox.record_epoch(&message_id, epoch, now())
                }
                _ => client.outbox.clear_unsent(&message_id, now()),
            };
            if let Err(cause) = changed {
                local.get_or_insert(cause);
            }
        }
        arrived_count += usize::from(arrived);
        lines.push(format!("{message_id} to {to}: {said}"));
        entries.push(Value::Object(vec![
            ("message_id".to_owned(), Value::Text(message_id)),
            ("to".to_owned(), Value::Text(to)),
            ("arrived".to_owned(), Value::Bool(arrived)),
            ("answer".to_owned(), answer),
        ]));
    }
    let posted = entries.len();
    Ok(Reposted {
        entries,
        lines,
        unreadable: contents.unreadable,
        first_refusal,
        local,
        posted,
        arrived: arrived_count,
    })
}

/// Which command a reconnect follows: it does not repeat the step the command was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Typed {
    /// `inbox open`: the reconnect opens nothing.
    Open,
    /// `inbox close`: the reconnect opens nothing.
    Close,
    /// `inbox repost`: the reconnect posts nothing.
    Repost,
    /// Every other client command.
    Other,
}

/// What a reconnect did: the line it writes to standard error and the member `reconnected` of
/// the `--json` document.
#[derive(Debug)]
pub struct Reconnected {
    /// The one line for standard error.
    pub line: String,
    /// The JSON document of the member `reconnected`: the new start's `epoch`, and `open` and
    /// `repost` for the steps that ran, each the document the typed command would print.
    pub document: String,
    /// What the open of the reconnect came to, when one ran.
    pub open: Option<Reopen>,
}

/// What the open of a reconnect came to.
#[derive(Debug)]
pub enum Reopen {
    /// The caller's inbox is open at the service: the open succeeded (its own, or the second one
    /// after a close meanwhile), or another command of the owner opened it meanwhile.
    Opened,
    /// An `inbox close` of the owner took the record meanwhile, and no `inbox open` wrote one
    /// again: the reconnect closed the inbox again and did not open it.
    Closed,
    /// The open failed (the first one, or the second after a close meanwhile).
    Failed(Failure),
}

/// The reconnect after a client command (`typed`): when the answer to the command's last request
/// names an epoch ([`Client::seen`]) and the files beside the outbox file say a step is due at it
/// ([`Outbox::due`]), it opens the caller's own inbox again with the adapter recorded for it, and
/// posts the kept envelopes again as `repost` does, at that epoch, without asking the service
/// for it: when an envelope is recorded at another epoch or marked unsent ([`Outbox::due`]),
/// each envelope not recorded at that epoch, once, but the envelope `inbox post` posted in this
/// command. The repost leaves each envelope in the state a typed post would leave it for the same
/// answer, but that `duplicate-key` keeps it, without a mark and without an epoch. When the open
/// reaches the service after a `close` took the record, it sends `CloseInbox` again
/// ([`closed_meanwhile`]); when it finds another record than the one it started from (an `open`
/// of the owner meanwhile), it leaves that record as it is (`opened-meanwhile`). The answers of
/// these requests start no reconnect: this runs once per command.
///
/// A step that fails changes nothing of the command's output and exit status, and leaves the
/// files as a failed typed step leaves them (a refused envelope stays kept at its epoch or with
/// its mark, a record keeps its epoch), so the next command tries again. A transport failure of
/// the open ends the reconnect: the post would meet the same service. `None` when nothing is
/// due.
pub fn reconnect(client: &Client, typed: Typed) -> Option<Reconnected> {
    let current = client.seen()?;
    let posted = client.posted.borrow().clone();
    let due = client
        .outbox
        .due(&client.caller.principal, &current, now(), posted.as_deref());
    let open = due
        .open
        .filter(|_| !matches!(typed, Typed::Open | Typed::Close));
    let repost = due.repost && typed != Typed::Repost;
    if open.is_none() && !repost {
        return None;
    }
    let mut members = vec![("epoch".to_owned(), Value::Text(current.as_str().to_owned()))];
    let mut said = Vec::new();
    let mut failed = false;
    let mut gone = false;
    let mut reopen = None;
    if let Some(from) = open {
        let inbox = client.own_inbox().0;
        let adapter = &from.adapter;
        let step = opened_with(client, adapter, Some(&from));
        let shown = match &step {
            Ok((done, Recorded::Written)) => {
                said.push(format!(
                    "opened {inbox} again with the adapter {adapter} ({})",
                    first_word(&done.text)
                ));
                reopen = Some(Reopen::Opened);
                parsed(&done.document)
            }
            Ok((done, Recorded::Gone)) => {
                let (shown, came) = closed_meanwhile(client, parsed(&done.document), &mut said);
                reopen = Some(came);
                shown
            }
            Ok((done, Recorded::Other)) => {
                reopen = Some(Reopen::Opened);
                said.push(format!(
                    "{inbox} was opened by another command meanwhile: its record is left as it is"
                ));
                Value::Object(vec![
                    (
                        "outcome".to_owned(),
                        Value::Text("opened-meanwhile".to_owned()),
                    ),
                    ("opened".to_owned(), parsed(&done.document)),
                ])
            }
            Err(failure) => {
                failed = true;
                gone = matches!(
                    failure,
                    Failure::Transport(_) | Failure::Certificate(_) | Failure::Unexpected { .. }
                );
                said.push(format!(
                    "the open of {inbox} again failed: {}",
                    failure.message()
                ));
                parsed(&failure.document())
            }
        };
        members.push(("open".to_owned(), shown));
        if let Err(failure) = step {
            reopen = Some(Reopen::Failed(failure));
        }
    }
    if repost && !gone {
        let (shown, line) = match repost_at(client, Some(current.clone()), posted.as_deref()) {
            Ok(run) => {
                let tally = format!(
                    "reposted {} kept envelope{}, {} arrived",
                    run.posted,
                    if run.posted == 1 { "" } else { "s" },
                    run.arrived
                );
                match run.finish(client) {
                    Ok(done) => (done.document, tally),
                    Err(failure) => {
                        failed = true;
                        (
                            failure.document(),
                            format!("{tally}: {}", failure.message()),
                        )
                    }
                }
            }
            Err(failure) => {
                failed = true;
                (
                    failure.document(),
                    format!("the repost failed: {}", failure.message()),
                )
            }
        };
        said.push(line);
        members.push(("repost".to_owned(), parsed(&shown)));
    }
    if failed {
        said.push("the next command tries again".to_owned());
    }
    Some(Reconnected {
        line: format!(
            "inbox: reconnect at the start {} of the service: {}",
            current.as_str(),
            said.join("; ")
        ),
        document: document(&Value::Object(members)),
        open: reopen,
    })
}

/// The open of a reconnect reached the service after an `inbox close` of the owner took the
/// record: the reconnect sends `CloseInbox` for the inbox, so the owner's close holds, then reads
/// the record again. When an `inbox open` of the owner wrote one meanwhile, it sends `OpenInbox`
/// once with that record's adapter and leaves the record as it is. Answers the `open` member of
/// the reconnect: `{"outcome":"closed-meanwhile","opened":<the open's answer>,"closed":<the
/// close's answer, or the document of its failure>}`, with `"reopened"` (the answer of that
/// second open, or the document of its failure) when it was sent; pushes the step's part of the
/// reconnect's line to `said`. A failure of the close is one line on standard error, a failure of
/// the second open a part of the reconnect's line; neither changes the exit status.
///
/// Answers also what the open came to: [`Reopen::Closed`] when no record was written again, else
/// what that second open came to.
fn closed_meanwhile(client: &Client, opened: Value, said: &mut Vec<String>) -> (Value, Reopen) {
    let input = CloseInbox {
        address: client.own_inbox(),
    };
    let mut body = String::new();
    wire::encode_command_inbox_messaging_close_inbox(&input, &mut body);
    let closed = match client.command("close-inbox", &body) {
        Ok(answer) => answer,
        Err(failure) => {
            warn(&format!(
                "inbox: {} was closed while the reconnect opened it, and closing it again failed: \
                 {}",
                input.address.0,
                failure.message()
            ));
            parsed(&failure.document())
        }
    };
    let mut members = vec![
        (
            "outcome".to_owned(),
            Value::Text("closed-meanwhile".to_owned()),
        ),
        ("opened".to_owned(), opened),
        ("closed".to_owned(), closed),
    ];
    let inbox = &input.address.0;
    let came = match client.outbox.opened(inbox) {
        None => {
            said.push(format!("{inbox} was closed meanwhile: not opened again"));
            Reopen::Closed
        }
        Some(record) => {
            let adapter = &record.adapter;
            let (reopened, came) = match sent_open(client, adapter) {
                Ok((done, _)) => {
                    said.push(format!(
                        "{inbox} was closed meanwhile, then opened by another command: opened it \
                         again with the adapter {adapter} ({})",
                        first_word(&done.text)
                    ));
                    (parsed(&done.document), Reopen::Opened)
                }
                Err(failure) => {
                    said.push(format!(
                        "{inbox} was closed meanwhile, then opened by another command: opening it \
                         again with the adapter {adapter} failed: {}",
                        failure.message()
                    ));
                    (parsed(&failure.document()), Reopen::Failed(failure))
                }
            };
            members.push(("reopened".to_owned(), reopened));
            came
        }
    };
    (Value::Object(members), came)
}

/// The first word of `text`: the outcome a command's text starts with.
fn first_word(text: &str) -> &str {
    text.split_whitespace().next().unwrap_or_default()
}

/// A document this client wrote, read back as a value.
fn parsed(written: &str) -> Value {
    json::parse(written).unwrap_or_else(|_| Value::Text(written.to_owned()))
}
