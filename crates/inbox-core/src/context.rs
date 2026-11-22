//! The context port: the caller's attributes, the service's clock and the answers to the 9
//! `external:` branches.

use inbox::behaviour::{ExternalCommand, MessageStorage, TryContext, unmet_context};
use inbox::messaging::{
    Address, Answer, DecisionAsk, DelegationId, Kind, MessageData, MessageId, MessageSnapshot,
    MessageState, OptionLetter, PostMessage, Principal, ReplyShape, Signature,
};
use inbox::obligation::UnmetObligation;
use inbox::primitives::Timestamp;
use inbox::primitives::invariant::{self, Fact, Op};
use inbox::server::json;

/// The attributes of the authenticated caller, as the embedding code hands them in.
///
/// A `Client` carries both; the `Sweeper` carries none. [`crate::handle`] answers a `Client`
/// that carries one or none the answer of an id no message carries wherever the model cannot
/// decide `MarkDelivered`, `MarkRead`, `Answer`, `Refuse` or `Collect` on a stored message
/// without the absent attribute, so such a client cannot tell a message between others from no
/// message either.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallerAttributes {
    /// `principal`: who the request was authenticated as.
    pub principal: Option<Principal>,
    /// `inbox`: the address of the inbox that principal owns.
    pub inbox: Option<Address>,
}

impl CallerAttributes {
    /// A caller carrying both attributes.
    pub fn new(principal: Principal, inbox: Address) -> Self {
        Self {
            principal: Some(principal),
            inbox: Some(inbox),
        }
    }

    /// A caller carrying no attribute, such as the expiry task.
    pub fn none() -> Self {
        Self::default()
    }
}

/// A signed envelope a [`SignatureVerifier`] checks: a post or a reply.
#[derive(Debug, Clone, Copy)]
pub enum Envelope<'a> {
    /// The input of `PostMessage`.
    Post(&'a PostMessage),
    /// The input of `Answer`: the reply envelope.
    Answer(&'a Answer),
}

impl<'a> Envelope<'a> {
    /// The key id and the signature.
    pub fn signature(&self) -> &'a Signature {
        match self {
            Self::Post(post) => &post.signature,
            Self::Answer(answer) => &answer.signature,
        }
    }

    /// Who sends.
    pub fn from(&self) -> &'a Principal {
        match self {
            Self::Post(post) => &post.from,
            Self::Answer(answer) => &answer.from,
        }
    }

    /// Whose word the envelope is.
    pub fn on_behalf_of(&self) -> &'a Principal {
        match self {
            Self::Post(post) => &post.on_behalf_of,
            Self::Answer(answer) => &answer.on_behalf_of,
        }
    }

    /// The delegation the envelope names, if any.
    pub fn authority(&self) -> Option<&'a DelegationId> {
        match self {
            Self::Post(post) => post.authority.as_ref(),
            Self::Answer(answer) => answer.authority.as_ref(),
        }
    }

    /// When the sender wrote it.
    pub fn created(&self) -> &'a Timestamp {
        match self {
            Self::Post(post) => &post.created,
            Self::Answer(answer) => &answer.created,
        }
    }
}

/// The SignatureVerifier port of the specification: the 2 `signature-invalid` and the 2
/// `authority-invalid` branches are asked of it. Object-safe.
pub trait SignatureVerifier {
    /// `true` when the key the envelope's signature names is known and live, and the signature
    /// verifies over the envelope; `false` takes `signature-invalid`.
    fn verifies(&self, envelope: Envelope<'_>) -> bool;

    /// `true` when `authority` is a live delegation that lets the envelope's `from` speak for its
    /// `on_behalf_of`; `false` takes `authority-invalid`. Asked only of an envelope that names a
    /// delegation.
    fn authorizes(&self, envelope: Envelope<'_>, authority: &DelegationId) -> bool;
}

impl<V: SignatureVerifier + ?Sized> SignatureVerifier for &V {
    fn verifies(&self, envelope: Envelope<'_>) -> bool {
        (**self).verifies(envelope)
    }

    fn authorizes(&self, envelope: Envelope<'_>, authority: &DelegationId) -> bool {
        (**self).authorizes(envelope, authority)
    }
}

impl<V: SignatureVerifier + ?Sized> SignatureVerifier for Box<V> {
    fn verifies(&self, envelope: Envelope<'_>) -> bool {
        (**self).verifies(envelope)
    }

    fn authorizes(&self, envelope: Envelope<'_>, authority: &DelegationId) -> bool {
        (**self).authorizes(envelope, authority)
    }
}

impl<V: SignatureVerifier + ?Sized> SignatureVerifier for std::sync::Arc<V> {
    fn verifies(&self, envelope: Envelope<'_>) -> bool {
        (**self).verifies(envelope)
    }

    fn authorizes(&self, envelope: Envelope<'_>, authority: &DelegationId) -> bool {
        (**self).authorizes(envelope, authority)
    }
}

/// The service's clock, as a command reads it: the instant the transition is decided at.
///
/// [`ServiceContext`] reads it once per question, when the command asks, which is inside the
/// backend's [`crate::Exclusive`] section where [`crate::handle`] runs the command. The reading is
/// the service's own: no member a client sends takes its place. A fixed [`Timestamp`] is a clock
/// that always reads that instant (for a test, or an embedding that decided the instant itself).
/// Object-safe.
pub trait CommandClock {
    /// The current instant, as RFC 3339 text.
    fn now(&self) -> Timestamp;
}

impl CommandClock for Timestamp {
    fn now(&self) -> Timestamp {
        self.clone()
    }
}

impl<K: CommandClock + ?Sized> CommandClock for &K {
    fn now(&self) -> Timestamp {
        (**self).now()
    }
}

impl<K: CommandClock + ?Sized> CommandClock for Box<K> {
    fn now(&self) -> Timestamp {
        (**self).now()
    }
}

impl<K: CommandClock + ?Sized> CommandClock for std::sync::Arc<K> {
    fn now(&self) -> Timestamp {
        (**self).now()
    }
}

/// Whether `expires` is at or before `now`, both read as instants, as the generated guards and
/// the conformance interpreter order a `Timestamp`; `None` where either names no instant.
fn due(expires: &Timestamp, now: &Timestamp) -> Option<bool> {
    invariant::compare(
        Some(Fact::text(&expires.0)),
        Op::Le,
        Some(Fact::text(&now.0)),
        true,
        false,
    )
}

/// The 3 `external:` branches of `PostMessage`, which the generated crate does not ask its
/// context port about: the obligation in [`crate::Core`] asks this, one question per branch, in
/// declaration order, after the input-guarded refusals and before any row is read. `true` takes
/// the branch. Object-safe. `unknown-option` is a declared guard, which the core decides from the
/// envelope.
pub trait PostExternals {
    /// `expired-on-arrival`: the service's Clock port reads at or after the post's `expires`.
    ///
    /// # Errors
    ///
    /// The clock reading or the post's `expires` names no instant: the post is refused as an
    /// unmet obligation, never stored.
    fn expired_on_arrival(&mut self, input: &PostMessage) -> Result<bool, UnmetObligation>;

    /// `signature-invalid`: the SignatureVerifier port does not know the key, finds it lapsed, or
    /// finds that the signature does not verify.
    fn signature_invalid(&mut self, input: &PostMessage) -> bool;

    /// `authority-invalid`: the SignatureVerifier port finds no live delegation with this id that
    /// lets `from` speak for `on_behalf_of`.
    fn authority_invalid(&mut self, input: &PostMessage) -> bool;
}

/// The service's context: the generated context port and [`PostExternals`] in one.
///
/// Built from the caller, a verifier, the backend it reads the addressed message from and the
/// service's clock, and nothing else: no outside code sets an answer on it. `shape-mismatch` is
/// computed from the reply and the stored message; the 2 `signature-invalid` and 2
/// `authority-invalid` branches are asked of the verifier; `expired-on-arrival` of `PostMessage`
/// and the 2 `deadline-passed` of `Answer` and `Refuse` compare an `expires` with the clock,
/// read when the command asks.
#[derive(Debug, Clone)]
pub struct ServiceContext<V, B, K> {
    caller: CallerAttributes,
    verifier: V,
    backend: B,
    clock: K,
}

impl<V, B, K> ServiceContext<V, B, K> {
    /// The context of one caller, deciding at `clock`'s readings.
    pub fn new(caller: CallerAttributes, verifier: V, backend: B, clock: K) -> Self {
        Self {
            caller,
            verifier,
            backend,
            clock,
        }
    }

    /// The caller this context answers for.
    pub fn caller(&self) -> &CallerAttributes {
        &self.caller
    }
}

impl<V, B: MessageStorage, K: CommandClock> ServiceContext<V, B, K> {
    /// `deadline-passed`: the message `id` is in `Sent`, `Delivered` or `Read`, and the clock reads
    /// at or after its stored `expires`. A message in another state is not due here: `Answer` and
    /// `Refuse` answer it `already-settled`.
    fn deadline_passed(&self, id: &MessageId) -> Result<bool, UnmetObligation> {
        let Some(held) = self.open(id) else {
            return Ok(false);
        };
        due(&held.data.expires, &self.clock.now())
            .ok_or_else(|| unmet_context("the Clock port's reading against the stored expires"))
    }

    /// `expired-on-arrival` of `Answer`: the message the reply answers is in `Sent`, `Delivered`
    /// or `Read`, as for `deadline-passed`, and the clock reads at or after the reply's own
    /// `expires`. A message in another state is answered `already-settled`, whatever the reply's
    /// `expires`: an answer sent again learns that the first was taken.
    fn reply_expired(&self, reply: &Answer) -> Result<bool, UnmetObligation> {
        if self.open(&reply.in_reply_to).is_none() {
            return Ok(false);
        }
        due(&reply.expires, &self.clock.now())
            .ok_or_else(|| unmet_context("the Clock port's reading against the reply's expires"))
    }

    /// The message `id` where it is open: in `Sent`, `Delivered` or `Read`.
    fn open(&self, id: &MessageId) -> Option<MessageSnapshot> {
        MessageStorage::get(&self.backend, id).filter(|held| {
            matches!(
                held.state,
                MessageState::Sent | MessageState::Delivered | MessageState::Read
            )
        })
    }
}

impl<V: SignatureVerifier, B: MessageStorage, K: CommandClock> TryContext
    for ServiceContext<V, B, K>
{
    fn try_caller_inbox(&self) -> Result<Option<Address>, UnmetObligation> {
        Ok(self.caller.inbox.clone())
    }

    fn try_caller_principal(&self) -> Result<Option<Principal>, UnmetObligation> {
        Ok(self.caller.principal.clone())
    }

    /// The 5 external branches of `Answer` and the 1 of `Refuse`, the only ones the generated
    /// crate asks about. An outcome name a command does not declare is answered as unavailable.
    fn try_external(
        &mut self,
        command: ExternalCommand<'_>,
        outcome: &'static str,
    ) -> Result<bool, UnmetObligation> {
        match (command, outcome) {
            (ExternalCommand::InboxMessagingAnswer(reply), "signature-invalid") => {
                Ok(!self.verifier.verifies(Envelope::Answer(reply)))
            }
            (ExternalCommand::InboxMessagingAnswer(reply), "authority-invalid") => {
                Ok(refuses_authority(&self.verifier, Envelope::Answer(reply)))
            }
            (ExternalCommand::InboxMessagingAnswer(reply), "deadline-passed") => {
                self.deadline_passed(&reply.in_reply_to)
            }
            (ExternalCommand::InboxMessagingAnswer(reply), "expired-on-arrival") => {
                self.reply_expired(reply)
            }
            (ExternalCommand::InboxMessagingAnswer(reply), "shape-mismatch") => {
                Ok(MessageStorage::get(&self.backend, &reply.in_reply_to)
                    .is_some_and(|held| !fits(reply, &held.data)))
            }
            (ExternalCommand::InboxMessagingRefuse(refusal), "deadline-passed") => {
                self.deadline_passed(&refusal.message_id)
            }
            _ => Err(unmet_context("external branch answer")),
        }
    }
}

impl<V: SignatureVerifier, B, K: CommandClock> PostExternals for ServiceContext<V, B, K> {
    fn expired_on_arrival(&mut self, input: &PostMessage) -> Result<bool, UnmetObligation> {
        due(&input.expires, &self.clock.now())
            .ok_or_else(|| unmet_context("the Clock port's reading against the post's expires"))
    }

    fn signature_invalid(&mut self, input: &PostMessage) -> bool {
        !self.verifier.verifies(Envelope::Post(input))
    }

    fn authority_invalid(&mut self, input: &PostMessage) -> bool {
        refuses_authority(&self.verifier, Envelope::Post(input))
    }
}

/// The letters of a decision's options, in order; none where there is no decision.
fn letters(decision: Option<&DecisionAsk>) -> Vec<&OptionLetter> {
    decision
        .map(|decision| {
            decision
                .options
                .iter()
                .map(|option| &option.letter)
                .collect()
        })
        .unwrap_or_default()
}

/// `authority-invalid` is taken where the envelope names a delegation the verifier does not
/// accept. An envelope that names none is `no-authority`'s, or speaks for its own sender.
fn refuses_authority<V: SignatureVerifier>(verifier: &V, envelope: Envelope<'_>) -> bool {
    envelope
        .authority()
        .is_some_and(|authority| !verifier.authorizes(envelope, authority))
}

/// Whether a reply fits the message it answers: its body fits the message's `reply_shape`, a
/// decision is answered with one of its options, and an option names one of the decision's.
fn fits(reply: &Answer, held: &MessageData) -> bool {
    let shaped = match &held.reply_shape {
        None | Some(ReplyShape::Free(_)) => true,
        Some(ReplyShape::Lines(most)) => {
            i64::try_from(reply.body.0.lines().count()).is_ok_and(|lines| lines <= *most)
        }
        Some(ReplyShape::Schema(_)) => json::parse(&reply.body.0).is_ok(),
    };
    let optioned = match &reply.option {
        Some(option) => letters(held.decision.as_ref()).contains(&option),
        None => held.kind != Kind::Decision,
    };
    shaped && optioned
}
