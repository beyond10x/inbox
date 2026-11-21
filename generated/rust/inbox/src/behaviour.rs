// generated from inbox v1
// contract digest ab60867d1f6317f8c5efd0d4356af7961995e57473578f829d0b63851c76c2f5
// do not edit: regenerate with `ess synthesize --layout crate`

//! What the specification fully determines, generated: the behaviour of every command the plan
//! lists as generated, written against ports the implementor supplies.
//!
//! Storage is a port: one trait per entity, get, put and delete of a snapshot by identity. ess
//! preserves the trait; generated network entries supply an ephemeral store. `Context` carries the caller's attributes,
//! every identity and value the model says the implementation assigns, and the answer to each
//! `external:` branch. [`Generated`] implements every generated `…Behavior` trait over those ports
//! and forwards every behaviour and query the plan still owes to them, so it is a complete bundle
//! for every component port. To replace one generated behaviour, write a bundle of your own that
//! implements that trait and delegates the rest to a `Generated`.
//!
//! An `Err` from a generated behaviour is the typed refusal naming the command: the model declares
//! no outcome for the request (a guard is undecidable over it, or no declared branch answers it),
//! or — as `entity invariant` — the declared outcome would leave an entity breaking an invariant.

use crate::obligation::UnmetObligation;

/// Where `inbox.messaging.Inbox` is stored — a port the implementor provides.
///
/// Keyed by the identity `address`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait InboxStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::messaging::Address) -> Option<crate::messaging::InboxSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::messaging::InboxSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::messaging::Address);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::messaging::InboxSnapshot>;
}

/// Where `inbox.messaging.Message` is stored — a port the implementor provides.
///
/// Keyed by the identity `message_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait MessageStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::messaging::MessageId) -> Option<crate::messaging::MessageSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::messaging::MessageSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::messaging::MessageId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::messaging::MessageSnapshot>;
}

/// The exact executing command input supplied to an external decision.
///
/// This supplies facts, not authority: the context must verify its request-bound proof.
#[derive(Debug, Clone, Copy)]
pub enum ExternalCommand<'a> {
    /// The executing `inbox.messaging.Answer` input.
    InboxMessagingAnswer(&'a crate::messaging::Answer),
}

impl ExternalCommand<'_> {
    /// The canonical qualified identity of this command.
    pub fn name(&self) -> &'static str {
        match self {
            Self::InboxMessagingAnswer(_) => "inbox.messaging.Answer",
        }
    }
}

/// What the specification leaves to the implementor's context — a port the implementor provides.
///
/// The caller's attributes, the values the model says the implementation assigns, and the answer
/// to each `external:` branch.
pub trait Context {
    /// The authenticated caller's `inbox`, or `None` where the caller carries none.
    fn caller_inbox(&self) -> Option<crate::messaging::Address>;

    /// The authenticated caller's `principal`, or `None` where the caller carries none.
    fn caller_principal(&self) -> Option<crate::messaging::Principal>;

    /// Whether the external branch `outcome` of `command` is taken on this invocation.
    ///
    /// Asked in declaration order, before the branch's input guard is read; the first branch
    /// answered `true` whose guard holds is taken. A test forces a branch by answering `true`
    /// for it alone; a deployment asks whatever decides it.
    fn external(&mut self, command: ExternalCommand<'_>, outcome: &'static str) -> bool;
}

/// Context answers that may be unavailable, without fabricated values.
/// Existing `Context` implementations receive the blanket adapter.
pub trait TryContext {
/// The caller's `inbox`, or a named unavailable answer.
fn try_caller_inbox(&self) -> Result<Option<crate::messaging::Address>, UnmetObligation>;
/// The caller's `principal`, or a named unavailable answer.
fn try_caller_principal(&self) -> Result<Option<crate::messaging::Principal>, UnmetObligation>;
/// Decides the named external branch, or names the unavailable answer.
fn try_external(&mut self, command: ExternalCommand<'_>, outcome: &'static str) -> Result<bool, UnmetObligation>;
}

impl<T: Context + ?Sized> TryContext for T {
fn try_caller_inbox(&self) -> Result<Option<crate::messaging::Address>, UnmetObligation> { Ok(Context::caller_inbox(self)) }
fn try_caller_principal(&self) -> Result<Option<crate::messaging::Principal>, UnmetObligation> { Ok(Context::caller_principal(self)) }
fn try_external(&mut self, command: ExternalCommand<'_>, outcome: &'static str) -> Result<bool, UnmetObligation> { Ok(Context::external(self, command, outcome)) }
}

/// An unavailable runtime context answer, rather than a new planned capability.
pub fn unmet_context(source: &'static str) -> UnmetObligation { UnmetObligation { capability: "context answer", source } }

/// Every generated behaviour of this workspace, over the ports `P` supplies.
///
/// `P` implements the storage trait of each entity a generated behaviour reads or writes,
/// `TryContext` (or its legacy `Context` blanket adapter) where one asks it anything, and every `…Behavior` and `…Query` trait the plan still
/// owes; `Generated<P>` forwards those to it.
pub struct Generated<P> {
    /// The storage and context ports, and every behaviour or query still owed.
    pub ports: P,
}

impl<P> Generated<P> {
    /// The generated behaviours, over `ports`.
    pub fn new(ports: P) -> Self {
        Self { ports }
    }
}

/// `inbox.messaging.Answer`, generated: every outcome is one the specification fully determines.
impl<P> crate::messaging::obligations::AnswerBehavior for Generated<P>
where
    P: TryContext + MessageStorage,
{
    fn answer(&mut self, input: crate::messaging::Answer) -> Result<crate::messaging::AnswerOutcome, UnmetObligation> {
        let _ = &input;
        // `not-sender`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(equal(Some(&input.from).map(|value| &value.0).map(|value| value.clone()), self.ports.try_caller_principal()?.as_ref().map(|value| &value.0).map(|value| value.clone())).map(|value| !value), "inbox.messaging.Answer")? {
            return Ok(crate::messaging::AnswerOutcome::NotSender { error: crate::messaging::NotSender });
        }
        // `no-authority`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(all(&[equal(Some(&input.on_behalf_of).map(|value| &value.0).map(|value| value.clone()), Some(&input.from).map(|value| &value.0).map(|value| value.clone())).map(|value| !value), (Some(Some(&input.authority).and_then(|value| value.as_ref()).map(|value| &value.0).is_some())).map(|value| !value)]), "inbox.messaging.Answer")? {
            return Ok(crate::messaging::AnswerOutcome::NoAuthority { error: crate::messaging::NoAuthority });
        }
        // `already-expired`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(compare_instants(Some(&input.expires).map(|value| value.0.clone()), Some(&input.created).map(|value| value.0.clone()), core::cmp::Ordering::is_le), "inbox.messaging.Answer")? {
            return Ok(crate::messaging::AnswerOutcome::AlreadyExpired { error: crate::messaging::AlreadyExpired });
        }
        // `signature-invalid`: an external refusal answered before the row is read, where the context takes it.
        if self.ports.try_external(ExternalCommand::InboxMessagingAnswer(&input), "signature-invalid")? {
            return Ok(crate::messaging::AnswerOutcome::SignatureInvalid { error: crate::messaging::SignatureInvalid });
        }
        // `authority-invalid`: an external refusal answered before the row is read, where the context takes it.
        if self.ports.try_external(ExternalCommand::InboxMessagingAnswer(&input), "authority-invalid")? {
            return Ok(crate::messaging::AnswerOutcome::AuthorityInvalid { error: crate::messaging::NoAuthority });
        }
        // The addressed row's existence, at step 2 of the one precedence rule.
        if MessageStorage::get(&self.ports, &input.in_reply_to).is_none() {
            return Ok(crate::messaging::AnswerOutcome::UnknownMessage { error: crate::messaging::UnknownMessage });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = MessageStorage::get(&self.ports, &input.in_reply_to) else {
            return Ok(crate::messaging::AnswerOutcome::UnknownMessage { error: crate::messaging::UnknownMessage });
        };
        let _ = &held;
        // `not-recipient`: selected by the addressed row.
        if decided(equal(Some(&held.data.to).map(|value| &value.0).map(|value| value.clone()), self.ports.try_caller_inbox()?.as_ref().map(|value| &value.0).map(|value| value.clone())).map(|value| !value), "inbox.messaging.Answer")? {
            return Ok(crate::messaging::AnswerOutcome::NotRecipient { error: crate::messaging::NotRecipient });
        }
        // `note`: selected by the addressed row.
        if decided(equal(Some(&held.data.kind).map(|value| match value { crate::messaging::Kind::Note => "note", crate::messaging::Kind::Request => "request", crate::messaging::Kind::Decision => "decision" }.to_owned()), Some("note".to_owned())), "inbox.messaging.Answer")? {
            return Ok(crate::messaging::AnswerOutcome::Note { error: crate::messaging::NotAnswerable });
        }
        // `shape-mismatch`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::InboxMessagingAnswer(&input), "shape-mismatch")? {
            return Ok(crate::messaging::AnswerOutcome::ShapeMismatch { error: crate::messaging::ShapeMismatch });
        }
        // `answered`: the default.
        let Some(held) = MessageStorage::get(&self.ports, &input.in_reply_to) else {
            return Ok(crate::messaging::AnswerOutcome::UnknownMessage { error: crate::messaging::UnknownMessage });
        };
        let _ = &held;
        let held_state = held.state;
        let before = held.data.clone();
        let moved = match held.refine() {
            crate::messaging::AnyMessage::Delivered(instance) => crate::messaging::AnyMessage::Answered(instance.answer()),
            crate::messaging::AnyMessage::Read(instance) => crate::messaging::AnyMessage::Answered(instance.answer()),
            crate::messaging::AnyMessage::Sent(instance) => crate::messaging::AnyMessage::Answered(instance.answer()),
            _ => return Ok(crate::messaging::AnswerOutcome::AlreadySettled { error: crate::messaging::AlreadySettled { state: held_state } }),
        };
        let mut next = moved.snapshot();
        next.data.expires = input.expires.clone();
        next.data.answered_by = Some(crate::messaging::AnsweredBy::Principal);
        next.data.decided_option = input.option.clone();
        next.data.reply = Some(crate::messaging::Reply { reply_id: input.reply_id.clone(), from: input.from.clone(), on_behalf_of: input.on_behalf_of.clone(), authority: input.authority.clone(), created: input.created.clone(), option: input.option.clone(), body: input.body.clone(), signature: input.signature.clone() });
        let answer = crate::messaging::AnswerOutcome::Answered { message_answered: crate::messaging::MessageAnswered { message_id: input.in_reply_to.clone(), reply_id: input.reply_id.clone(), to: before.from.clone(), thread: before.thread.clone() } };
        MessageStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `inbox.messaging.CloseInbox`, generated: every outcome is one the specification fully determines.
impl<P> crate::messaging::obligations::CloseInboxBehavior for Generated<P>
where
    P: TryContext + InboxStorage,
{
    fn close_inbox(&mut self, input: crate::messaging::CloseInbox) -> Result<crate::messaging::CloseInboxOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if InboxStorage::get(&self.ports, &input.address).is_none() {
            return Ok(crate::messaging::CloseInboxOutcome::UnknownInbox { error: crate::messaging::UnknownInbox });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = InboxStorage::get(&self.ports, &input.address) else {
            return Ok(crate::messaging::CloseInboxOutcome::UnknownInbox { error: crate::messaging::UnknownInbox });
        };
        let _ = &held;
        // `not-owner`: selected by the addressed row.
        if decided(equal(Some(&held.data.owner).map(|value| &value.0).map(|value| value.clone()), self.ports.try_caller_principal()?.as_ref().map(|value| &value.0).map(|value| value.clone())).map(|value| !value), "inbox.messaging.CloseInbox")? {
            return Ok(crate::messaging::CloseInboxOutcome::NotOwner { error: crate::messaging::NotRecipient });
        }
        // `closed`: selected by the addressed row.
        if decided(Some(matches!(held.state, crate::messaging::InboxState::Open)), "inbox.messaging.CloseInbox")? {
            let Some(held) = InboxStorage::get(&self.ports, &input.address) else {
                return Ok(crate::messaging::CloseInboxOutcome::UnknownInbox { error: crate::messaging::UnknownInbox });
            };
            let _ = &held;
            let moved = match held.refine() {
                crate::messaging::AnyInbox::Open(instance) => crate::messaging::AnyInbox::Closed(instance.close()),
                _ => return Err(undeclared("inbox.messaging.CloseInbox")),
            };
            let next = moved.snapshot();
            let answer = crate::messaging::CloseInboxOutcome::Closed { inbox_closed: crate::messaging::InboxClosed { address: input.address.clone() } };
            InboxStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `already-closed`: the default.
        let Some(held) = InboxStorage::get(&self.ports, &input.address) else {
            return Ok(crate::messaging::CloseInboxOutcome::UnknownInbox { error: crate::messaging::UnknownInbox });
        };
        let _ = &held;
        let answer = crate::messaging::CloseInboxOutcome::AlreadyClosed;
        return Ok(answer);
    }
}

/// `inbox.messaging.Collect`, generated: every outcome is one the specification fully determines.
impl<P> crate::messaging::obligations::CollectBehavior for Generated<P>
where
    P: TryContext + MessageStorage,
{
    fn collect(&mut self, input: crate::messaging::Collect) -> Result<crate::messaging::CollectOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if MessageStorage::get(&self.ports, &input.message_id).is_none() {
            return Ok(crate::messaging::CollectOutcome::UnknownMessage { error: crate::messaging::UnknownMessage });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = MessageStorage::get(&self.ports, &input.message_id) else {
            return Ok(crate::messaging::CollectOutcome::UnknownMessage { error: crate::messaging::UnknownMessage });
        };
        let _ = &held;
        // `not-sender`: selected by the addressed row.
        if decided(equal(Some(&held.data.from).map(|value| &value.0).map(|value| value.clone()), self.ports.try_caller_principal()?.as_ref().map(|value| &value.0).map(|value| value.clone())).map(|value| !value), "inbox.messaging.Collect")? {
            return Ok(crate::messaging::CollectOutcome::NotSender { error: crate::messaging::NotSender });
        }
        // `collected`: selected by the addressed row.
        if decided(Some(matches!(held.state, crate::messaging::MessageState::Answered | crate::messaging::MessageState::Refused)), "inbox.messaging.Collect")? {
            let Some(held) = MessageStorage::get(&self.ports, &input.message_id) else {
                return Ok(crate::messaging::CollectOutcome::UnknownMessage { error: crate::messaging::UnknownMessage });
            };
            let _ = &held;
            let answer = crate::messaging::CollectOutcome::Collected { message_handled: crate::messaging::MessageHandled { message_id: input.message_id.clone() } };
            MessageStorage::delete(&mut self.ports, &input.message_id);
            return Ok(answer);
        }
        // `not-settled`: the default.
        return Ok(crate::messaging::CollectOutcome::NotSettled { error: crate::messaging::NotSettled });
    }
}

impl<P: crate::messaging::obligations::ExpireBehavior> crate::messaging::obligations::ExpireBehavior for Generated<P> {
    fn expire(&mut self, input: crate::messaging::Expire) -> Result<crate::messaging::ExpireOutcome, UnmetObligation> {
        crate::messaging::obligations::ExpireBehavior::expire(&mut self.ports, input)
    }
}

/// `inbox.messaging.MarkDelivered`, generated: every outcome is one the specification fully determines.
impl<P> crate::messaging::obligations::MarkDeliveredBehavior for Generated<P>
where
    P: TryContext + MessageStorage,
{
    fn mark_delivered(&mut self, input: crate::messaging::MarkDelivered) -> Result<crate::messaging::MarkDeliveredOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if MessageStorage::get(&self.ports, &input.message_id).is_none() {
            return Ok(crate::messaging::MarkDeliveredOutcome::UnknownMessage { error: crate::messaging::UnknownMessage });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = MessageStorage::get(&self.ports, &input.message_id) else {
            return Ok(crate::messaging::MarkDeliveredOutcome::UnknownMessage { error: crate::messaging::UnknownMessage });
        };
        let _ = &held;
        // `not-recipient`: selected by the addressed row.
        if decided(equal(Some(&held.data.to).map(|value| &value.0).map(|value| value.clone()), self.ports.try_caller_inbox()?.as_ref().map(|value| &value.0).map(|value| value.clone())).map(|value| !value), "inbox.messaging.MarkDelivered")? {
            return Ok(crate::messaging::MarkDeliveredOutcome::NotRecipient { error: crate::messaging::NotRecipient });
        }
        // `delivered`: selected by the addressed row.
        if decided(Some(matches!(held.state, crate::messaging::MessageState::Sent)), "inbox.messaging.MarkDelivered")? {
            let Some(held) = MessageStorage::get(&self.ports, &input.message_id) else {
                return Ok(crate::messaging::MarkDeliveredOutcome::UnknownMessage { error: crate::messaging::UnknownMessage });
            };
            let _ = &held;
            let moved = match held.refine() {
                crate::messaging::AnyMessage::Sent(instance) => crate::messaging::AnyMessage::Delivered(instance.deliver()),
                _ => return Err(undeclared("inbox.messaging.MarkDelivered")),
            };
            let next = moved.snapshot();
            let answer = crate::messaging::MarkDeliveredOutcome::Delivered { message_delivered: crate::messaging::MessageDelivered { message_id: input.message_id.clone() } };
            MessageStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `already-delivered`: the default.
        let Some(held) = MessageStorage::get(&self.ports, &input.message_id) else {
            return Ok(crate::messaging::MarkDeliveredOutcome::UnknownMessage { error: crate::messaging::UnknownMessage });
        };
        let _ = &held;
        let answer = crate::messaging::MarkDeliveredOutcome::AlreadyDelivered;
        return Ok(answer);
    }
}

/// `inbox.messaging.MarkRead`, generated: every outcome is one the specification fully determines.
impl<P> crate::messaging::obligations::MarkReadBehavior for Generated<P>
where
    P: TryContext + MessageStorage,
{
    fn mark_read(&mut self, input: crate::messaging::MarkRead) -> Result<crate::messaging::MarkReadOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if MessageStorage::get(&self.ports, &input.message_id).is_none() {
            return Ok(crate::messaging::MarkReadOutcome::UnknownMessage { error: crate::messaging::UnknownMessage });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = MessageStorage::get(&self.ports, &input.message_id) else {
            return Ok(crate::messaging::MarkReadOutcome::UnknownMessage { error: crate::messaging::UnknownMessage });
        };
        let _ = &held;
        // `not-recipient`: selected by the addressed row.
        if decided(equal(Some(&held.data.to).map(|value| &value.0).map(|value| value.clone()), self.ports.try_caller_inbox()?.as_ref().map(|value| &value.0).map(|value| value.clone())).map(|value| !value), "inbox.messaging.MarkRead")? {
            return Ok(crate::messaging::MarkReadOutcome::NotRecipient { error: crate::messaging::NotRecipient });
        }
        // `handled`: selected by the addressed row.
        if decided(all(&[equal(Some(&held.data.kind).map(|value| match value { crate::messaging::Kind::Note => "note", crate::messaging::Kind::Request => "request", crate::messaging::Kind::Decision => "decision" }.to_owned()), Some("note".to_owned())), any(&[equal(Some(&held.state).map(|value| match value { crate::messaging::MessageState::Answered => "Answered", crate::messaging::MessageState::Delivered => "Delivered", crate::messaging::MessageState::Read => "Read", crate::messaging::MessageState::Refused => "Refused", crate::messaging::MessageState::Sent => "Sent" }.to_owned()), Some("Sent".to_owned())), equal(Some(&held.state).map(|value| match value { crate::messaging::MessageState::Answered => "Answered", crate::messaging::MessageState::Delivered => "Delivered", crate::messaging::MessageState::Read => "Read", crate::messaging::MessageState::Refused => "Refused", crate::messaging::MessageState::Sent => "Sent" }.to_owned()), Some("Delivered".to_owned()))])]), "inbox.messaging.MarkRead")? {
            let Some(held) = MessageStorage::get(&self.ports, &input.message_id) else {
                return Ok(crate::messaging::MarkReadOutcome::UnknownMessage { error: crate::messaging::UnknownMessage });
            };
            let _ = &held;
            let answer = crate::messaging::MarkReadOutcome::Handled { message_handled: crate::messaging::MessageHandled { message_id: input.message_id.clone() } };
            MessageStorage::delete(&mut self.ports, &input.message_id);
            return Ok(answer);
        }
        // `read`: selected by the addressed row.
        if decided(Some(matches!(held.state, crate::messaging::MessageState::Delivered | crate::messaging::MessageState::Sent)), "inbox.messaging.MarkRead")? {
            let Some(held) = MessageStorage::get(&self.ports, &input.message_id) else {
                return Ok(crate::messaging::MarkReadOutcome::UnknownMessage { error: crate::messaging::UnknownMessage });
            };
            let _ = &held;
            let moved = match held.refine() {
                crate::messaging::AnyMessage::Delivered(instance) => crate::messaging::AnyMessage::Read(instance.read()),
                crate::messaging::AnyMessage::Sent(instance) => crate::messaging::AnyMessage::Read(instance.read()),
                _ => return Err(undeclared("inbox.messaging.MarkRead")),
            };
            let next = moved.snapshot();
            let answer = crate::messaging::MarkReadOutcome::Read { message_read: crate::messaging::MessageRead { message_id: input.message_id.clone() } };
            MessageStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `already-read`: the default.
        let Some(held) = MessageStorage::get(&self.ports, &input.message_id) else {
            return Ok(crate::messaging::MarkReadOutcome::UnknownMessage { error: crate::messaging::UnknownMessage });
        };
        let _ = &held;
        let answer = crate::messaging::MarkReadOutcome::AlreadyRead;
        return Ok(answer);
    }
}

/// `inbox.messaging.OpenInbox`, generated: every outcome is one the specification fully determines.
impl<P> crate::messaging::obligations::OpenInboxBehavior for Generated<P>
where
    P: TryContext + InboxStorage,
{
    fn open_inbox(&mut self, input: crate::messaging::OpenInbox) -> Result<crate::messaging::OpenInboxOutcome, UnmetObligation> {
        let _ = &input;
        // `not-owner`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(equal(Some(&input.address).map(|value| &value.0).map(|value| value.clone()), self.ports.try_caller_inbox()?.as_ref().map(|value| &value.0).map(|value| value.clone())).map(|value| !value), "inbox.messaging.OpenInbox")? {
            return Ok(crate::messaging::OpenInboxOutcome::NotOwner { error: crate::messaging::NotRecipient });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = InboxStorage::get(&self.ports, &input.address) else {
            let identity: crate::messaging::Address = input.address.clone();
            let data = crate::messaging::InboxData {
                address: identity.clone(),
                owner: self.ports.try_caller_principal()?.ok_or_else(|| undeclared("inbox.messaging.OpenInbox"))?,
                adapter: input.adapter.clone(),
            };
            let answer = crate::messaging::OpenInboxOutcome::Opened { inbox_opened: crate::messaging::InboxOpened { address: identity.clone() } };
            InboxStorage::put(&mut self.ports, crate::messaging::AnyInbox::Open(crate::messaging::Inbox::new(data)).snapshot());
            return Ok(answer);
        };
        let _ = &held;
        // `reopened`: selected by the addressed row.
        if decided(Some(matches!(held.state, crate::messaging::InboxState::Closed)), "inbox.messaging.OpenInbox")? {
            let Some(held) = InboxStorage::get(&self.ports, &input.address) else {
                let identity: crate::messaging::Address = input.address.clone();
                let data = crate::messaging::InboxData {
                    address: identity.clone(),
                    owner: self.ports.try_caller_principal()?.ok_or_else(|| undeclared("inbox.messaging.OpenInbox"))?,
                    adapter: input.adapter.clone(),
                };
                let answer = crate::messaging::OpenInboxOutcome::Opened { inbox_opened: crate::messaging::InboxOpened { address: identity.clone() } };
                InboxStorage::put(&mut self.ports, crate::messaging::AnyInbox::Open(crate::messaging::Inbox::new(data)).snapshot());
                return Ok(answer);
            };
            let _ = &held;
            let moved = match held.refine() {
                crate::messaging::AnyInbox::Closed(instance) => crate::messaging::AnyInbox::Open(instance.reopen()),
                _ => return Err(undeclared("inbox.messaging.OpenInbox")),
            };
            let mut next = moved.snapshot();
            next.data.adapter = input.adapter.clone();
            let answer = crate::messaging::OpenInboxOutcome::Reopened { inbox_opened: crate::messaging::InboxOpened { address: input.address.clone() } };
            InboxStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `already-open`: the default.
        let Some(held) = InboxStorage::get(&self.ports, &input.address) else {
            let identity: crate::messaging::Address = input.address.clone();
            let data = crate::messaging::InboxData {
                address: identity.clone(),
                owner: self.ports.try_caller_principal()?.ok_or_else(|| undeclared("inbox.messaging.OpenInbox"))?,
                adapter: input.adapter.clone(),
            };
            let answer = crate::messaging::OpenInboxOutcome::Opened { inbox_opened: crate::messaging::InboxOpened { address: identity.clone() } };
            InboxStorage::put(&mut self.ports, crate::messaging::AnyInbox::Open(crate::messaging::Inbox::new(data)).snapshot());
            return Ok(answer);
        };
        let _ = &held;
        let answer = crate::messaging::OpenInboxOutcome::AlreadyOpen;
        return Ok(answer);
    }
}

impl<P: crate::messaging::obligations::PostMessageBehavior> crate::messaging::obligations::PostMessageBehavior for Generated<P> {
    fn post_message(&mut self, input: crate::messaging::PostMessage) -> Result<crate::messaging::PostMessageOutcome, UnmetObligation> {
        crate::messaging::obligations::PostMessageBehavior::post_message(&mut self.ports, input)
    }
}

/// `inbox.messaging.Refuse`, generated: every outcome is one the specification fully determines.
impl<P> crate::messaging::obligations::RefuseBehavior for Generated<P>
where
    P: TryContext + MessageStorage,
{
    fn refuse(&mut self, input: crate::messaging::Refuse) -> Result<crate::messaging::RefuseOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if MessageStorage::get(&self.ports, &input.message_id).is_none() {
            return Ok(crate::messaging::RefuseOutcome::UnknownMessage { error: crate::messaging::UnknownMessage });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = MessageStorage::get(&self.ports, &input.message_id) else {
            return Ok(crate::messaging::RefuseOutcome::UnknownMessage { error: crate::messaging::UnknownMessage });
        };
        let _ = &held;
        // `not-recipient`: selected by the addressed row.
        if decided(equal(Some(&held.data.to).map(|value| &value.0).map(|value| value.clone()), self.ports.try_caller_inbox()?.as_ref().map(|value| &value.0).map(|value| value.clone())).map(|value| !value), "inbox.messaging.Refuse")? {
            return Ok(crate::messaging::RefuseOutcome::NotRecipient { error: crate::messaging::NotRecipient });
        }
        // `refused`: the default.
        let Some(held) = MessageStorage::get(&self.ports, &input.message_id) else {
            return Ok(crate::messaging::RefuseOutcome::UnknownMessage { error: crate::messaging::UnknownMessage });
        };
        let _ = &held;
        let held_state = held.state;
        let before = held.data.clone();
        let moved = match held.refine() {
            crate::messaging::AnyMessage::Delivered(instance) => crate::messaging::AnyMessage::Refused(instance.refuse()),
            crate::messaging::AnyMessage::Read(instance) => crate::messaging::AnyMessage::Refused(instance.refuse()),
            crate::messaging::AnyMessage::Sent(instance) => crate::messaging::AnyMessage::Refused(instance.refuse()),
            _ => return Ok(crate::messaging::RefuseOutcome::AlreadySettled { error: crate::messaging::AlreadySettled { state: held_state } }),
        };
        let mut next = moved.snapshot();
        next.data.refusal = Some(input.reason.clone());
        let answer = crate::messaging::RefuseOutcome::Refused { message_refused: crate::messaging::MessageRefused { message_id: input.message_id.clone(), to: before.from.clone(), thread: before.thread.clone() } };
        MessageStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `inbox.messaging.Due`, generated: every row is one the specification fully determines from the stored `inbox.messaging.Message`s.
impl<P> crate::messaging::obligations::DueQuery for Generated<P>
where
    P: MessageStorage,
{
    fn due(&self, param_0: crate::primitives::Timestamp) -> Result<Vec<crate::messaging::Due>, UnmetObligation> {
        let mut admitted = MessageStorage::list(&self.ports);
        // `filter:` shows a row where it holds; false or unknown hides it.
        admitted.retain(|held| compare_instants(Some(&held.data.expires).map(|value| value.0.clone()), Some(&param_0).map(|value| value.0.clone()), core::cmp::Ordering::is_le) == Some(true));
        Ok(admitted
            .into_iter()
            .map(|held| crate::messaging::Due {
                message_id: held.data.message_id,
                from: held.data.from,
                on_behalf_of: held.data.on_behalf_of,
                authority: held.data.authority,
                to: held.data.to,
                kind: held.data.kind,
                thread: held.data.thread,
                reply_shape: held.data.reply_shape,
                decision: held.data.decision,
                default_on_silence: held.data.default_on_silence,
                created: held.data.created,
                expires: held.data.expires,
                dedup_key: held.data.dedup_key,
                body: held.data.body,
                signature: held.data.signature,
                answered_by: held.data.answered_by,
                decided_option: held.data.decided_option,
                reply: held.data.reply,
                refusal: held.data.refusal,
                state: held.state,
            })
            .collect())
    }
}

/// `inbox.messaging.Inboxes`, generated: every row is one the specification fully determines from the stored `inbox.messaging.Inbox`s.
impl<P> crate::messaging::obligations::InboxesQuery for Generated<P>
where
    P: InboxStorage,
{
    fn inboxes(&self) -> Result<Vec<crate::messaging::Inboxes>, UnmetObligation> {
        let admitted = InboxStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::messaging::Inboxes {
                address: held.data.address,
                owner: held.data.owner,
                adapter: held.data.adapter,
                state: held.state,
            })
            .collect())
    }
}

/// `inbox.messaging.Messages`, generated: every row is one the specification fully determines from the stored `inbox.messaging.Message`s.
impl<P> crate::messaging::obligations::MessagesQuery for Generated<P>
where
    P: MessageStorage,
{
    fn messages(&self) -> Result<Vec<crate::messaging::Messages>, UnmetObligation> {
        let admitted = MessageStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::messaging::Messages {
                message_id: held.data.message_id,
                from: held.data.from,
                on_behalf_of: held.data.on_behalf_of,
                authority: held.data.authority,
                to: held.data.to,
                kind: held.data.kind,
                thread: held.data.thread,
                reply_shape: held.data.reply_shape,
                decision: held.data.decision,
                default_on_silence: held.data.default_on_silence,
                created: held.data.created,
                expires: held.data.expires,
                dedup_key: held.data.dedup_key,
                body: held.data.body,
                signature: held.data.signature,
                answered_by: held.data.answered_by,
                decided_option: held.data.decided_option,
                reply: held.data.reply,
                refusal: held.data.refusal,
                state: held.state,
            })
            .collect())
    }
}

/// `inbox.messaging.Outbox`, generated: every row is one the specification fully determines from the stored `inbox.messaging.Message`s.
impl<P> crate::messaging::obligations::OutboxQuery for Generated<P>
where
    P: MessageStorage,
{
    fn outbox(&self, param_0: crate::messaging::Principal) -> Result<Vec<crate::messaging::Outbox>, UnmetObligation> {
        let mut admitted = MessageStorage::list(&self.ports);
        // `filter:` shows a row where it holds; false or unknown hides it.
        admitted.retain(|held| equal(Some(&held.data.from).map(|value| &value.0).map(|value| value.clone()), Some(&param_0).map(|value| &value.0).map(|value| value.clone())) == Some(true));
        Ok(admitted
            .into_iter()
            .map(|held| crate::messaging::Outbox {
                message_id: held.data.message_id,
                from: held.data.from,
                on_behalf_of: held.data.on_behalf_of,
                authority: held.data.authority,
                to: held.data.to,
                kind: held.data.kind,
                thread: held.data.thread,
                reply_shape: held.data.reply_shape,
                decision: held.data.decision,
                default_on_silence: held.data.default_on_silence,
                created: held.data.created,
                expires: held.data.expires,
                dedup_key: held.data.dedup_key,
                body: held.data.body,
                signature: held.data.signature,
                answered_by: held.data.answered_by,
                decided_option: held.data.decided_option,
                reply: held.data.reply,
                refusal: held.data.refusal,
                state: held.state,
            })
            .collect())
    }
}

/// `inbox.messaging.Pending`, generated: every row is one the specification fully determines from the stored `inbox.messaging.Message`s.
impl<P> crate::messaging::obligations::PendingQuery for Generated<P>
where
    P: MessageStorage,
{
    fn pending(&self, param_0: crate::messaging::Address) -> Result<Vec<crate::messaging::Pending>, UnmetObligation> {
        let mut admitted = MessageStorage::list(&self.ports);
        // `filter:` shows a row where it holds; false or unknown hides it.
        admitted.retain(|held| all(&[equal(Some(&held.data.to).map(|value| &value.0).map(|value| value.clone()), Some(&param_0).map(|value| &value.0).map(|value| value.clone())), any(&[equal(Some(&held.state).map(|value| match value { crate::messaging::MessageState::Answered => "Answered", crate::messaging::MessageState::Delivered => "Delivered", crate::messaging::MessageState::Read => "Read", crate::messaging::MessageState::Refused => "Refused", crate::messaging::MessageState::Sent => "Sent" }.to_owned()), Some("Sent".to_owned())), equal(Some(&held.state).map(|value| match value { crate::messaging::MessageState::Answered => "Answered", crate::messaging::MessageState::Delivered => "Delivered", crate::messaging::MessageState::Read => "Read", crate::messaging::MessageState::Refused => "Refused", crate::messaging::MessageState::Sent => "Sent" }.to_owned()), Some("Delivered".to_owned()))])]) == Some(true));
        Ok(admitted
            .into_iter()
            .map(|held| crate::messaging::Pending {
                message_id: held.data.message_id,
                from: held.data.from,
                on_behalf_of: held.data.on_behalf_of,
                authority: held.data.authority,
                to: held.data.to,
                kind: held.data.kind,
                thread: held.data.thread,
                reply_shape: held.data.reply_shape,
                decision: held.data.decision,
                default_on_silence: held.data.default_on_silence,
                created: held.data.created,
                expires: held.data.expires,
                dedup_key: held.data.dedup_key,
                body: held.data.body,
                signature: held.data.signature,
                state: held.state,
            })
            .collect())
    }
}

/// `inbox.messaging.Replies`, generated: every row is one the specification fully determines from the stored `inbox.messaging.Message`s.
impl<P> crate::messaging::obligations::RepliesQuery for Generated<P>
where
    P: MessageStorage,
{
    fn replies(&self, param_0: crate::messaging::ThreadId) -> Result<Vec<crate::messaging::Replies>, UnmetObligation> {
        let mut admitted = MessageStorage::list(&self.ports);
        // `filter:` shows a row where it holds; false or unknown hides it.
        admitted.retain(|held| all(&[equal(Some(&held.data.thread).map(|value| &value.0).map(|value| value.clone()), Some(&param_0).map(|value| &value.0).map(|value| value.clone())), equal(Some(&held.state).map(|value| match value { crate::messaging::MessageState::Answered => "Answered", crate::messaging::MessageState::Delivered => "Delivered", crate::messaging::MessageState::Read => "Read", crate::messaging::MessageState::Refused => "Refused", crate::messaging::MessageState::Sent => "Sent" }.to_owned()), Some("Answered".to_owned()))]) == Some(true));
        Ok(admitted
            .into_iter()
            .map(|held| crate::messaging::Replies {
                message_id: held.data.message_id,
                from: held.data.from,
                on_behalf_of: held.data.on_behalf_of,
                authority: held.data.authority,
                to: held.data.to,
                kind: held.data.kind,
                thread: held.data.thread,
                reply_shape: held.data.reply_shape,
                decision: held.data.decision,
                default_on_silence: held.data.default_on_silence,
                created: held.data.created,
                expires: held.data.expires,
                dedup_key: held.data.dedup_key,
                body: held.data.body,
                signature: held.data.signature,
                answered_by: held.data.answered_by,
                decided_option: held.data.decided_option,
                reply: held.data.reply,
                state: held.state,
            })
            .collect())
    }
}

/// `inbox.messaging.Settled`, generated: every row is one the specification fully determines from the stored `inbox.messaging.Message`s.
impl<P> crate::messaging::obligations::SettledQuery for Generated<P>
where
    P: MessageStorage,
{
    fn settled(&self, param_0: crate::messaging::Principal) -> Result<Vec<crate::messaging::Settled>, UnmetObligation> {
        let mut admitted = MessageStorage::list(&self.ports);
        // `filter:` shows a row where it holds; false or unknown hides it.
        admitted.retain(|held| all(&[equal(Some(&held.data.from).map(|value| &value.0).map(|value| value.clone()), Some(&param_0).map(|value| &value.0).map(|value| value.clone())), any(&[equal(Some(&held.state).map(|value| match value { crate::messaging::MessageState::Answered => "Answered", crate::messaging::MessageState::Delivered => "Delivered", crate::messaging::MessageState::Read => "Read", crate::messaging::MessageState::Refused => "Refused", crate::messaging::MessageState::Sent => "Sent" }.to_owned()), Some("Answered".to_owned())), equal(Some(&held.state).map(|value| match value { crate::messaging::MessageState::Answered => "Answered", crate::messaging::MessageState::Delivered => "Delivered", crate::messaging::MessageState::Read => "Read", crate::messaging::MessageState::Refused => "Refused", crate::messaging::MessageState::Sent => "Sent" }.to_owned()), Some("Refused".to_owned()))])]) == Some(true));
        Ok(admitted
            .into_iter()
            .map(|held| crate::messaging::Settled {
                message_id: held.data.message_id,
                from: held.data.from,
                on_behalf_of: held.data.on_behalf_of,
                authority: held.data.authority,
                to: held.data.to,
                kind: held.data.kind,
                thread: held.data.thread,
                reply_shape: held.data.reply_shape,
                decision: held.data.decision,
                default_on_silence: held.data.default_on_silence,
                created: held.data.created,
                expires: held.data.expires,
                dedup_key: held.data.dedup_key,
                body: held.data.body,
                signature: held.data.signature,
                answered_by: held.data.answered_by,
                decided_option: held.data.decided_option,
                reply: held.data.reply,
                refusal: held.data.refusal,
                state: held.state,
            })
            .collect())
    }
}

/// `inbox.messaging.Thread`, generated: every row is one the specification fully determines from the stored `inbox.messaging.Message`s.
impl<P> crate::messaging::obligations::ThreadQuery for Generated<P>
where
    P: MessageStorage,
{
    fn thread(&self, param_0: crate::messaging::ThreadId) -> Result<Vec<crate::messaging::Thread>, UnmetObligation> {
        let mut admitted = MessageStorage::list(&self.ports);
        // `filter:` shows a row where it holds; false or unknown hides it.
        admitted.retain(|held| equal(Some(&held.data.thread).map(|value| &value.0).map(|value| value.clone()), Some(&param_0).map(|value| &value.0).map(|value| value.clone())) == Some(true));
        Ok(admitted
            .into_iter()
            .map(|held| crate::messaging::Thread {
                message_id: held.data.message_id,
                from: held.data.from,
                on_behalf_of: held.data.on_behalf_of,
                authority: held.data.authority,
                to: held.data.to,
                kind: held.data.kind,
                thread: held.data.thread,
                reply_shape: held.data.reply_shape,
                decision: held.data.decision,
                default_on_silence: held.data.default_on_silence,
                created: held.data.created,
                expires: held.data.expires,
                dedup_key: held.data.dedup_key,
                body: held.data.body,
                signature: held.data.signature,
                answered_by: held.data.answered_by,
                decided_option: held.data.decided_option,
                reply: held.data.reply,
                refusal: held.data.refusal,
                state: held.state,
            })
            .collect())
    }
}

/// `inbox.messaging.Unanswered`, generated: every row is one the specification fully determines from the stored `inbox.messaging.Message`s.
impl<P> crate::messaging::obligations::UnansweredQuery for Generated<P>
where
    P: MessageStorage,
{
    fn unanswered(&self, param_0: crate::messaging::Address) -> Result<Vec<crate::messaging::Unanswered>, UnmetObligation> {
        let mut admitted = MessageStorage::list(&self.ports);
        // `filter:` shows a row where it holds; false or unknown hides it.
        admitted.retain(|held| all(&[equal(Some(&held.data.to).map(|value| &value.0).map(|value| value.clone()), Some(&param_0).map(|value| &value.0).map(|value| value.clone())), equal(Some(&held.state).map(|value| match value { crate::messaging::MessageState::Answered => "Answered", crate::messaging::MessageState::Delivered => "Delivered", crate::messaging::MessageState::Read => "Read", crate::messaging::MessageState::Refused => "Refused", crate::messaging::MessageState::Sent => "Sent" }.to_owned()), Some("Read".to_owned()))]) == Some(true));
        Ok(admitted
            .into_iter()
            .map(|held| crate::messaging::Unanswered {
                message_id: held.data.message_id,
                from: held.data.from,
                on_behalf_of: held.data.on_behalf_of,
                authority: held.data.authority,
                to: held.data.to,
                kind: held.data.kind,
                thread: held.data.thread,
                reply_shape: held.data.reply_shape,
                decision: held.data.decision,
                default_on_silence: held.data.default_on_silence,
                created: held.data.created,
                expires: held.data.expires,
                dedup_key: held.data.dedup_key,
                body: held.data.body,
                signature: held.data.signature,
                state: held.state,
            })
            .collect())
    }
}

/// The typed refusal of a request the model declares no outcome for.
fn undeclared(source: &'static str) -> UnmetObligation {
    let capability = "command behaviour";
    UnmetObligation { capability, source }
}

/// A guard's truth, where it has one: Unknown selects no branch, so the model declares no outcome.
fn decided(truth: Option<bool>, command: &'static str) -> Result<bool, UnmetObligation> {
    truth.ok_or_else(|| undeclared(command))
}

/// Three-valued conjunction: false wins, then Unknown.
fn all(truths: &[Option<bool>]) -> Option<bool> {
    if truths.contains(&Some(false)) {
        Some(false)
    } else if truths.contains(&None) {
        None
    } else {
        Some(true)
    }
}

/// Three-valued disjunction: true wins, then Unknown.
fn any(truths: &[Option<bool>]) -> Option<bool> {
    if truths.contains(&Some(true)) {
        Some(true)
    } else if truths.contains(&None) {
        None
    } else {
        Some(false)
    }
}

/// Equality of two read values; an unread one is Unknown.
fn equal<T: PartialEq>(left: Option<T>, right: Option<T>) -> Option<bool> {
    Some(left? == right?)
}

/// Compares two RFC 3339 renderings by the instant each names; an unread one, or one that names
/// no instant, is Unknown — never ordered by its spelling.
fn compare_instants(
    left: Option<String>,
    right: Option<String>,
    accepts: fn(core::cmp::Ordering) -> bool,
) -> Option<bool> {
    Some(accepts(instant_of(&left?)?.cmp(&instant_of(&right?)?)))
}

/// The instant an RFC 3339 `date-time` names, as seconds from the epoch and nanoseconds.
fn instant_of(text: &str) -> Option<(i64, u32)> {
    let bytes = text.as_bytes();
    let digits = |from: usize, to: usize| -> Option<u32> {
        let slice = bytes.get(from..to)?;
        if slice.is_empty() || !slice.iter().all(u8::is_ascii_digit) {
            return None;
        }
        slice
            .iter()
            .try_fold(0u32, |total, digit| Some(total * 10 + u32::from(digit - b'0')))
    };
    let at = |index: usize, expected: &[u8]| bytes.get(index).is_some_and(|b| expected.contains(b));
    if !(at(4, b"-") && at(7, b"-") && at(10, b"Tt") && at(13, b":") && at(16, b":")) {
        return None;
    }
    let (year, month, day) = (i64::from(digits(0, 4)?), digits(5, 7)?, digits(8, 10)?);
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let length = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    let (hour, minute, second) = (digits(11, 13)?, digits(14, 16)?, digits(17, 19)?);
    if day < 1 || day > length || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let mut position = 19;
    let mut nanos = 0u32;
    if at(position, b".") {
        let mut end = position + 1;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        let width = end - position - 1;
        if width == 0 || width > 9 {
            return None;
        }
        nanos = digits(position + 1, end)? * 10u32.pow(u32::try_from(9 - width).ok()?);
        position = end;
    }
    let offset = match bytes.get(position..)? {
        b"Z" | b"z" => 0i64,
        [sign @ (b'+' | b'-'), _, _, b':', _, _] => {
            let (hours, minutes) = (digits(position + 1, position + 3)?, digits(position + 4, position + 6)?);
            if hours > 23 || minutes > 59 {
                return None;
            }
            let magnitude = i64::from(hours * 3600 + minutes * 60);
            if *sign == b'-' { -magnitude } else { magnitude }
        }
        _ => return None,
    };
    let shifted = if month <= 2 { year - 1 } else { year };
    let era = shifted.div_euclid(400);
    let of_era = shifted - era * 400;
    let march = i64::from(if month > 2 { month - 3 } else { month + 9 });
    let of_year = (153 * march + 2) / 5 + i64::from(day) - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    let days = era * 146_097 + of_cycle - 719_468;
    Some((days * 86_400 + i64::from(hour * 3600 + minute * 60 + second) - offset, nanos))
}
