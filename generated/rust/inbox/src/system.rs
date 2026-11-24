// generated from inbox v1
// contract digest e34e82a0a81f7c01ba3d91fa287c7a723f89e6d1fbdd928b462676dc857fd14a
// do not edit: regenerate with `ess synthesize --layout crate`

//! The `inbox` system, v1: its components assembled, its bindings wired, and its one transport.
//!
//! The transport is derived from the specification, not chosen: `at_least_once` is the only
//! delivery guarantee the model declares, so published events land on an append-only log and a
//! pump delivers each to every binding that reacts to it. The log is the system's observable
//! record, and so is the record of what each binding invoked. What no specification determines
//! — how an escalation event is filled, behaviour behind the ports — stays an obligation; see
//! the `PLAN.md` beside this workspace.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// An event on the system's log: everything any component publishes, and everything a binding
/// escalates into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemEvent {
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

impl SystemEvent {
    /// The qualified name the specification declares this event under.
    pub fn name(&self) -> &'static str {
        match self {
            Self::DecisionTakenOnSilence(_) => "inbox.messaging.DecisionTakenOnSilence",
            Self::InboxClosed(_) => "inbox.messaging.InboxClosed",
            Self::InboxOpened(_) => "inbox.messaging.InboxOpened",
            Self::MessageAnswered(_) => "inbox.messaging.MessageAnswered",
            Self::MessageDelivered(_) => "inbox.messaging.MessageDelivered",
            Self::MessageExpired(_) => "inbox.messaging.MessageExpired",
            Self::MessageHandled(_) => "inbox.messaging.MessageHandled",
            Self::MessagePosted(_) => "inbox.messaging.MessagePosted",
            Self::MessageRead(_) => "inbox.messaging.MessageRead",
            Self::MessageRefused(_) => "inbox.messaging.MessageRefused",
        }
    }
}

impl From<crate::ports::inbox_service::PublishedEvent> for SystemEvent {
    fn from(event: crate::ports::inbox_service::PublishedEvent) -> Self {
        match event {
            crate::ports::inbox_service::PublishedEvent::DecisionTakenOnSilence(event) => Self::DecisionTakenOnSilence(event),
            crate::ports::inbox_service::PublishedEvent::InboxClosed(event) => Self::InboxClosed(event),
            crate::ports::inbox_service::PublishedEvent::InboxOpened(event) => Self::InboxOpened(event),
            crate::ports::inbox_service::PublishedEvent::MessageAnswered(event) => Self::MessageAnswered(event),
            crate::ports::inbox_service::PublishedEvent::MessageDelivered(event) => Self::MessageDelivered(event),
            crate::ports::inbox_service::PublishedEvent::MessageExpired(event) => Self::MessageExpired(event),
            crate::ports::inbox_service::PublishedEvent::MessageHandled(event) => Self::MessageHandled(event),
            crate::ports::inbox_service::PublishedEvent::MessagePosted(event) => Self::MessagePosted(event),
            crate::ports::inbox_service::PublishedEvent::MessageRead(event) => Self::MessageRead(event),
            crate::ports::inbox_service::PublishedEvent::MessageRefused(event) => Self::MessageRefused(event),
        }
    }
}

/// The `inbox` system: every component behind its port, and the transport between them.
///
/// The component fields are public because commands enter the system through a component's own
/// port; the log and its delivery cursor are not, because publishing happens by pumping, not by
/// writing history directly.
pub struct System<InboxServiceBehaviors> {
    /// The `inbox-service` component.
    pub inbox_service: crate::ports::inbox_service::InboxService<InboxServiceBehaviors>,
    published: Vec<SystemEvent>,
    cursor: usize,
}

impl<InboxServiceBehaviors> System<InboxServiceBehaviors> {
    /// Assembles the system from its components.
    pub fn new(inbox_service: crate::ports::inbox_service::InboxService<InboxServiceBehaviors>) -> Self {
        Self {
            inbox_service,
            published: Vec::new(),
            cursor: 0,
        }
    }

    /// Everything published so far, in publication order — the system's observable record.
    pub fn published(&self) -> &[SystemEvent] {
        &self.published
    }

    /// Takes every event the pump has already delivered off the log, in publication order.
    ///
    /// A long-running shell calls this after each `pump`, or the log holds every event the
    /// process ever published. A `pump` returns with every logged event delivered: each
    /// reacting binding has had its attempt, and a binding whose attempt stopped holds the event in
    /// its own held-back list, not on the log. Events published since the last `pump` stay on the
    /// log, so the next `pump` still delivers them; taking never skips a binding.
    pub fn take_published(&mut self) -> Vec<SystemEvent> {
        let delivered: Vec<SystemEvent> = self.published.drain(..self.cursor).collect();
        self.cursor = 0;
        delivered
    }
}

impl<InboxServiceBehaviors> System<InboxServiceBehaviors>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    /// Delivers until quiescent: collects every component's outbox onto the log. No binding
    /// reacts to anything this specification publishes, so collecting is the whole delivery.
    pub fn pump(&mut self) -> Result<(), crate::obligation::UnmetObligation> {
        loop {
            self.collect();
            if self.cursor == self.published.len() {
                return Ok(());
            }
            self.cursor += 1;
        }
    }

    /// Moves every component's outbox onto the log, in component order.
    fn collect(&mut self) {
        for event in self.inbox_service.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
    }
}
