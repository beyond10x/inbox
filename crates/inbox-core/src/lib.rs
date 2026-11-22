//! The core of the inbox service: what the synthesized crate `inbox` leaves open, supplied once.
//!
//! `generated/rust/inbox` implements 6 of the 9 command behaviours and all 9 view queries over
//! storage and context ports it does not implement, and owes 3 obligations (`PLAN.md`). This crate
//! supplies the rest and restates none of the specification's rules:
//!
//! - [`Core`]: the behaviours of `PostMessage`, `Collect` and `Expire` (the obligations), over a
//!   backend `B` and a context `C`. Handed to [`inbox::behaviour::Generated`], it completes the
//!   system.
//! - [`ServiceContext`]: the generated context port ([`inbox::behaviour::TryContext`]) and
//!   [`PostExternals`], built from a caller, a [`SignatureVerifier`], the backend and the service's
//!   [`CommandClock`]. It answers the 9 `external:` branches: 3 of `PostMessage`, 5 of `Answer`, 1
//!   of `Refuse`. The clock is part of the context the embedding passes per request, so
//!   [`handle`] applies the deadline of a message as the HTTP service does.
//! - [`MemoryBackend`]: both generated storage ports, in memory. Its clones share one store, and it
//!   is `Send` and `Sync`.
//!
//! [`handle`] runs one command or view for one caller, one command at a time per shared store
//! ([`Exclusive`]), and holds every view answer to the read rule ([`readable`]): a client reads
//! only the rows of the messages it is a party of. It is the entry for an embedding with another
//! caller per request. ESS 0.97.3 refuses a view filter over the caller (`ESS-VIEW-003`), so the
//! rule is here and not in the specification. It answers a client that is no party of a message
//! the answer of an id no message carries ([`as_unknown`]): the model's outcome `foreign-message`
//! would tell it that the id exists. A client whose context answers its inbox or its principal as
//! none gets that answer too wherever the model cannot decide its command on a stored message
//! without the attribute ([`handle`] states the rule).
//!
//! [`handle_as_specified`] is [`handle_with_events`] without these two answers: the conformance
//! runner's entry, which judges the model's own outcomes.
//!
//! [`system`] assembles the generated [`inbox::system::System`]; it serializes nothing and applies
//! no read rule: the generated `handle` and `dispatch` over it answer every row a view selects by
//! its parameters, to any client. Use it directly only where no answer reaches another principal
//! (a test, a tool of the operator), or apply [`readable`] to each view answer and [`as_unknown`]
//! to each command answer, as the HTTP service does over the generated `dispatch`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod backend;
mod context;
mod obligations;

pub use backend::{Exclusive, MemoryBackend};
pub use context::{
    CallerAttributes, CommandClock, Envelope, PostExternals, ServiceContext, SignatureVerifier,
};
pub use obligations::Core;

use inbox::actor::{Actor, Caller};
use inbox::behaviour::{Generated, InboxStorage, MessageStorage, TryContext};
use inbox::ports::inbox_service::InboxService;
use inbox::server::entry::Refused;
use inbox::server::inbox_service;
use inbox::server::json::Value;
use inbox::system::{System, SystemEvent};

/// The generated system over the behaviours of [`Core`]: what the generated `handle` and
/// `dispatch` run.
pub type Service<B, C> = System<Generated<Core<B, C>>>;

/// Assembles the generated system: the 6 generated behaviours and 9 view queries, and the 3
/// obligations of [`Core`], over `backend` and `context`.
///
/// The generated `InboxService` gives no access to its behaviours after construction, so the
/// context is fixed for the life of the system.
///
/// `system` alone serializes nothing: two systems over clones of one backend, run at once, each
/// read before the other writes, and both decide on the same rows. Nor does it apply the read
/// rule or the rename: a view through it answers every row its parameters select, a command the
/// model's `foreign-message`. An embedding with another caller per request goes through
/// [`handle`], which runs each request inside the backend's [`Exclusive`] section, holds each
/// view answer to [`readable`] and each command answer to [`as_unknown`].
pub fn system<B, C>(backend: B, context: C) -> Service<B, C> {
    System::new(InboxService::new(Generated::new(Core::new(
        backend, context,
    ))))
}

/// Runs one command or view by its qualified name for one request, as the generated
/// [`inbox_service::handle`] does, inside the backend's [`Exclusive`] section, and holds the
/// answer to the read rule ([`readable`]) for `caller` and the attributes `context` answers
/// (`try_caller_inbox`, `try_caller_principal`; an attribute it cannot answer is none). A command
/// answer of the outcome `foreign-message` is answered as `unknown-message` ([`as_unknown`]).
///
/// A caller whose context answers the inbox, the principal or both as none (or cannot answer it)
/// gets from `MarkDelivered`, `MarkRead`, `Answer`, `Refuse` and `Collect` the answer of an id no
/// message carries, with no event, wherever the model refuses the command as an unmet obligation
/// because it cannot decide a branch without that attribute: for a message between others in
/// every state, and also for a message the caller is a party of by its one attribute where a
/// branch needs the other. The core cannot tell such a caller from a third principal, so it
/// answers it as one, and an id is answered alike whether a message carries it or not. Where
/// the model decides without the attribute that is none (the sender, known by its principal and
/// with no inbox, collects its settled message) the answer is the model's; a context whose read
/// of an attribute errs gets the answer of no message for every stored message. Every other
/// command and every view is answered as before. The HTTP service hands in both attributes for
/// every client.
///
/// The section is taken before the system is assembled over a clone of `backend` and `context`,
/// and released after the generated `handle` answered, so no other request reads or writes the
/// store in between. `caller` is the actor the request was authenticated as, `context` the
/// request's own (a [`ServiceContext`] over the request's caller, the verifier, a clone of the
/// backend and the service's clock, which a command reads inside the section). A refusal releases the section like an answer does; a panic inside the command
/// unwinds through it and the backend's section does not stay locked ([`Exclusive`] says what
/// the store holds then).
///
/// `context`, its verifier and its `PostExternals` must not call this entry over the same store
/// from inside the command; [`MemoryBackend`] refuses such a call at once with a panic naming the
/// rule, and another backend's [`Exclusive`] refuses it or documents what it does.
///
/// # Errors
///
/// What the generated `handle` refuses ([`Refused`]): an unknown name, an input that is not the
/// declared one, an actor the specification does not grant, an unmet obligation.
pub fn handle<B, C>(
    backend: &B,
    context: C,
    caller: Option<&Caller>,
    name: &str,
    input: Value,
) -> Result<Value, Refused>
where
    B: Exclusive + InboxStorage + MessageStorage + Clone,
    C: TryContext + PostExternals,
{
    handle_with_events(backend, context, caller, name, input).0
}

/// [`handle`], and every event the request published, in log order, as the generated
/// [`inbox_service::handle_with_events`] hands them back.
///
/// The rule of [`handle`] holds here too: `context`, its verifier and its `PostExternals` must
/// not call the entry over the same store from inside the command.
pub fn handle_with_events<B, C>(
    backend: &B,
    context: C,
    caller: Option<&Caller>,
    name: &str,
    input: Value,
) -> (Result<Value, Refused>, Vec<SystemEvent>)
where
    B: Exclusive + InboxStorage + MessageStorage + Clone,
    C: TryContext + PostExternals,
{
    let blind = context.try_caller_principal().ok().flatten().is_none()
        || context.try_caller_inbox().ok().flatten().is_none();
    let addressed = ADDRESS_A_MESSAGE.contains(&name);
    let (answered, events) = handle_as_specified(backend, context, caller, name, input);
    let answered = match answered {
        Err(Refused::Unmet(_)) if blind && addressed => Ok(unknown_message()),
        answered => answered.map(as_unknown),
    };
    (answered, events)
}

/// The 5 commands that address a stored message by its id, each of which the model answers
/// `foreign-message` for a caller that is neither party of it.
const ADDRESS_A_MESSAGE: [&str; 5] = [
    "inbox.messaging.MarkDelivered",
    "inbox.messaging.MarkRead",
    "inbox.messaging.Answer",
    "inbox.messaging.Refuse",
    "inbox.messaging.Collect",
];

/// The answer of [`UNKNOWN_MESSAGE`] as the generated writer gives it for an id no message
/// carries: the members `outcome`, `published` (empty) and `error`, in that order.
fn unknown_message() -> Value {
    Value::Object(vec![
        (
            "outcome".to_owned(),
            Value::Text(UNKNOWN_MESSAGE.to_owned()),
        ),
        ("published".to_owned(), Value::Array(Vec::new())),
        (
            "error".to_owned(),
            Value::Text("inbox.messaging.UnknownMessage".to_owned()),
        ),
    ])
}

/// The conformance runner's entry: [`handle_with_events`] without [`as_unknown`] and without the
/// answer of no message to a caller without an attribute, so a client that is no party of a
/// message gets the model's own outcome `foreign-message`, which tells it that a message with
/// that id exists, and a client without an attribute the model's unmet obligation. The run of
/// the authored scenarios judges the model's order of refusals through it
/// (`crates/inbox-conformance/src/core_target.rs`).
///
/// Use it only where no answer reaches another principal, as [`system`]; an embedding goes
/// through [`handle`] or [`handle_with_events`]. The section, the read rule and the rule of
/// [`handle`] about calls from inside the command hold here as there.
pub fn handle_as_specified<B, C>(
    backend: &B,
    context: C,
    caller: Option<&Caller>,
    name: &str,
    input: Value,
) -> (Result<Value, Refused>, Vec<SystemEvent>)
where
    B: Exclusive + InboxStorage + MessageStorage + Clone,
    C: TryContext + PostExternals,
{
    let attributes = CallerAttributes {
        principal: context.try_caller_principal().ok().flatten(),
        inbox: context.try_caller_inbox().ok().flatten(),
    };
    let (answered, events) = backend.exclusive(|| {
        let mut service = system(backend.clone(), context);
        inbox_service::handle_with_events(&mut service, caller, name, input)
    });
    (
        answered.map(|answer| readable(caller, &attributes, answer)),
        events,
    )
}

/// The outcome the model names for a caller that is neither party of the addressed message.
pub const FOREIGN_MESSAGE: &str = "foreign-message";

/// The outcome of an id no message carries.
pub const UNKNOWN_MESSAGE: &str = "unknown-message";

/// The one rename of a command answer, and its one copy: an answer of the outcome
/// [`FOREIGN_MESSAGE`] becomes the answer of [`UNKNOWN_MESSAGE`], its `outcome` member renamed in
/// place and every other member kept as it was, so a client that is no party of a message cannot
/// tell it from no message. The generated answer of `foreign-message` carries the members of
/// `unknown-message` (`published` empty, `error` `inbox.messaging.UnknownMessage`) in the same
/// order, so the renamed answer is the other one byte for byte. Any other answer is handed back as
/// it is. [`handle`] applies it, and so does the HTTP service. The model decides who is a party;
/// this compares no caller with no message.
pub fn as_unknown(answer: Value) -> Value {
    let Value::Object(members) = answer else {
        return answer;
    };
    Value::Object(
        members
            .into_iter()
            .map(|(name, value)| match value {
                Value::Text(text) if name == "outcome" && text == FOREIGN_MESSAGE => {
                    (name, Value::Text(UNKNOWN_MESSAGE.to_owned()))
                }
                value => (name, value),
            })
            .collect(),
    )
}

/// The read rule: of a view answer to a client, the rows of messages the client is no party of
/// are removed. The one copy of the rule; [`handle`] and [`handle_as_specified`] apply it, and so
/// does the HTTP service.
///
/// A row that carries `to` or `from` (a row of `Pending`, `Unanswered`, `Outbox`, `Settled`,
/// `Thread` or `Replies`) stays only when its `to` is `attributes.inbox` (the caller owns the
/// inbox it was sent to) or its `from` is `attributes.principal` (the caller sent it); a caller
/// without the attribute matches nothing by it. A row that carries neither (`Inboxes`) stays.
/// The rule binds every caller but the `Sweeper`, the service's own expiry task, whose `Due` and
/// `Messages` (which no client is granted) stay whole. An answer without a `rows` array (a
/// command's) is handed back as it is; the members of an answer and of each row keep their order.
pub fn readable(caller: Option<&Caller>, attributes: &CallerAttributes, answer: Value) -> Value {
    if caller.is_some_and(|caller| matches!(caller.actor, Actor::Sweeper)) {
        return answer;
    }
    let Value::Object(members) = answer else {
        return answer;
    };
    Value::Object(
        members
            .into_iter()
            .map(|(name, value)| match value {
                Value::Array(rows) if name == "rows" => (
                    name,
                    Value::Array(
                        rows.into_iter()
                            .filter(|row| party(row, attributes))
                            .collect(),
                    ),
                ),
                value => (name, value),
            })
            .collect(),
    )
}

/// Whether the caller is a party of the message `row` shows, or `row` shows no message.
fn party(row: &Value, attributes: &CallerAttributes) -> bool {
    let (to, from) = (row.member("to"), row.member("from"));
    if to.is_none() && from.is_none() {
        return true;
    }
    let is = |member: Option<&Value>, mine: Option<&str>| matches!((member, mine), (Some(Value::Text(text)), Some(mine)) if text == mine);
    is(to, attributes.inbox.as_ref().map(|inbox| inbox.0.as_str()))
        || is(
            from,
            attributes
                .principal
                .as_ref()
                .map(|principal| principal.0.as_str()),
        )
}

const _: fn() = || {
    fn shared<T: Send + Sync + Clone + 'static>() {}
    shared::<MemoryBackend>();
};
