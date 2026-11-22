//! The one target: the generated `handle` over the obligations of `inbox-core` and its
//! `MemoryBackend`, in process, through `inbox_core::handle_as_specified`.

use inbox::actor::{Actor, Caller, may_read};
use inbox::behaviour::{ExternalCommand, TryContext};
use inbox::messaging::{Address, PostMessage, Principal};
use inbox::obligation::UnmetObligation;
use inbox::server::entry::Refused;
use inbox::server::json::{self, Value};
use inbox::server::wire;
use inbox::system::SystemEvent;
use inbox_core::{MemoryBackend, PostExternals};

use crate::runner::{Answer, Command, Event, Executed, Read, Reader, Target};
use crate::value::exact_equal;

/// `inbox-core` over a fresh [`MemoryBackend`] per scenario.
///
/// Each step is one call of [`inbox_core::handle_as_specified`] with a [`StepContext`]: the caller
/// is the step's actor and attributes, and each of the 9 `external:` branches is answered as the
/// step's expected outcome says. Every other rule is the generated crate's and the core's. That
/// entry is `handle_with_events` without the rename of `foreign-message` to `unknown-message`
/// ([`inbox_core::as_unknown`]), so the run judges the model's own outcome and its place in the
/// order of refusals; an embedding gets the renamed answer.
///
/// A read of the suite names no caller, and the core answers a client only the rows it is a
/// party of ([`inbox_core::readable`]). So a read as a client is sent as every caller of a
/// client step so far, and the rows are joined, each listed once, as [`crate::WireTarget`]
/// joins the rows of every key: a core that hides a row from both its parties fails the read.
#[derive(Debug, Default)]
pub struct CoreTarget {
    backend: MemoryBackend,
    readers: Vec<StepContext>,
}

impl CoreTarget {
    /// A target over an empty store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Target for CoreTarget {
    fn reset(&mut self) {
        self.backend = MemoryBackend::new();
        self.readers.clear();
    }

    fn execute(&mut self, command: &Command<'_>) -> Executed {
        let broken = |reason: String| Executed {
            answer: Answer::Broken(reason),
            events: Vec::new(),
        };
        let caller = match command.actor.map(actor).transpose() {
            Ok(caller) => caller,
            Err(reason) => return broken(reason),
        };
        let context = match StepContext::new(command.caller, command.expected) {
            Ok(context) => context,
            Err(reason) => return broken(reason),
        };
        if caller.is_some_and(|caller| matches!(caller.actor, Actor::Client)) {
            let reader = context.reader();
            if !self.readers.contains(&reader) {
                self.readers.push(reader);
            }
        }
        let (answered, published) = inbox_core::handle_as_specified(
            &self.backend,
            context,
            caller.as_ref(),
            command.name,
            command.input.clone(),
        );
        let events = match published.iter().map(event).collect::<Result<Vec<_>, _>>() {
            Ok(events) => events,
            Err(reason) => return broken(reason),
        };
        let answer = match answered {
            Ok(value) => outcome(&value),
            Err(Refused::NotGranted(actor)) => Answer::NotGranted(actor),
            Err(other) => Answer::Refused(other.to_string()),
        };
        Executed { answer, events }
    }

    fn query(&mut self, view: &str, reader: &Reader, params: Value) -> Read {
        let caller = match reader {
            Reader::Unstated => Actor::ALL
                .iter()
                .find(|actor| may_read(**actor).contains(&view))
                .map(|actor| Caller { actor: *actor }),
            Reader::As(None) => None,
            Reader::As(Some(name)) => match actor(name) {
                Ok(caller) => Some(caller),
                Err(reason) => return Read::Broken(reason),
            },
        };
        let client = caller.is_some_and(|caller| matches!(caller.actor, Actor::Client));
        let readers = if client && !self.readers.is_empty() {
            self.readers.clone()
        } else {
            vec![StepContext::reading()]
        };
        let mut union: Option<Vec<Value>> = None;
        for reader in readers {
            let read = self.read_as(reader, caller.as_ref(), view, params.clone());
            match (read, &mut union) {
                (Read::Rows(rows), Some(kept)) => {
                    for row in rows {
                        if !kept.iter().any(|held| exact_equal(held, &row)) {
                            kept.push(row);
                        }
                    }
                }
                (Read::Rows(rows), None) => union = Some(rows),
                (other, _) => return other,
            }
        }
        Read::Rows(union.unwrap_or_default())
    }
}

impl CoreTarget {
    /// One read through the entry as one caller.
    fn read_as(
        &self,
        reader: StepContext,
        caller: Option<&Caller>,
        view: &str,
        params: Value,
    ) -> Read {
        match inbox_core::handle(&self.backend, reader, caller, view, params) {
            Ok(value) => match value.member("rows") {
                Some(Value::Array(rows)) => Read::Rows(rows.clone()),
                _ => Read::Broken(format!("`{view}` answered no `rows` array")),
            },
            Err(Refused::NotGranted(actor)) => Read::NotGranted(actor),
            Err(other) => Read::Refused(other.to_string()),
        }
    }
}

/// The caller of a declared actor, by its qualified name.
fn actor(name: &str) -> Result<Caller, String> {
    Actor::ALL
        .iter()
        .find(|actor| actor.name() == name)
        .map(|actor| Caller { actor: *actor })
        .ok_or_else(|| format!("`{name}` is no actor the inbox declares"))
}

/// The declared outcome an answer renders.
fn outcome(value: &Value) -> Answer {
    let Some(Value::Text(outcome)) = value.member("outcome") else {
        return Answer::Broken("the answer names no outcome".to_owned());
    };
    let error = match value.member("error") {
        Some(Value::Text(error)) => Some(error.clone()),
        _ => None,
    };
    Answer::Outcome {
        outcome: outcome.clone(),
        error,
        response: value.member("response").cloned(),
    }
}

/// An event of the system's log, as the wire encodes its payload.
fn event(published: &SystemEvent) -> Result<Event, String> {
    let mut payload = String::new();
    match published {
        SystemEvent::DecisionTakenOnSilence(event) => {
            wire::encode_event_inbox_messaging_decision_taken_on_silence(event, &mut payload);
        }
        SystemEvent::InboxClosed(event) => {
            wire::encode_event_inbox_messaging_inbox_closed(event, &mut payload);
        }
        SystemEvent::InboxOpened(event) => {
            wire::encode_event_inbox_messaging_inbox_opened(event, &mut payload);
        }
        SystemEvent::MessageAnswered(event) => {
            wire::encode_event_inbox_messaging_message_answered(event, &mut payload);
        }
        SystemEvent::MessageDelivered(event) => {
            wire::encode_event_inbox_messaging_message_delivered(event, &mut payload);
        }
        SystemEvent::MessageExpired(event) => {
            wire::encode_event_inbox_messaging_message_expired(event, &mut payload);
        }
        SystemEvent::MessageHandled(event) => {
            wire::encode_event_inbox_messaging_message_handled(event, &mut payload);
        }
        SystemEvent::MessagePosted(event) => {
            wire::encode_event_inbox_messaging_message_posted(event, &mut payload);
        }
        SystemEvent::MessageRead(event) => {
            wire::encode_event_inbox_messaging_message_read(event, &mut payload);
        }
        SystemEvent::MessageRefused(event) => {
            wire::encode_event_inbox_messaging_message_refused(event, &mut payload);
        }
    }
    let payload = json::parse_written(&payload)
        .map_err(|error| format!("the payload of `{}` is not JSON: {error}", published.name()))?;
    Ok(Event {
        name: published.name().to_owned(),
        payload,
    })
}

/// The context of one step: the caller's attributes, and the answer to each `external:` branch.
///
/// A branch is taken exactly when it is the outcome the step expects, so a step that expects
/// `signature-invalid` finds the signature refused and every other branch not taken, and a step
/// that expects `posted` finds none taken. The 5 branches of `Answer` and the 1 each of `Refuse`,
/// `MarkDelivered` and `MarkRead` are asked through [`TryContext::try_external`], the 3 of
/// `PostMessage` through [`PostExternals`]. The 6 that read the service's clock (the 2
/// `expired-on-arrival`, the 4 `deadline-passed`) are answered so too: a scenario states no clock,
/// and its instants are literals.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StepContext {
    principal: Option<Principal>,
    inbox: Option<Address>,
    expected: Option<String>,
}

impl StepContext {
    /// The context of a command step: `caller` holds the attributes by name (`principal`,
    /// `inbox`), `expected` the outcome the step expects.
    ///
    /// # Errors
    ///
    /// An attribute the inbox's actors do not declare.
    pub fn new(caller: &[(String, String)], expected: Option<&str>) -> Result<Self, String> {
        let mut context = Self {
            expected: expected.map(str::to_owned),
            ..Self::default()
        };
        for (name, value) in caller {
            match name.as_str() {
                "principal" => context.principal = Some(Principal(value.clone())),
                "inbox" => context.inbox = Some(Address(value.clone())),
                other => {
                    return Err(format!(
                        "the caller attribute `{other}` is not one the inbox's actors declare"
                    ));
                }
            }
        }
        Ok(context)
    }

    /// The context of a read: no attribute, no branch taken.
    pub fn reading() -> Self {
        Self::default()
    }

    /// The context of a read as this step's caller: its attributes, no branch taken.
    pub fn reader(&self) -> Self {
        Self {
            expected: None,
            ..self.clone()
        }
    }

    /// Whether the branch named `outcome` is taken.
    pub fn takes(&self, outcome: &str) -> bool {
        self.expected.as_deref() == Some(outcome)
    }
}

impl TryContext for StepContext {
    fn try_caller_inbox(&self) -> Result<Option<Address>, UnmetObligation> {
        Ok(self.inbox.clone())
    }

    fn try_caller_principal(&self) -> Result<Option<Principal>, UnmetObligation> {
        Ok(self.principal.clone())
    }

    fn try_external(
        &mut self,
        _command: ExternalCommand<'_>,
        outcome: &'static str,
    ) -> Result<bool, UnmetObligation> {
        Ok(self.takes(outcome))
    }
}

impl PostExternals for StepContext {
    fn expired_on_arrival(&mut self, _input: &PostMessage) -> Result<bool, UnmetObligation> {
        Ok(self.takes("expired-on-arrival"))
    }

    fn signature_invalid(&mut self, _input: &PostMessage) -> bool {
        self.takes("signature-invalid")
    }

    fn authority_invalid(&mut self, _input: &PostMessage) -> bool {
        self.takes("authority-invalid")
    }
}
