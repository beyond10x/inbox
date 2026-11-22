//! The client commands: each signs its requests with the caller's key and talks to the service
//! only.

use std::path::Path;
use std::time::Duration;

use inbox::messaging::{
    Address, Answer, Body, CloseInbox, Collect, DecisionAsk, DecisionOption, DedupKey,
    DelegationId, MarkRead, MessageId, OpenInbox, OptionLetter, PostMessage, Principal, Refuse,
    Signature, ThreadId,
};
use inbox::primitives::Timestamp;
use inbox::server::json::{self, Value};
use inbox::server::wire;
use inbox_server::{Clock, SystemClock, format_instant};
use inbox_verify::{Instant, sign_envelope};

use crate::client::{Epoch, Service, declared_error, query_value, route};
use crate::collected;
use crate::failure::Failure;
use crate::ids::ulid;
use crate::keys::Caller;
use crate::outbox::{Outbox, Unreadable};
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
}

impl Client {
    fn send(&self, method: &str, target: &str, body: &[u8]) -> Result<Value, Failure> {
        self.service
            .send(&self.caller, method, target, body, Duration::ZERO)?
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
        let sent = self.service.send(
            &self.caller,
            "POST",
            route("POST", wire),
            body.as_bytes(),
            Duration::ZERO,
        );
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
        Ok(self
            .service
            .send(&self.caller, "GET", &target, b"", Duration::ZERO)?
            .epoch)
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

/// `inbox open`.
pub fn open(client: &Client, adapter: &str) -> Result<Done, Failure> {
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
    let answer = client.command("open-inbox", &body)?;
    Ok(outcome(&answer, &input.address.0))
}

/// `inbox close`.
pub fn close(client: &Client) -> Result<Done, Failure> {
    let input = CloseInbox {
        address: client.own_inbox(),
    };
    let mut body = String::new();
    wire::encode_command_inbox_messaging_close_inbox(&input, &mut body);
    let answer = client.command("close-inbox", &body)?;
    Ok(outcome(&answer, &input.address.0))
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
    let inbox = inbox.map_or_else(|| client.own_inbox().0, str::to_owned);
    let mut target = format!("{}?inbox={}", route("GET", "pending"), query_value(&inbox));
    if let Some(seconds) = wait {
        target.push_str(&format!("&wait={seconds}"));
    }
    let answer = client
        .service
        .send(
            &client.caller,
            "GET",
            &target,
            b"",
            Duration::from_secs(wait.unwrap_or(0)),
        )?
        .accepted()?;
    Ok(view_done(&answer))
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
/// it back through the generated decoder, keeps it in the outbox, and posts it. A final refusal
/// ([`final_refusal`]) drops it from the outbox again; a transport failure and every other
/// refusal keep it for `repost`. A post that arrived records the epoch of its answer on the kept
/// envelope; the message id is printed also when the outbox file cannot be changed after that,
/// and the local failure is reported beside it.
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
    let (outcome, epoch) = client.command_in_epoch("post", &text);
    match outcome {
        Ok(answer) => {
            let recorded =
                epoch.map(|epoch| client.outbox.record_epoch(&message_id, &epoch, now()));
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
            }
            Err(Failure::Refused {
                name,
                status,
                answer,
            })
        }
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
    let contents = client.outbox.kept(&client.caller.principal, now())?;
    let kept = contents.kept;
    let current = if kept.iter().any(|entry| entry.epoch.is_some()) {
        client.epoch()?
    } else {
        None
    };
    let mut entries = Vec::new();
    let mut lines = Vec::new();
    let mut first_refusal: Option<(String, u16)> = None;
    let mut local: Option<Failure> = None;
    for entry in kept {
        if current.is_some() && entry.epoch == current {
            continue;
        }
        let message_id = entry.envelope.message_id.0.clone();
        let to = entry.envelope.to.0.clone();
        let (outcome, epoch) = client.command_in_epoch("post", &entry.text);
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
                    if final_refusal(&name, &answer)
                        && let Err(cause) = client.outbox.remove(&message_id, now())
                    {
                        local.get_or_insert(cause);
                    }
                    let said = format!("refused: {name} ({status})");
                    first_refusal.get_or_insert((name, status));
                    (false, answer, said)
                }
            }
            Err(other) => return Err(other),
        };
        if let (true, Some(epoch)) = (arrived && holds(&answer), &epoch)
            && let Err(cause) = client.outbox.record_epoch(&message_id, epoch, now())
        {
            local.get_or_insert(cause);
        }
        lines.push(format!("{message_id} to {to}: {said}"));
        entries.push(Value::Object(vec![
            ("message_id".to_owned(), Value::Text(message_id)),
            ("to".to_owned(), Value::Text(to)),
            ("arrived".to_owned(), Value::Bool(arrived)),
            ("answer".to_owned(), answer),
        ]));
    }
    let report = Value::Object(vec![
        ("reposted".to_owned(), Value::Array(entries)),
        (
            "unreadable".to_owned(),
            unreadable_value(&contents.unreadable),
        ),
    ]);
    lines.extend(unreadable_lines(&contents.unreadable));
    if first_refusal.is_some() || !contents.unreadable.is_empty() {
        for line in &lines {
            warn(&format!("inbox repost: {line}"));
        }
    }
    if let Some(cause) = local {
        return Err(Failure::After {
            done: Done {
                document: document(&report),
                text: if lines.is_empty() {
                    "nothing to repost".to_owned()
                } else {
                    lines.join("\n")
                },
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
    if !contents.unreadable.is_empty() {
        let numbers: Vec<String> = contents
            .unreadable
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
        text: if lines.is_empty() {
            "nothing to repost".to_owned()
        } else {
            lines.join("\n")
        },
    })
}
