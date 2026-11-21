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

use crate::client::{Service, declared_error, query_value, route};
use crate::failure::Failure;
use crate::ids::ulid;
use crate::keys::Caller;
use crate::outbox::Outbox;
use crate::output::{self, Done, document, rows, rows_text};

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

/// `inbox collect`: reads the message's row in the caller's `Outbox` first, so the answer or the
/// refusal it takes is shown, then collects it and drops it from the outbox file.
pub fn collect(client: &Client, message_id: &str) -> Result<Done, Failure> {
    let outbox = client.view(
        "outbox",
        &format!("sender={}", query_value(&client.caller.principal)),
    )?;
    let row = rows(&outbox)
        .iter()
        .find(|row| output::text(row, "message_id") == message_id)
        .cloned();
    let mut body = String::new();
    wire::encode_command_inbox_messaging_collect(
        &Collect {
            message_id: MessageId(message_id.to_owned()),
        },
        &mut body,
    );
    let answer = client.command("collect", &body)?;
    client.outbox.remove(message_id, now())?;
    let row = row.unwrap_or(Value::Null);
    let shown = Value::Object(vec![(
        "rows".to_owned(),
        Value::Array(if row == Value::Null {
            Vec::new()
        } else {
            vec![row.clone()]
        }),
    )]);
    Ok(Done {
        document: document(&Value::Object(vec![
            ("message".to_owned(), row),
            ("answer".to_owned(), answer.clone()),
        ])),
        text: format!(
            "{} {message_id}\n{}",
            output::text(&answer, "outcome"),
            rows_text(&shown)
        ),
    })
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

/// `inbox outbox`: the caller's `Outbox` view, every live message it sent.
pub fn outbox(client: &Client) -> Result<Done, Failure> {
    let answer = client.view(
        "outbox",
        &format!("sender={}", query_value(&client.caller.principal)),
    )?;
    Ok(view_done(&answer))
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

/// `inbox post`: writes the envelope (draws `id`, and `thread` when none is given; `created` is
/// the clock reading and `expires` `created` plus 24 hours when none is given), signs it, keeps
/// it in the outbox, and posts it. A declared refusal drops it from the outbox again, except
/// `UnknownInbox`; a transport failure and the service's own refusals keep it for `repost`.
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
    client.outbox.keep(&text, reading)?;
    match client.command("post", &text) {
        Ok(answer) => Ok(Done {
            document: document(&Value::Object(vec![
                ("message_id".to_owned(), Value::Text(message_id.clone())),
                ("thread".to_owned(), Value::Text(envelope.thread.0.clone())),
                ("answer".to_owned(), answer),
            ])),
            text: message_id,
        }),
        Err(Failure::Refused {
            name,
            status,
            answer,
        }) => {
            if declared_error(&answer).is_some() && name != UNKNOWN_INBOX {
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

/// `inbox repost`: posts every envelope the outbox keeps for the caller again, unchanged.
/// `posted` and `Duplicate` count as arrived. A declared refusal other than `UnknownInbox` drops
/// the envelope; the service's own refusals keep it. Exits as a refusal, naming the first, when
/// any envelope did not arrive; a transport failure ends the run.
pub fn repost(client: &Client) -> Result<Done, Failure> {
    let kept = client.outbox.kept(&client.caller.principal, now())?;
    let mut entries = Vec::new();
    let mut lines = Vec::new();
    let mut first_refusal: Option<(String, u16)> = None;
    for entry in kept {
        let message_id = entry.envelope.message_id.0.clone();
        let to = entry.envelope.to.0.clone();
        let (arrived, answer, said) = match client.command("post", &entry.text) {
            Ok(answer) => (true, answer, "arrived".to_owned()),
            Err(Failure::Refused {
                name,
                status,
                answer,
            }) => {
                let declared = declared_error(&answer).is_some();
                if declared && name == DUPLICATE {
                    (true, answer, format!("arrived ({name})"))
                } else {
                    if declared && name != UNKNOWN_INBOX {
                        client.outbox.remove(&message_id, now())?;
                    }
                    let said = format!("refused: {name} ({status})");
                    first_refusal.get_or_insert((name, status));
                    (false, answer, said)
                }
            }
            Err(other) => return Err(other),
        };
        lines.push(format!("{message_id} to {to}: {said}"));
        entries.push(Value::Object(vec![
            ("message_id".to_owned(), Value::Text(message_id)),
            ("to".to_owned(), Value::Text(to)),
            ("arrived".to_owned(), Value::Bool(arrived)),
            ("answer".to_owned(), answer),
        ]));
    }
    let report = Value::Object(vec![("reposted".to_owned(), Value::Array(entries))]);
    if let Some((name, status)) = first_refusal {
        for line in &lines {
            eprintln!("inbox repost: {line}");
        }
        return Err(Failure::Refused {
            name,
            status,
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
