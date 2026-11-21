// generated from inbox v1
// contract digest d43aaff41f76331aceb590e4dc67c9411ad482322f4b4f286990e94b49e5fe69
// do not edit: regenerate with `ess synthesize --layout crate`

//! inbox-service — the `inbox-service` component of `inbox` v1.
//!
//! The one service every client talks to (`inbox serve`). It holds the inboxes and their messages in the backend it was started with, checks every post and every answer through the SignatureVerifier port, and removes a message when it is handled or its `expires` passes.
//!
//! The component's outer surface exactly as the specification declares it: accepted commands as
//! handlers, declared views as queries, published events as a typed outbox. The behaviour behind
//! every handler is an implementation obligation — see the `PLAN.md` beside this workspace — and
//! until one is satisfied, its stub answers with a typed refusal naming what is owed.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// An event this component declares it publishes, on its way to the system's transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishedEvent {
    /// `inbox.messaging.DecisionTakenOnSilence`.
    DecisionTakenOnSilence(crate::messaging::DecisionTakenOnSilence),
    /// `inbox.messaging.InboxClosed`.
    InboxClosed(crate::messaging::InboxClosed),
    /// `inbox.messaging.InboxOpened`.
    InboxOpened(crate::messaging::InboxOpened),
    /// `inbox.messaging.MessageAnswered`.
    MessageAnswered(crate::messaging::MessageAnswered),
    /// `inbox.messaging.MessageDelivered`.
    MessageDelivered(crate::messaging::MessageDelivered),
    /// `inbox.messaging.MessageExpired`.
    MessageExpired(crate::messaging::MessageExpired),
    /// `inbox.messaging.MessageHandled`.
    MessageHandled(crate::messaging::MessageHandled),
    /// `inbox.messaging.MessagePosted`.
    MessagePosted(crate::messaging::MessagePosted),
    /// `inbox.messaging.MessageRead`.
    MessageRead(crate::messaging::MessageRead),
    /// `inbox.messaging.MessageRefused`.
    MessageRefused(crate::messaging::MessageRefused),
}

/// inbox-service — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct InboxService<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> InboxService<B> {
    /// A new port over the given obligation implementations.
    pub fn new(behaviors: B) -> Self {
        Self {
            behaviors,
            outbox: Vec::new(),
        }
    }

    /// Hands over everything published since the last drain, in publication order.
    ///
    /// The system's transport calls this; anything else reading it is taking events the transport
    /// will then never deliver.
    pub fn drain_outbox(&mut self) -> Vec<PublishedEvent> {
        core::mem::take(&mut self.outbox)
    }
}

impl<B> InboxService<B>
where
    B: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    /// Accepts `inbox.messaging.Answer`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn answer(&mut self, input: crate::messaging::Answer) -> Result<crate::messaging::AnswerOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.answer(input)?;
        match &outcome {
            crate::messaging::AnswerOutcome::NoAuthority { .. } => {}
            crate::messaging::AnswerOutcome::AlreadyExpired { .. } => {}
            crate::messaging::AnswerOutcome::SignatureInvalid { .. } => {}
            crate::messaging::AnswerOutcome::AuthorityInvalid { .. } => {}
            crate::messaging::AnswerOutcome::UnknownMessage { .. } => {}
            crate::messaging::AnswerOutcome::NotRecipient { .. } => {}
            crate::messaging::AnswerOutcome::AlreadySettled { .. } => {}
            crate::messaging::AnswerOutcome::Note { .. } => {}
            crate::messaging::AnswerOutcome::ShapeMismatch { .. } => {}
            crate::messaging::AnswerOutcome::Answered { message_answered, .. } => {
                self.outbox.push(PublishedEvent::MessageAnswered(message_answered.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `inbox.messaging.CloseInbox`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn close_inbox(&mut self, input: crate::messaging::CloseInbox) -> Result<crate::messaging::CloseInboxOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.close_inbox(input)?;
        match &outcome {
            crate::messaging::CloseInboxOutcome::UnknownInbox { .. } => {}
            crate::messaging::CloseInboxOutcome::NotOwner { .. } => {}
            crate::messaging::CloseInboxOutcome::Closed { inbox_closed, .. } => {
                self.outbox.push(PublishedEvent::InboxClosed(inbox_closed.clone()));
            }
            crate::messaging::CloseInboxOutcome::AlreadyClosed { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `inbox.messaging.Collect`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn collect(&mut self, input: crate::messaging::Collect) -> Result<crate::messaging::CollectOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.collect(input)?;
        match &outcome {
            crate::messaging::CollectOutcome::UnknownMessage { .. } => {}
            crate::messaging::CollectOutcome::NotSender { .. } => {}
            crate::messaging::CollectOutcome::Collected { message_handled, .. } => {
                self.outbox.push(PublishedEvent::MessageHandled(message_handled.clone()));
            }
            crate::messaging::CollectOutcome::NotSettled { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `inbox.messaging.Expire`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn expire(&mut self, input: crate::messaging::Expire) -> Result<crate::messaging::ExpireOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.expire(input)?;
        match &outcome {
            crate::messaging::ExpireOutcome::UnknownMessage { .. } => {}
            crate::messaging::ExpireOutcome::NotDue { .. } => {}
            crate::messaging::ExpireOutcome::DecidedOnSilence { decision_taken_on_silence, .. } => {
                self.outbox.push(PublishedEvent::DecisionTakenOnSilence(decision_taken_on_silence.clone()));
            }
            crate::messaging::ExpireOutcome::Expired { message_expired, .. } => {
                self.outbox.push(PublishedEvent::MessageExpired(message_expired.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `inbox.messaging.MarkDelivered`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn mark_delivered(&mut self, input: crate::messaging::MarkDelivered) -> Result<crate::messaging::MarkDeliveredOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.mark_delivered(input)?;
        match &outcome {
            crate::messaging::MarkDeliveredOutcome::UnknownMessage { .. } => {}
            crate::messaging::MarkDeliveredOutcome::NotRecipient { .. } => {}
            crate::messaging::MarkDeliveredOutcome::Delivered { message_delivered, .. } => {
                self.outbox.push(PublishedEvent::MessageDelivered(message_delivered.clone()));
            }
            crate::messaging::MarkDeliveredOutcome::AlreadyDelivered { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `inbox.messaging.MarkRead`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn mark_read(&mut self, input: crate::messaging::MarkRead) -> Result<crate::messaging::MarkReadOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.mark_read(input)?;
        match &outcome {
            crate::messaging::MarkReadOutcome::UnknownMessage { .. } => {}
            crate::messaging::MarkReadOutcome::NotRecipient { .. } => {}
            crate::messaging::MarkReadOutcome::Handled { message_handled, .. } => {
                self.outbox.push(PublishedEvent::MessageHandled(message_handled.clone()));
            }
            crate::messaging::MarkReadOutcome::Read { message_read, .. } => {
                self.outbox.push(PublishedEvent::MessageRead(message_read.clone()));
            }
            crate::messaging::MarkReadOutcome::AlreadyRead { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `inbox.messaging.OpenInbox`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn open_inbox(&mut self, input: crate::messaging::OpenInbox) -> Result<crate::messaging::OpenInboxOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.open_inbox(input)?;
        match &outcome {
            crate::messaging::OpenInboxOutcome::NotOwner { .. } => {}
            crate::messaging::OpenInboxOutcome::Opened { inbox_opened, .. } => {
                self.outbox.push(PublishedEvent::InboxOpened(inbox_opened.clone()));
            }
            crate::messaging::OpenInboxOutcome::Reopened { inbox_opened, .. } => {
                self.outbox.push(PublishedEvent::InboxOpened(inbox_opened.clone()));
            }
            crate::messaging::OpenInboxOutcome::AlreadyOpen { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `inbox.messaging.PostMessage`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn post_message(&mut self, input: crate::messaging::PostMessage) -> Result<crate::messaging::PostMessageOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.post_message(input)?;
        match &outcome {
            crate::messaging::PostMessageOutcome::NoAuthority { .. } => {}
            crate::messaging::PostMessageOutcome::AlreadyExpired { .. } => {}
            crate::messaging::PostMessageOutcome::RequestWithoutShape { .. } => {}
            crate::messaging::PostMessageOutcome::DecisionWithoutOptions { .. } => {}
            crate::messaging::PostMessageOutcome::UnknownOption { .. } => {}
            crate::messaging::PostMessageOutcome::SignatureInvalid { .. } => {}
            crate::messaging::PostMessageOutcome::AuthorityInvalid { .. } => {}
            crate::messaging::PostMessageOutcome::Duplicate { .. } => {}
            crate::messaging::PostMessageOutcome::DuplicateKey { .. } => {}
            crate::messaging::PostMessageOutcome::UnknownInbox { .. } => {}
            crate::messaging::PostMessageOutcome::InboxClosed { .. } => {}
            crate::messaging::PostMessageOutcome::Posted { message_posted, .. } => {
                self.outbox.push(PublishedEvent::MessagePosted(message_posted.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `inbox.messaging.Refuse`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn refuse(&mut self, input: crate::messaging::Refuse) -> Result<crate::messaging::RefuseOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.refuse(input)?;
        match &outcome {
            crate::messaging::RefuseOutcome::UnknownMessage { .. } => {}
            crate::messaging::RefuseOutcome::NotRecipient { .. } => {}
            crate::messaging::RefuseOutcome::Refused { message_refused, .. } => {
                self.outbox.push(PublishedEvent::MessageRefused(message_refused.clone()));
            }
            crate::messaging::RefuseOutcome::AlreadySettled { .. } => {}
        }
        Ok(outcome)
    }

    /// Serves `inbox.messaging.Due` at `read_your_writes` consistency, from the owed projection.
    pub fn due(&self, now: crate::primitives::Timestamp) -> Result<Vec<crate::messaging::Due>, crate::obligation::UnmetObligation> {
        self.behaviors.due(now)
    }

    /// Serves `inbox.messaging.Inboxes` at `read_your_writes` consistency, from the owed projection.
    pub fn inboxes(&self) -> Result<Vec<crate::messaging::Inboxes>, crate::obligation::UnmetObligation> {
        self.behaviors.inboxes()
    }

    /// Serves `inbox.messaging.Messages` at `read_your_writes` consistency, from the owed projection.
    pub fn messages(&self) -> Result<Vec<crate::messaging::Messages>, crate::obligation::UnmetObligation> {
        self.behaviors.messages()
    }

    /// Serves `inbox.messaging.Outbox` at `read_your_writes` consistency, from the owed projection.
    pub fn outbox(&self, sender: crate::messaging::Principal) -> Result<Vec<crate::messaging::Outbox>, crate::obligation::UnmetObligation> {
        self.behaviors.outbox(sender)
    }

    /// Serves `inbox.messaging.Pending` at `read_your_writes` consistency, from the owed projection.
    pub fn pending(&self, inbox: crate::messaging::Address) -> Result<Vec<crate::messaging::Pending>, crate::obligation::UnmetObligation> {
        self.behaviors.pending(inbox)
    }

    /// Serves `inbox.messaging.Replies` at `read_your_writes` consistency, from the owed projection.
    pub fn replies(&self, thread: crate::messaging::ThreadId) -> Result<Vec<crate::messaging::Replies>, crate::obligation::UnmetObligation> {
        self.behaviors.replies(thread)
    }

    /// Serves `inbox.messaging.Settled` at `read_your_writes` consistency, from the owed projection.
    pub fn settled(&self, sender: crate::messaging::Principal) -> Result<Vec<crate::messaging::Settled>, crate::obligation::UnmetObligation> {
        self.behaviors.settled(sender)
    }

    /// Serves `inbox.messaging.Thread` at `read_your_writes` consistency, from the owed projection.
    pub fn thread(&self, thread: crate::messaging::ThreadId) -> Result<Vec<crate::messaging::Thread>, crate::obligation::UnmetObligation> {
        self.behaviors.thread(thread)
    }

    /// Serves `inbox.messaging.Unanswered` at `read_your_writes` consistency, from the owed projection.
    pub fn unanswered(&self, inbox: crate::messaging::Address) -> Result<Vec<crate::messaging::Unanswered>, crate::obligation::UnmetObligation> {
        self.behaviors.unanswered(inbox)
    }
}
