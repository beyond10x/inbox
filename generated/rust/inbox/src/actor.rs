// generated from inbox v1
// contract digest b2752e02fb814efe015dd697c394e9b35780af59421c2491cff5e6943b6603b5
// do not edit: regenerate with `ess synthesize --layout crate`

//! Every actor the specification declares, and the commands each may invoke — as data.
//!
//! A grant is checked against a caller identity, which these types do not read from anywhere:
//! whatever authenticates a request builds a [`Caller`], and a served surface checks it against
//! [`may`] before the command runs. The `PLAN.md` beside this workspace says, per actor,
//! whether a generated surface enforces the grant or the caller does.

/// An actor the specification declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Actor {
    /// `inbox.messaging.Client`.
    Client,
    /// `inbox.messaging.Sweeper`.
    Sweeper,
}

impl Actor {
    /// Every declared actor, ordered by qualified name.
    pub const ALL: &'static [Actor] = &[
        Actor::Client,
        Actor::Sweeper,
    ];

    /// The actor's qualified name, as the specification spells it.
    pub const fn name(self) -> &'static str {
        match self {
            Actor::Client => "inbox.messaging.Client",
            Actor::Sweeper => "inbox.messaging.Sweeper",
        }
    }
}

/// The qualified names of the commands `actor` may invoke, ordered by name; empty for an
/// actor that only observes.
pub fn may(actor: Actor) -> &'static [&'static str] {
    match actor {
        Actor::Client => &[
            "inbox.messaging.Answer",
            "inbox.messaging.CloseInbox",
            "inbox.messaging.Collect",
            "inbox.messaging.MarkDelivered",
            "inbox.messaging.MarkRead",
            "inbox.messaging.OpenInbox",
            "inbox.messaging.PostMessage",
            "inbox.messaging.Refuse",
        ],
        Actor::Sweeper => &[
            "inbox.messaging.Expire",
        ],
    }
}

/// Who a request was authenticated as.
///
/// Built by whatever authenticates the request — a session, a token, a certificate — and
/// handed to the served surface's `dispatch` and `handle`, which check its grant
/// before the command runs. Never derived from the request itself: a client can write
/// anything into a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Caller {
    /// The declared actor.
    pub actor: Actor,
}

impl Caller {
    /// `true` when this caller may invoke `command`, named by its qualified name.
    pub fn may(&self, command: &str) -> bool {
        may(self.actor).contains(&command)
    }
}

/// The qualified names of the views `actor` may read, ordered by name: the views its grant
/// names. A view no actor's grant names is open to every caller and listed for none.
pub fn may_read(actor: Actor) -> &'static [&'static str] {
    match actor {
        Actor::Client => &[
            "inbox.messaging.Inboxes",
            "inbox.messaging.Outbox",
            "inbox.messaging.Pending",
            "inbox.messaging.Replies",
            "inbox.messaging.Settled",
            "inbox.messaging.Thread",
            "inbox.messaging.Unanswered",
        ],
        Actor::Sweeper => &[
            "inbox.messaging.Due",
            "inbox.messaging.Messages",
        ],
    }
}

impl Caller {
    /// `true` when this caller may read `view`, a view some actor's grant names, named by
    /// its qualified name.
    pub fn may_read(&self, view: &str) -> bool {
        may_read(self.actor).contains(&view)
    }
}
