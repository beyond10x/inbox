//! The context port: the caller's attributes and the answers to the 5 `external:` branches.

use inbox::behaviour::{ExternalCommand, MessageStorage, TryContext, unmet_context};
use inbox::messaging::{
    Address, Answer, DecisionAsk, DelegationId, Kind, MessageData, OptionLetter, PostMessage,
    Principal, ReplyShape, Signature,
};
use inbox::obligation::UnmetObligation;
use inbox::primitives::Timestamp;
use inbox::server::json;

/// The attributes of the authenticated caller, as the embedding code hands them in.
///
/// A `Client` carries both; the `Sweeper` carries none.
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

/// The 2 `external:` branches of `PostMessage`, which the generated crate does not ask its
/// context port about: the obligation in [`crate::Core`] asks this, one question per branch, in
/// declaration order, after the input-guarded refusals and before any row is read. `true` takes
/// the branch. Object-safe. `unknown-option` is a declared guard, which the core decides from the
/// envelope.
pub trait PostExternals {
    /// `signature-invalid`: the SignatureVerifier port does not know the key, finds it lapsed, or
    /// finds that the signature does not verify.
    fn signature_invalid(&mut self, input: &PostMessage) -> bool;

    /// `authority-invalid`: the SignatureVerifier port finds no live delegation with this id that
    /// lets `from` speak for `on_behalf_of`.
    fn authority_invalid(&mut self, input: &PostMessage) -> bool;
}

/// The service's context: the generated context port and [`PostExternals`] in one.
///
/// Built from the caller, a verifier and the backend it reads the answered message from, and
/// nothing else: no outside code sets an answer on it. `shape-mismatch` is computed from the reply
/// and the stored message, and the 2
/// `signature-invalid` and 2 `authority-invalid` branches are asked of the verifier.
#[derive(Debug, Clone)]
pub struct ServiceContext<V, B> {
    caller: CallerAttributes,
    verifier: V,
    backend: B,
}

impl<V, B> ServiceContext<V, B> {
    /// The context of one caller.
    pub fn new(caller: CallerAttributes, verifier: V, backend: B) -> Self {
        Self {
            caller,
            verifier,
            backend,
        }
    }

    /// The caller this context answers for.
    pub fn caller(&self) -> &CallerAttributes {
        &self.caller
    }
}

impl<V: SignatureVerifier, B: MessageStorage> TryContext for ServiceContext<V, B> {
    fn try_caller_inbox(&self) -> Result<Option<Address>, UnmetObligation> {
        Ok(self.caller.inbox.clone())
    }

    fn try_caller_principal(&self) -> Result<Option<Principal>, UnmetObligation> {
        Ok(self.caller.principal.clone())
    }

    /// The 3 external branches of `Answer`, the only ones the generated crate asks about. An
    /// outcome name it does not declare is answered as unavailable.
    fn try_external(
        &mut self,
        command: ExternalCommand<'_>,
        outcome: &'static str,
    ) -> Result<bool, UnmetObligation> {
        let ExternalCommand::InboxMessagingAnswer(reply) = command;
        match outcome {
            "signature-invalid" => Ok(!self.verifier.verifies(Envelope::Answer(reply))),
            "authority-invalid" => Ok(refuses_authority(&self.verifier, Envelope::Answer(reply))),
            "shape-mismatch" => Ok(MessageStorage::get(&self.backend, &reply.in_reply_to)
                .is_some_and(|held| !fits(reply, &held.data))),
            _ => Err(unmet_context("external branch answer")),
        }
    }
}

impl<V: SignatureVerifier, B> PostExternals for ServiceContext<V, B> {
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
