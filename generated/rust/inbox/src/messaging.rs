// generated from inbox v1
// contract digest f807145c2ee167ef46048e8e80ca37f3559adea0def6d63fa0d8b3b81c9a7a9d
// do not edit: regenerate with `ess synthesize --layout crate`

//! Messaging — `inbox.messaging`.
//!
//! One addressable inbox per agent or person, and the messages that wait in them. A message is one signed envelope for one inbox. The service keeps it until it is handled or its `expires` passes, then removes it; nothing is kept as history.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// Adapter — `inbox.messaging.Adapter`: one of a closed set of names.
///
/// The wake surface the owner of an inbox reads it through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Adapter {
    /// `claude_code`.
    ClaudeCode,
    /// `metaharness`.
    Metaharness,
    /// `slack`.
    Slack,
    /// `terminal`.
    Terminal,
}

/// Address — `inbox.messaging.Address`: a distinct wrapper around `String`.
///
/// The address of an inbox: `agent:<host>/<session-name>` or `human:<name>`. `group:<name>` is reserved; version 1 opens no group inbox, and a sender that addresses several inboxes posts one envelope to each under one `thread`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address(pub String);

/// AnsweredBy — `inbox.messaging.AnsweredBy`: one of a closed set of names.
///
/// principal: the reply's `on_behalf_of` answered. silence: nobody answered before `expires`, and the service took the decision's `default_on_silence` (the convention of `sdc.decision.PrincipalKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsweredBy {
    /// `principal`.
    Principal,
    /// `silence`.
    Silence,
}

/// Body — `inbox.messaging.Body`: a distinct wrapper around `String`.
///
/// The text of a message or a reply, at most 65536 bytes of UTF-8.
///
/// Every value satisfies `value.utf8_bytes <= 65536` — declared here, enforced by whatever behaviour constructs one.
///
/// Checked by [`Body::broken_constraint`]; the generated decoder refuses a value that breaks one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body(pub String);

impl Body {
    /// The first constraint `inbox.messaging.Body` declares that this value breaks — its alphabet, its prefix,
    /// then each invariant — as what a decoder expected instead, or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value, as the conformance
    /// interpreter reads it. A wrapped newtype's own constraints are its own to check.
    pub fn broken_constraint(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::count(self.0.as_str().len())), iv::Op::Le, iv::Fact::number("65536"), false, true)) {
            return Some("a value satisfying `value.utf8_bytes <= 65536`");
        }
        None
    }
}

/// CollectedReply — `inbox.messaging.CollectedReply`.
///
/// The reply envelope as `Collect` hands it to the sender: the fields of `Reply`, its body as plain text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectedReply {
    /// `reply_id` — `inbox.messaging.MessageId`.
    pub reply_id: MessageId,
    /// `from` — `inbox.messaging.Principal`.
    pub from: Principal,
    /// `on_behalf_of` — `inbox.messaging.Principal`.
    pub on_behalf_of: Principal,
    /// `authority` — `Optional<inbox.messaging.DelegationId>`.
    pub authority: Option<DelegationId>,
    /// `created` — `Timestamp`.
    pub created: crate::primitives::Timestamp,
    /// `option` — `Optional<inbox.messaging.OptionLetter>`.
    pub option: Option<OptionLetter>,
    /// `body` — `String`.
    pub body: String,
    /// `signature` — `inbox.messaging.Signature`.
    pub signature: Signature,
}

/// DecisionAsk — `inbox.messaging.DecisionAsk`.
///
/// What a decision asks. The deadline is the envelope's `expires`; the option silence takes is the envelope's `default_on_silence`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionAsk {
    /// `options` — `List<inbox.messaging.DecisionOption>`.
    pub options: Vec<DecisionOption>,
    /// `recommended_option` — `Optional<inbox.messaging.OptionLetter>`.
    pub recommended_option: Option<OptionLetter>,
    /// `request_ref` — `Optional<String>`.
    pub request_ref: Option<String>,
}

/// DecisionOption — `inbox.messaging.DecisionOption`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionOption {
    /// `letter` — `inbox.messaging.OptionLetter`.
    pub letter: OptionLetter,
    /// `text` — `String`.
    pub text: String,
}

/// DedupKey — `inbox.messaging.DedupKey`: a distinct wrapper around `String`.
///
/// The sender's key for one logical message. A sender with no key of its own writes the message id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DedupKey(pub String);

/// DelegationEntry — `inbox.messaging.DelegationEntry`.
///
/// One delegation of the key file: `agent` may speak for `on_behalf_of` until `valid_until`. It covers every message the agent sends for that principal; version 1 has no narrower scope and no chain. The operator key signs the entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegationEntry {
    /// `delegation_id` — `inbox.messaging.DelegationId`.
    pub delegation_id: DelegationId,
    /// `agent` — `inbox.messaging.Principal`.
    pub agent: Principal,
    /// `on_behalf_of` — `inbox.messaging.Principal`.
    pub on_behalf_of: Principal,
    /// `valid_until` — `Timestamp`.
    pub valid_until: crate::primitives::Timestamp,
    /// `operator_signature` — `String`.
    pub operator_signature: String,
}

/// DelegationId — `inbox.messaging.DelegationId`: a distinct wrapper around `String`.
///
/// The id of a delegation that lets `from` speak for `on_behalf_of`. The SignatureVerifier port resolves it; the inbox stores no delegation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegationId(pub String);

/// FreeShape — `inbox.messaging.FreeShape`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreeShape {
}

/// The states of `inbox.messaging.Inbox`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Inbox<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxState {
    /// `Closed`.
    Closed,
    /// `Open`.
    Open,
}

/// KeyEntry — `inbox.messaging.KeyEntry`.
///
/// One key of the key file the version 1 SignatureVerifier reads: this key signs for this principal until `valid_until`. The operator key signs the entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEntry {
    /// `key_id` — `String`.
    pub key_id: String,
    /// `principal` — `inbox.messaging.Principal`.
    pub principal: Principal,
    /// `public_key` — `String`.
    pub public_key: String,
    /// `valid_until` — `Timestamp`.
    pub valid_until: crate::primitives::Timestamp,
    /// `operator_signature` — `String`.
    pub operator_signature: String,
}

/// KeyFile — `inbox.messaging.KeyFile`.
///
/// The file the version 1 SignatureVerifier reads. The service stores none of it: an entry counts because the operator key signed it, and the operator public key is pinned in the service's configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyFile {
    /// `keys` — `List<inbox.messaging.KeyEntry>`.
    pub keys: Vec<KeyEntry>,
    /// `delegations` — `List<inbox.messaging.DelegationEntry>`.
    pub delegations: Vec<DelegationEntry>,
}

/// Kind — `inbox.messaging.Kind`: one of a closed set of names.
///
/// note: handled when read. request: handled when answered or refused. decision: a request with lettered options, which silence may answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `note`.
    Note,
    /// `request`.
    Request,
    /// `decision`.
    Decision,
}

/// The states of `inbox.messaging.Message`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Message<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageState {
    /// `Answered`.
    Answered,
    /// `Delivered`.
    Delivered,
    /// `Read`.
    Read,
    /// `Refused`.
    Refused,
    /// `Sent`.
    Sent,
}

/// MessageId — `inbox.messaging.MessageId`: a distinct wrapper around `String`.
///
/// A ULID the sender sets. A sender that posts a message again sends the same id.
///
/// Every character is one of `0123456789ABCDEFGHJKMNPQRSTVWXYZ`.
///
/// Every value satisfies `value.count == 26` — declared here, enforced by whatever behaviour constructs one.
///
/// Checked by [`MessageId::broken_constraint`]; the generated decoder refuses a value that breaks one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageId(pub String);

impl MessageId {
    /// The first constraint `inbox.messaging.MessageId` declares that this value breaks — its alphabet, its prefix,
    /// then each invariant — as what a decoder expected instead, or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value, as the conformance
    /// interpreter reads it. A wrapped newtype's own constraints are its own to check.
    pub fn broken_constraint(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if !self.0.chars().all(|c| "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(c)) {
            return Some("a value whose every character is one of `0123456789ABCDEFGHJKMNPQRSTVWXYZ`");
        }
        if iv::broken(iv::compare(Some(iv::Fact::count(self.0.chars().count())), iv::Op::Eq, iv::Fact::number("26"), false, true)) {
            return Some("a value satisfying `value.count == 26`");
        }
        None
    }
}

/// OptionLetter — `inbox.messaging.OptionLetter`: a distinct wrapper around `String`.
///
/// Every character is one of `ABCDEFGHIJKLMNOPQRSTUVWXYZ`.
///
/// Every value satisfies `value.count == 1` — declared here, enforced by whatever behaviour constructs one.
///
/// Checked by [`OptionLetter::broken_constraint`]; the generated decoder refuses a value that breaks one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionLetter(pub String);

impl OptionLetter {
    /// The first constraint `inbox.messaging.OptionLetter` declares that this value breaks — its alphabet, its prefix,
    /// then each invariant — as what a decoder expected instead, or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value, as the conformance
    /// interpreter reads it. A wrapped newtype's own constraints are its own to check.
    pub fn broken_constraint(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if !self.0.chars().all(|c| "ABCDEFGHIJKLMNOPQRSTUVWXYZ".contains(c)) {
            return Some("a value whose every character is one of `ABCDEFGHIJKLMNOPQRSTUVWXYZ`");
        }
        if iv::broken(iv::compare(Some(iv::Fact::count(self.0.chars().count())), iv::Op::Eq, iv::Fact::number("1"), false, true)) {
            return Some("a value satisfying `value.count == 1`");
        }
        None
    }
}

/// Principal — `inbox.messaging.Principal`: a distinct wrapper around `String`.
///
/// Who sends, or whose word a message is: `agent:<host>/<session-name>` or `human:<name>`. The SignatureVerifier port decides which key speaks for which principal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal(pub String);

/// Reply — `inbox.messaging.Reply`.
///
/// The signed reply envelope: the answer to a request or a decision. Its kind is reply, its `in_reply_to` is the message that holds it and its `to` is that message's sender. It stays on the message it answers until the sender collects it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// `reply_id` — `inbox.messaging.MessageId`.
    pub reply_id: MessageId,
    /// `from` — `inbox.messaging.Principal`.
    pub from: Principal,
    /// `on_behalf_of` — `inbox.messaging.Principal`.
    pub on_behalf_of: Principal,
    /// `authority` — `Optional<inbox.messaging.DelegationId>`.
    pub authority: Option<DelegationId>,
    /// `created` — `Timestamp`.
    pub created: crate::primitives::Timestamp,
    /// `option` — `Optional<inbox.messaging.OptionLetter>`.
    pub option: Option<OptionLetter>,
    /// `body` — `inbox.messaging.Body`.
    pub body: Body,
    /// `signature` — `inbox.messaging.Signature`.
    pub signature: Signature,
}

/// ReplyShape — `inbox.messaging.ReplyShape`: one of a fixed set of shapes, tagged on the wire by `shape`.
///
/// What a reply to a request must look like. free: any text. lines: at most this many lines. schema: a reference to a JSON Schema the reply body must satisfy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplyShape {
    /// Tagged `free` — `inbox.messaging.FreeShape`.
    Free(FreeShape),
    /// Tagged `lines` — `Integer`.
    Lines(i64),
    /// Tagged `schema` — `String`.
    Schema(String),
}

/// Signature — `inbox.messaging.Signature`.
///
/// The key id and the Ed25519 signature over the canonical JSON of every other envelope field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    /// `key_id` — `String`.
    pub key_id: String,
    /// `value` — `String`.
    pub value: String,
}

/// ThreadId — `inbox.messaging.ThreadId`: a distinct wrapper around `String`.
///
/// The ULID that ties the envelopes of one fan-out together: a request sent to 11 inboxes is 11 messages with one thread. A message sent to one inbox carries its own id.
///
/// Every character is one of `0123456789ABCDEFGHJKMNPQRSTVWXYZ`.
///
/// Every value satisfies `value.count == 26` — declared here, enforced by whatever behaviour constructs one.
///
/// Checked by [`ThreadId::broken_constraint`]; the generated decoder refuses a value that breaks one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadId(pub String);

impl ThreadId {
    /// The first constraint `inbox.messaging.ThreadId` declares that this value breaks — its alphabet, its prefix,
    /// then each invariant — as what a decoder expected instead, or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value, as the conformance
    /// interpreter reads it. A wrapped newtype's own constraints are its own to check.
    pub fn broken_constraint(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if !self.0.chars().all(|c| "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(c)) {
            return Some("a value whose every character is one of `0123456789ABCDEFGHJKMNPQRSTVWXYZ`");
        }
        if iv::broken(iv::compare(Some(iv::Fact::count(self.0.chars().count())), iv::Op::Eq, iv::Fact::number("26"), false, true)) {
            return Some("a value satisfying `value.count == 26`");
        }
        None
    }
}

/// What Inbox — `inbox.messaging.Inbox` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Inbox<S>`], and at a boundary by [`InboxSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxData {
    /// The identity: `address` — `inbox.messaging.Address`.
    pub address: Address,
    /// `owner` — `inbox.messaging.Principal`.
    pub owner: Principal,
    /// `adapter` — `inbox.messaging.Adapter`.
    pub adapter: Adapter,
}

/// The states of `inbox.messaging.Inbox`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](inbox_state::Marker), so [`Inbox<S>`](Inbox) can only ever rest in a real state.
pub mod inbox_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Closed {}
        impl Sealed for super::Open {}
    }

    /// A declared state of `Inbox`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::InboxState;
    }

    /// `Closed`.
    pub struct Closed;

    impl Marker for Closed {
        const STATE: super::InboxState = super::InboxState::Closed;
    }

    /// `Open`. Where a new instance starts.
    pub struct Open;

    impl Marker for Open {
        const STATE: super::InboxState = super::InboxState::Open;
    }
}

/// Inbox — `inbox.messaging.Inbox` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Open`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`InboxSnapshot`]
/// and [`InboxSnapshot::refine`].
pub struct Inbox<S: inbox_state::Marker> {
    data: InboxData,
    state: core::marker::PhantomData<S>,
}

impl<S: inbox_state::Marker> Inbox<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> InboxState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &InboxData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> InboxData {
        self.data
    }
}

impl Inbox<inbox_state::Open> {
    /// A new instance, resting in `Open` — the only state the lifecycle starts one in.
    pub fn new(data: InboxData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Inbox<inbox_state::Closed> {
    /// `reopen` — `Closed` → `Open`. Taken by the `reopened` outcome of `inbox.messaging.OpenInbox`.
    pub fn reopen(self) -> Inbox<inbox_state::Open> {
        Inbox {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl Inbox<inbox_state::Open> {
    /// `close` — `Open` → `Closed`. Taken by the `closed` outcome of `inbox.messaging.CloseInbox`.
    pub fn close(self) -> Inbox<inbox_state::Closed> {
        Inbox {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `inbox.messaging.Inbox` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`InboxSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: InboxState,
    /// What it holds.
    pub data: InboxData,
}

/// An `Inbox` in whichever declared state it was found.
pub enum AnyInbox {
    /// Resting in `Closed`.
    Closed(Inbox<inbox_state::Closed>),
    /// Resting in `Open`.
    Open(Inbox<inbox_state::Open>),
}

impl InboxSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `InboxState` cannot spell one.
    pub fn refine(self) -> AnyInbox {
        match self.state {
            InboxState::Closed => AnyInbox::Closed(Inbox {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            InboxState::Open => AnyInbox::Open(Inbox {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyInbox {
    /// The state, as the runtime value.
    pub fn state(&self) -> InboxState {
        match self {
            Self::Closed(_) => InboxState::Closed,
            Self::Open(_) => InboxState::Open,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> InboxSnapshot {
        match self {
            Self::Closed(instance) => InboxSnapshot {
                state: InboxState::Closed,
                data: instance.into_data(),
            },
            Self::Open(instance) => InboxSnapshot {
                state: InboxState::Open,
                data: instance.into_data(),
            },
        }
    }
}

/// What Message — `inbox.messaging.Message` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Message<S>`], and at a boundary by [`MessageSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageData {
    /// The identity: `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
    /// `from` — `inbox.messaging.Principal`.
    pub from: Principal,
    /// `on_behalf_of` — `inbox.messaging.Principal`.
    pub on_behalf_of: Principal,
    /// `authority` — `Optional<inbox.messaging.DelegationId>`.
    pub authority: Option<DelegationId>,
    /// `to` — `inbox.messaging.Address`.
    ///
    /// Carries `inbox`: `inbox.messaging.Message` references one `inbox.messaging.Inbox`.
    pub to: Address,
    /// `kind` — `inbox.messaging.Kind`.
    pub kind: Kind,
    /// `thread` — `inbox.messaging.ThreadId`.
    pub thread: ThreadId,
    /// `reply_shape` — `Optional<inbox.messaging.ReplyShape>`.
    pub reply_shape: Option<ReplyShape>,
    /// `decision` — `Optional<inbox.messaging.DecisionAsk>`.
    pub decision: Option<DecisionAsk>,
    /// `default_on_silence` — `Optional<inbox.messaging.OptionLetter>`.
    pub default_on_silence: Option<OptionLetter>,
    /// `created` — `Timestamp`.
    pub created: crate::primitives::Timestamp,
    /// `expires` — `Timestamp`.
    pub expires: crate::primitives::Timestamp,
    /// `dedup_key` — `inbox.messaging.DedupKey`.
    pub dedup_key: DedupKey,
    /// `body` — `inbox.messaging.Body`.
    pub body: Body,
    /// `signature` — `inbox.messaging.Signature`.
    pub signature: Signature,
    /// `answered_by` — `Optional<inbox.messaging.AnsweredBy>`.
    pub answered_by: Option<AnsweredBy>,
    /// `decided_option` — `Optional<inbox.messaging.OptionLetter>`.
    pub decided_option: Option<OptionLetter>,
    /// `reply` — `Optional<inbox.messaging.Reply>`.
    pub reply: Option<Reply>,
    /// `refusal` — `Optional<String>`.
    pub refusal: Option<String>,
}

/// The states of `inbox.messaging.Message`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](message_state::Marker), so [`Message<S>`](Message) can only ever rest in a real state.
pub mod message_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Answered {}
        impl Sealed for super::Delivered {}
        impl Sealed for super::Read {}
        impl Sealed for super::Refused {}
        impl Sealed for super::Sent {}
    }

    /// A declared state of `Message`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::MessageState;
    }

    /// `Answered`. Terminal: an instance may rest here forever.
    pub struct Answered;

    impl Marker for Answered {
        const STATE: super::MessageState = super::MessageState::Answered;
    }

    /// `Delivered`.
    pub struct Delivered;

    impl Marker for Delivered {
        const STATE: super::MessageState = super::MessageState::Delivered;
    }

    /// `Read`.
    pub struct Read;

    impl Marker for Read {
        const STATE: super::MessageState = super::MessageState::Read;
    }

    /// `Refused`. Terminal: an instance may rest here forever.
    pub struct Refused;

    impl Marker for Refused {
        const STATE: super::MessageState = super::MessageState::Refused;
    }

    /// `Sent`. Where a new instance starts.
    pub struct Sent;

    impl Marker for Sent {
        const STATE: super::MessageState = super::MessageState::Sent;
    }
}

/// Message — `inbox.messaging.Message` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Sent`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`MessageSnapshot`]
/// and [`MessageSnapshot::refine`].
pub struct Message<S: message_state::Marker> {
    data: MessageData,
    state: core::marker::PhantomData<S>,
}

impl<S: message_state::Marker> Message<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> MessageState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &MessageData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> MessageData {
        self.data
    }
}

impl Message<message_state::Sent> {
    /// A new instance, resting in `Sent` — the only state the lifecycle starts one in.
    pub fn new(data: MessageData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Message<message_state::Delivered> {
    /// `read` — `Delivered` → `Read`. Taken by the `read` outcome of `inbox.messaging.MarkRead`.
    pub fn read(self) -> Message<message_state::Read> {
        Message {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `answer` — `Delivered` → `Answered`. Taken by the `answered` outcome of `inbox.messaging.Answer`.
    pub fn answer(self) -> Message<message_state::Answered> {
        Message {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `decide_on_silence` — `Delivered` → `Answered`. Taken by the `decided-on-silence` outcome of `inbox.messaging.Expire`.
    pub fn decide_on_silence(self) -> Message<message_state::Answered> {
        Message {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `refuse` — `Delivered` → `Refused`. Taken by the `refused` outcome of `inbox.messaging.Refuse`.
    pub fn refuse(self) -> Message<message_state::Refused> {
        Message {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl Message<message_state::Read> {
    /// `answer` — `Read` → `Answered`. Taken by the `answered` outcome of `inbox.messaging.Answer`.
    pub fn answer(self) -> Message<message_state::Answered> {
        Message {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `decide_on_silence` — `Read` → `Answered`. Taken by the `decided-on-silence` outcome of `inbox.messaging.Expire`.
    pub fn decide_on_silence(self) -> Message<message_state::Answered> {
        Message {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `refuse` — `Read` → `Refused`. Taken by the `refused` outcome of `inbox.messaging.Refuse`.
    pub fn refuse(self) -> Message<message_state::Refused> {
        Message {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl Message<message_state::Sent> {
    /// `deliver` — `Sent` → `Delivered`. Taken by the `delivered` outcome of `inbox.messaging.MarkDelivered`.
    pub fn deliver(self) -> Message<message_state::Delivered> {
        Message {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `read` — `Sent` → `Read`. Taken by the `read` outcome of `inbox.messaging.MarkRead`.
    pub fn read(self) -> Message<message_state::Read> {
        Message {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `answer` — `Sent` → `Answered`. Taken by the `answered` outcome of `inbox.messaging.Answer`.
    pub fn answer(self) -> Message<message_state::Answered> {
        Message {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `decide_on_silence` — `Sent` → `Answered`. Taken by the `decided-on-silence` outcome of `inbox.messaging.Expire`.
    pub fn decide_on_silence(self) -> Message<message_state::Answered> {
        Message {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `refuse` — `Sent` → `Refused`. Taken by the `refused` outcome of `inbox.messaging.Refuse`.
    pub fn refuse(self) -> Message<message_state::Refused> {
        Message {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `inbox.messaging.Message` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`MessageSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: MessageState,
    /// What it holds.
    pub data: MessageData,
}

/// An `Message` in whichever declared state it was found.
pub enum AnyMessage {
    /// Resting in `Answered`.
    Answered(Message<message_state::Answered>),
    /// Resting in `Delivered`.
    Delivered(Message<message_state::Delivered>),
    /// Resting in `Read`.
    Read(Message<message_state::Read>),
    /// Resting in `Refused`.
    Refused(Message<message_state::Refused>),
    /// Resting in `Sent`.
    Sent(Message<message_state::Sent>),
}

impl MessageSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `MessageState` cannot spell one.
    pub fn refine(self) -> AnyMessage {
        match self.state {
            MessageState::Answered => AnyMessage::Answered(Message {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            MessageState::Delivered => AnyMessage::Delivered(Message {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            MessageState::Read => AnyMessage::Read(Message {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            MessageState::Refused => AnyMessage::Refused(Message {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            MessageState::Sent => AnyMessage::Sent(Message {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyMessage {
    /// The state, as the runtime value.
    pub fn state(&self) -> MessageState {
        match self {
            Self::Answered(_) => MessageState::Answered,
            Self::Delivered(_) => MessageState::Delivered,
            Self::Read(_) => MessageState::Read,
            Self::Refused(_) => MessageState::Refused,
            Self::Sent(_) => MessageState::Sent,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> MessageSnapshot {
        match self {
            Self::Answered(instance) => MessageSnapshot {
                state: MessageState::Answered,
                data: instance.into_data(),
            },
            Self::Delivered(instance) => MessageSnapshot {
                state: MessageState::Delivered,
                data: instance.into_data(),
            },
            Self::Read(instance) => MessageSnapshot {
                state: MessageState::Read,
                data: instance.into_data(),
            },
            Self::Refused(instance) => MessageSnapshot {
                state: MessageState::Refused,
                data: instance.into_data(),
            },
            Self::Sent(instance) => MessageSnapshot {
                state: MessageState::Sent,
                data: instance.into_data(),
            },
        }
    }
}

/// Answer — the input of `inbox.messaging.Answer`.
///
/// Answer a request or a decision. The input is the reply envelope: its kind is reply and its `to` is the sender of the message it answers. The service checks it as it checks a post.
///
/// Everything it can result in is [`AnswerOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// `in_reply_to` — `inbox.messaging.MessageId`.
    pub in_reply_to: MessageId,
    /// `reply_id` — `inbox.messaging.MessageId`.
    pub reply_id: MessageId,
    /// `from` — `inbox.messaging.Principal`.
    pub from: Principal,
    /// `on_behalf_of` — `inbox.messaging.Principal`.
    pub on_behalf_of: Principal,
    /// `authority` — `Optional<inbox.messaging.DelegationId>`.
    pub authority: Option<DelegationId>,
    /// `created` — `Timestamp`.
    pub created: crate::primitives::Timestamp,
    /// `expires` — `Timestamp`.
    pub expires: crate::primitives::Timestamp,
    /// `option` — `Optional<inbox.messaging.OptionLetter>`.
    pub option: Option<OptionLetter>,
    /// `body` — `inbox.messaging.Body`.
    pub body: Body,
    /// `signature` — `inbox.messaging.Signature`.
    pub signature: Signature,
}

/// Everything `inbox.messaging.Answer` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerOutcome {
    /// `not-sender` — when `from != caller.principal`.
    NotSender {
        /// Why it was refused: `inbox.messaging.NotSender`.
        error: NotSender,
    },
    /// `no-authority` — when `(on_behalf_of != {fact: from} and not (defined(authority)))`.
    NoAuthority {
        /// Why it was refused: `inbox.messaging.NoAuthority`.
        error: NoAuthority,
    },
    /// `already-expired` — when `expires <= {fact: created} as timestamp`.
    AlreadyExpired {
        /// Why it was refused: `inbox.messaging.AlreadyExpired`.
        error: AlreadyExpired,
    },
    /// `signature-invalid` — externally decided (the SignatureVerifier port does not know the key, finds it lapsed, or finds that the signature does not verify).
    SignatureInvalid {
        /// Why it was refused: `inbox.messaging.SignatureInvalid`.
        error: SignatureInvalid,
    },
    /// `authority-invalid` — externally decided (the SignatureVerifier port finds no live delegation with this id that lets `from` speak for `on_behalf_of`).
    AuthorityInvalid {
        /// Why it was refused: `inbox.messaging.NoAuthority`.
        error: NoAuthority,
    },
    /// `unknown-message` — for an identity no record carries.
    UnknownMessage {
        /// Why it was refused: `inbox.messaging.UnknownMessage`.
        error: UnknownMessage,
    },
    /// `foreign-message` — when the existing subject's stored fields satisfy `(to != caller.inbox and from != caller.principal)`.
    ///
    /// The caller is neither the recipient nor the sender of the message. The service answers it as `unknown-message`, so from this command a third principal cannot tell a message between others from no message.
    ForeignMessage {
        /// Why it was refused: `inbox.messaging.UnknownMessage`.
        error: UnknownMessage,
    },
    /// `not-recipient` — when the existing subject's stored fields satisfy `to != caller.inbox`.
    NotRecipient {
        /// Why it was refused: `inbox.messaging.NotRecipient`.
        error: NotRecipient,
    },
    /// `already-settled` — from a state no declared move starts in.
    AlreadySettled {
        /// Why it was refused: `inbox.messaging.AlreadySettled`.
        error: AlreadySettled,
    },
    /// `note` — when the existing subject's stored fields satisfy `kind == note`.
    Note {
        /// Why it was refused: `inbox.messaging.NotAnswerable`.
        error: NotAnswerable,
    },
    /// `deadline-passed` — externally decided (the service's Clock port reads at or after the message's stored `expires`) when the existing subject's stored fields satisfy `state in [Sent, Delivered, Read]`.
    ///
    /// The deadline has passed: the expiry task decides the message, by silence with its `default_on_silence` or by removal.
    DeadlinePassed {
        /// Why it was refused: `inbox.messaging.Expired`.
        error: Expired,
    },
    /// `expired-on-arrival` — externally decided (the service's Clock port reads at or after the reply's own `expires`) when the existing subject's stored fields satisfy `state in [Sent, Delivered, Read]`.
    ///
    /// The reply's own `expires` has passed by the service's clock: stored, it would be removed before its sender could collect it. The message stays as it was.
    ExpiredOnArrival {
        /// Why it was refused: `inbox.messaging.AlreadyExpired`.
        error: AlreadyExpired,
    },
    /// `shape-mismatch` — externally decided (the reply does not fit the message's `reply_shape`, or names no option of the decision).
    ShapeMismatch {
        /// Why it was refused: `inbox.messaging.ShapeMismatch`.
        error: ShapeMismatch,
    },
    /// `answered` — otherwise.
    Answered {
        /// The `inbox.messaging.MessageAnswered` this outcome publishes.
        message_answered: MessageAnswered,
    },
}

/// CloseInbox — the input of `inbox.messaging.CloseInbox`.
///
/// Close an inbox. Posts to it are refused; the messages it holds stay until they expire.
///
/// Everything it can result in is [`CloseInboxOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseInbox {
    /// `address` — `inbox.messaging.Address`.
    pub address: Address,
}

/// Everything `inbox.messaging.CloseInbox` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseInboxOutcome {
    /// `unknown-inbox` — for an identity no record carries.
    UnknownInbox {
        /// Why it was refused: `inbox.messaging.UnknownInbox`.
        error: UnknownInbox,
    },
    /// `not-owner` — when the existing subject's stored fields satisfy `owner != caller.principal`.
    NotOwner {
        /// Why it was refused: `inbox.messaging.NotRecipient`.
        error: NotRecipient,
    },
    /// `closed` — when the existing subject is in Open.
    Closed {
        /// The `inbox.messaging.InboxClosed` this outcome publishes.
        inbox_closed: InboxClosed,
    },
    /// `already-closed` — otherwise.
    AlreadyClosed,
}

/// Collect — the input of `inbox.messaging.Collect`.
///
/// The sender takes the answer or the refusal of its message. The message is handled by that and is removed, and the answer carries the settled record as the message held it when it was removed: the members of its `Outbox` row, among them the envelope, its state, who answered, the decided option, the reply with the `expires` its signature covers, and the refusal.
///
/// Everything it can result in is [`CollectOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collect {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
}

/// Actual typed response of `inbox.messaging.Collect`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectResponse {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
    /// `from` — `inbox.messaging.Principal`.
    pub from: Principal,
    /// `on_behalf_of` — `inbox.messaging.Principal`.
    pub on_behalf_of: Principal,
    /// `authority` — `Optional<inbox.messaging.DelegationId>`.
    pub authority: Option<DelegationId>,
    /// `to` — `inbox.messaging.Address`.
    pub to: Address,
    /// `kind` — `inbox.messaging.Kind`.
    pub kind: Kind,
    /// `thread` — `inbox.messaging.ThreadId`.
    pub thread: ThreadId,
    /// `reply_shape` — `Optional<inbox.messaging.ReplyShape>`.
    pub reply_shape: Option<ReplyShape>,
    /// `decision` — `Optional<inbox.messaging.DecisionAsk>`.
    pub decision: Option<DecisionAsk>,
    /// `default_on_silence` — `Optional<inbox.messaging.OptionLetter>`.
    pub default_on_silence: Option<OptionLetter>,
    /// `created` — `Timestamp`.
    pub created: crate::primitives::Timestamp,
    /// `expires` — `Timestamp`.
    pub expires: crate::primitives::Timestamp,
    /// `dedup_key` — `inbox.messaging.DedupKey`.
    pub dedup_key: DedupKey,
    /// `body` — `String`.
    pub body: String,
    /// `signature` — `inbox.messaging.Signature`.
    pub signature: Signature,
    /// `answered_by` — `Optional<inbox.messaging.AnsweredBy>`.
    pub answered_by: Option<AnsweredBy>,
    /// `decided_option` — `Optional<inbox.messaging.OptionLetter>`.
    pub decided_option: Option<OptionLetter>,
    /// `reply` — `Optional<inbox.messaging.CollectedReply>`.
    pub reply: Option<CollectedReply>,
    /// `refusal` — `Optional<String>`.
    pub refusal: Option<String>,
    /// `state` — `inbox.messaging.Message.State`.
    pub state: MessageState,
}

/// Everything `inbox.messaging.Collect` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectOutcome {
    /// `unknown-message` — for an identity no record carries.
    UnknownMessage {
        /// Why it was refused: `inbox.messaging.UnknownMessage`.
        error: UnknownMessage,
    },
    /// `foreign-message` — when the existing subject's stored fields satisfy `(to != caller.inbox and from != caller.principal)`.
    ///
    /// The caller is neither the recipient nor the sender of the message. The service answers it as `unknown-message`, so from this command a third principal cannot tell a message between others from no message.
    ForeignMessage {
        /// Why it was refused: `inbox.messaging.UnknownMessage`.
        error: UnknownMessage,
    },
    /// `not-sender` — when the existing subject's stored fields satisfy `from != caller.principal`.
    NotSender {
        /// Why it was refused: `inbox.messaging.NotSender`.
        error: NotSender,
    },
    /// `collected` — when the existing subject is in Answered or Refused.
    Collected {
        /// Actual response returned by this branch.
        response: CollectResponse,
        /// The `inbox.messaging.MessageHandled` this outcome publishes.
        message_handled: MessageHandled,
    },
    /// `not-settled` — otherwise.
    NotSettled {
        /// Why it was refused: `inbox.messaging.NotSettled`.
        error: NotSettled,
    },
}

/// Expire — the input of `inbox.messaging.Expire`.
///
/// The service's expiry task removes one message whose `expires` has passed. `now` is the Clock port's reading. A decision that names a `default_on_silence` is answered by silence instead and kept until `keep_until`.
///
/// Everything it can result in is [`ExpireOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expire {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
    /// `now` — `Timestamp`.
    pub now: crate::primitives::Timestamp,
    /// `keep_until` — `Timestamp`.
    pub keep_until: crate::primitives::Timestamp,
}

/// Everything `inbox.messaging.Expire` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpireOutcome {
    /// `unknown-message` — for an identity no record carries.
    UnknownMessage {
        /// Why it was refused: `inbox.messaging.UnknownMessage`.
        error: UnknownMessage,
    },
    /// `not-due` — when the existing subject's stored fields satisfy `expires > input.now as timestamp`.
    NotDue {
        /// Why it was refused: `inbox.messaging.NotDue`.
        error: NotDue,
    },
    /// `decided-on-silence` — when the existing subject's stored fields satisfy `(kind == decision and state in [Sent, Delivered, Read] and defined(default_on_silence))`.
    DecidedOnSilence {
        /// The `inbox.messaging.DecisionTakenOnSilence` this outcome publishes.
        decision_taken_on_silence: DecisionTakenOnSilence,
    },
    /// `expired` — otherwise.
    Expired {
        /// The `inbox.messaging.MessageExpired` this outcome publishes.
        message_expired: MessageExpired,
    },
}

/// MarkDelivered — the input of `inbox.messaging.MarkDelivered`.
///
/// An adapter reports that a live wake surface took the message. It is sent only after the surface took it.
///
/// Everything it can result in is [`MarkDeliveredOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkDelivered {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
}

/// Everything `inbox.messaging.MarkDelivered` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkDeliveredOutcome {
    /// `unknown-message` — for an identity no record carries.
    UnknownMessage {
        /// Why it was refused: `inbox.messaging.UnknownMessage`.
        error: UnknownMessage,
    },
    /// `foreign-message` — when the existing subject's stored fields satisfy `(to != caller.inbox and from != caller.principal)`.
    ///
    /// The caller is neither the recipient nor the sender of the message. The service answers it as `unknown-message`, so from this command a third principal cannot tell a message between others from no message.
    ForeignMessage {
        /// Why it was refused: `inbox.messaging.UnknownMessage`.
        error: UnknownMessage,
    },
    /// `not-recipient` — when the existing subject's stored fields satisfy `to != caller.inbox`.
    NotRecipient {
        /// Why it was refused: `inbox.messaging.NotRecipient`.
        error: NotRecipient,
    },
    /// `deadline-passed` — externally decided (the service's Clock port reads at or after the message's stored `expires`) when the existing subject's stored fields satisfy `state in [Sent, Delivered, Read]`.
    ///
    /// The deadline has passed: the message stays as it is until the expiry task decides it.
    DeadlinePassed {
        /// Why it was refused: `inbox.messaging.Expired`.
        error: Expired,
    },
    /// `delivered` — when the existing subject is in Sent.
    Delivered {
        /// The `inbox.messaging.MessageDelivered` this outcome publishes.
        message_delivered: MessageDelivered,
    },
    /// `already-delivered` — otherwise.
    AlreadyDelivered,
}

/// MarkRead — the input of `inbox.messaging.MarkRead`.
///
/// The owner of the inbox read the message. A note is handled by that and is removed; a request or a decision waits for its answer.
///
/// Everything it can result in is [`MarkReadOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkRead {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
}

/// Everything `inbox.messaging.MarkRead` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkReadOutcome {
    /// `unknown-message` — for an identity no record carries.
    UnknownMessage {
        /// Why it was refused: `inbox.messaging.UnknownMessage`.
        error: UnknownMessage,
    },
    /// `foreign-message` — when the existing subject's stored fields satisfy `(to != caller.inbox and from != caller.principal)`.
    ///
    /// The caller is neither the recipient nor the sender of the message. The service answers it as `unknown-message`, so from this command a third principal cannot tell a message between others from no message.
    ForeignMessage {
        /// Why it was refused: `inbox.messaging.UnknownMessage`.
        error: UnknownMessage,
    },
    /// `not-recipient` — when the existing subject's stored fields satisfy `to != caller.inbox`.
    NotRecipient {
        /// Why it was refused: `inbox.messaging.NotRecipient`.
        error: NotRecipient,
    },
    /// `deadline-passed` — externally decided (the service's Clock port reads at or after the message's stored `expires`) when the existing subject's stored fields satisfy `state in [Sent, Delivered, Read]`.
    ///
    /// The deadline has passed: the message stays as it is until the expiry task decides it.
    DeadlinePassed {
        /// Why it was refused: `inbox.messaging.Expired`.
        error: Expired,
    },
    /// `handled` — when the existing subject's stored fields satisfy `(kind == note and state in [Sent, Delivered])`.
    ///
    /// The note is read, so it is handled and gone.
    Handled {
        /// The `inbox.messaging.MessageHandled` this outcome publishes.
        message_handled: MessageHandled,
    },
    /// `read` — when the existing subject is in Delivered or Sent.
    Read {
        /// The `inbox.messaging.MessageRead` this outcome publishes.
        message_read: MessageRead,
    },
    /// `already-read` — otherwise.
    AlreadyRead,
}

/// OpenInbox — the input of `inbox.messaging.OpenInbox`.
///
/// Open the inbox at an address, or open it again. The caller becomes its owner. A client sends this after every reconnect, because a restarted service holds no inbox.
///
/// Everything it can result in is [`OpenInboxOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenInbox {
    /// `address` — `inbox.messaging.Address`.
    pub address: Address,
    /// `adapter` — `inbox.messaging.Adapter`.
    pub adapter: Adapter,
}

/// Everything `inbox.messaging.OpenInbox` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenInboxOutcome {
    /// `not-owner` — when `address != caller.inbox`.
    NotOwner {
        /// Why it was refused: `inbox.messaging.NotRecipient`.
        error: NotRecipient,
    },
    /// `opened` — for an identity no record carries.
    Opened {
        /// The `inbox.messaging.InboxOpened` this outcome publishes.
        inbox_opened: InboxOpened,
    },
    /// `reopened` — when the existing subject is in Closed.
    Reopened {
        /// The `inbox.messaging.InboxOpened` this outcome publishes.
        inbox_opened: InboxOpened,
    },
    /// `already-open` — otherwise.
    AlreadyOpen,
}

/// PostMessage — the input of `inbox.messaging.PostMessage`.
///
/// Post one signed envelope to one inbox. The input is the envelope. The service checks the signature and the authority before it stores anything; a refused post stores nothing and the refusal is the sender's answer.
///
/// Everything it can result in is [`PostMessageOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostMessage {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
    /// `from` — `inbox.messaging.Principal`.
    pub from: Principal,
    /// `on_behalf_of` — `inbox.messaging.Principal`.
    pub on_behalf_of: Principal,
    /// `authority` — `Optional<inbox.messaging.DelegationId>`.
    pub authority: Option<DelegationId>,
    /// `to` — `inbox.messaging.Address`.
    pub to: Address,
    /// `kind` — `inbox.messaging.Kind`.
    pub kind: Kind,
    /// `thread` — `inbox.messaging.ThreadId`.
    pub thread: ThreadId,
    /// `reply_shape` — `Optional<inbox.messaging.ReplyShape>`.
    pub reply_shape: Option<ReplyShape>,
    /// `decision` — `Optional<inbox.messaging.DecisionAsk>`.
    pub decision: Option<DecisionAsk>,
    /// `default_on_silence` — `Optional<inbox.messaging.OptionLetter>`.
    pub default_on_silence: Option<OptionLetter>,
    /// `created` — `Timestamp`.
    pub created: crate::primitives::Timestamp,
    /// `expires` — `Timestamp`.
    pub expires: crate::primitives::Timestamp,
    /// `dedup_key` — `inbox.messaging.DedupKey`.
    pub dedup_key: DedupKey,
    /// `body` — `inbox.messaging.Body`.
    pub body: Body,
    /// `signature` — `inbox.messaging.Signature`.
    pub signature: Signature,
}

/// Everything `inbox.messaging.PostMessage` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostMessageOutcome {
    /// `not-sender` — when `from != caller.principal`.
    NotSender {
        /// Why it was refused: `inbox.messaging.NotSender`.
        error: NotSender,
    },
    /// `no-authority` — when `(on_behalf_of != {fact: from} and not (defined(authority)))`.
    ///
    /// The sender speaks for another principal and names no delegation.
    NoAuthority {
        /// Why it was refused: `inbox.messaging.NoAuthority`.
        error: NoAuthority,
    },
    /// `already-expired` — when `expires <= {fact: created} as timestamp`.
    AlreadyExpired {
        /// Why it was refused: `inbox.messaging.AlreadyExpired`.
        error: AlreadyExpired,
    },
    /// `request-without-shape` — when `(kind == request and not (defined(reply_shape)))`.
    RequestWithoutShape {
        /// Why it was refused: `inbox.messaging.ShapeMismatch`.
        error: ShapeMismatch,
    },
    /// `decision-without-options` — when `(kind == decision and (not (defined(decision)) or decision.options.count == 0))`.
    ///
    /// A decision names no `decision`, or a `decision` with no option.
    DecisionWithoutOptions {
        /// Why it was refused: `inbox.messaging.ShapeMismatch`.
        error: ShapeMismatch,
    },
    /// `unknown-option` — when `((defined(default_on_silence) and not (defined(decision))) or (defined(decision) and (not (distinct o in decision.options by o.letter as string) or (defined(decision.recommended_option) and not (exists o in decision.options: (o.letter == decision.recommended_option))) or (defined(default_on_silence) and not (exists o in decision.options: (o.letter == {fact: default_on_silence}))))))`.
    ///
    /// `recommended_option` or `default_on_silence` names no option of the decision (an envelope without a decision offers none), or two options carry one letter.
    UnknownOption {
        /// Why it was refused: `inbox.messaging.ShapeMismatch`.
        error: ShapeMismatch,
    },
    /// `expired-on-arrival` — externally decided (the service's Clock port reads at or after the post's `expires`).
    ExpiredOnArrival {
        /// Why it was refused: `inbox.messaging.AlreadyExpired`.
        error: AlreadyExpired,
    },
    /// `signature-invalid` — externally decided (the SignatureVerifier port does not know the key, finds it lapsed, or finds that the signature does not verify).
    SignatureInvalid {
        /// Why it was refused: `inbox.messaging.SignatureInvalid`.
        error: SignatureInvalid,
    },
    /// `authority-invalid` — externally decided (the SignatureVerifier port finds no live delegation with this id that lets `from` speak for `on_behalf_of`).
    AuthorityInvalid {
        /// Why it was refused: `inbox.messaging.NoAuthority`.
        error: NoAuthority,
    },
    /// `duplicate` — when, of the `inbox.messaging.Message` rows satisfying `(message_id == input.message_id and from == input.from)`, at least one row is selected.
    ///
    /// A live message of this sender carries this id.
    Duplicate {
        /// Why it was refused: `inbox.messaging.Duplicate`.
        error: Duplicate,
    },
    /// `duplicate-key` — when, of the `inbox.messaging.Message` rows satisfying `(dedup_key == input.dedup_key and from == input.from)`, at least one row is selected.
    DuplicateKey {
        /// Why it was refused: `inbox.messaging.Duplicate`.
        error: Duplicate,
    },
    /// `unknown-inbox` — when no `inbox.messaging.Inbox` carries the identity `input.to` names.
    UnknownInbox {
        /// Why it was refused: `inbox.messaging.UnknownInbox`.
        error: UnknownInbox,
    },
    /// `inbox-closed` — when the `inbox.messaging.Inbox` that `input.to` names satisfies `state == Closed`.
    InboxClosed {
        /// Why it was refused: `inbox.messaging.UnknownInbox`.
        error: UnknownInbox,
    },
    /// `id-taken` — when, of the `inbox.messaging.Message` rows satisfying `message_id == input.message_id`, at least one row is selected.
    ///
    /// A live message of another sender carries this id.
    IdTaken {
        /// Why it was refused: `inbox.messaging.MessageIdTaken`.
        error: MessageIdTaken,
    },
    /// `posted` — otherwise.
    Posted {
        /// The `inbox.messaging.MessagePosted` this outcome publishes.
        message_posted: MessagePosted,
    },
}

/// Refuse — the input of `inbox.messaging.Refuse`.
///
/// The owner of the inbox refuses the message and says why. The sender collects the reason.
///
/// Everything it can result in is [`RefuseOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refuse {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
    /// `reason` — `String`.
    pub reason: String,
}

/// Everything `inbox.messaging.Refuse` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefuseOutcome {
    /// `unknown-message` — for an identity no record carries.
    UnknownMessage {
        /// Why it was refused: `inbox.messaging.UnknownMessage`.
        error: UnknownMessage,
    },
    /// `foreign-message` — when the existing subject's stored fields satisfy `(to != caller.inbox and from != caller.principal)`.
    ///
    /// The caller is neither the recipient nor the sender of the message. The service answers it as `unknown-message`, so from this command a third principal cannot tell a message between others from no message.
    ForeignMessage {
        /// Why it was refused: `inbox.messaging.UnknownMessage`.
        error: UnknownMessage,
    },
    /// `not-recipient` — when the existing subject's stored fields satisfy `to != caller.inbox`.
    NotRecipient {
        /// Why it was refused: `inbox.messaging.NotRecipient`.
        error: NotRecipient,
    },
    /// `note` — when the existing subject's stored fields satisfy `kind == note`.
    ///
    /// A note asks for nothing, so it is not refused either; it is removed by `MarkRead` or `Expire`.
    Note {
        /// Why it was refused: `inbox.messaging.NotAnswerable`.
        error: NotAnswerable,
    },
    /// `deadline-passed` — externally decided (the service's Clock port reads at or after the message's stored `expires`) when the existing subject's stored fields satisfy `state in [Sent, Delivered, Read]`.
    ///
    /// The deadline has passed: the expiry task decides the message, by silence with its `default_on_silence` or by removal.
    DeadlinePassed {
        /// Why it was refused: `inbox.messaging.Expired`.
        error: Expired,
    },
    /// `refused` — otherwise.
    Refused {
        /// The `inbox.messaging.MessageRefused` this outcome publishes.
        message_refused: MessageRefused,
    },
    /// `already-settled` — from a state no declared move starts in.
    AlreadySettled {
        /// Why it was refused: `inbox.messaging.AlreadySettled`.
        error: AlreadySettled,
    },
}

/// DecisionTakenOnSilence — the event `inbox.messaging.DecisionTakenOnSilence`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionTakenOnSilence {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
    /// `to` — `inbox.messaging.Principal`.
    pub to: Principal,
    /// `thread` — `inbox.messaging.ThreadId`.
    pub thread: ThreadId,
}

/// InboxClosed — the event `inbox.messaging.InboxClosed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxClosed {
    /// `address` — `inbox.messaging.Address`.
    pub address: Address,
}

/// InboxOpened — the event `inbox.messaging.InboxOpened`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxOpened {
    /// `address` — `inbox.messaging.Address`.
    pub address: Address,
}

/// MessageAnswered — the event `inbox.messaging.MessageAnswered`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageAnswered {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
    /// `reply_id` — `inbox.messaging.MessageId`.
    pub reply_id: MessageId,
    /// `to` — `inbox.messaging.Principal`.
    pub to: Principal,
    /// `thread` — `inbox.messaging.ThreadId`.
    pub thread: ThreadId,
}

/// MessageDelivered — the event `inbox.messaging.MessageDelivered`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageDelivered {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
}

/// MessageExpired — the event `inbox.messaging.MessageExpired`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageExpired {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
    /// `to` — `inbox.messaging.Principal`.
    pub to: Principal,
    /// `thread` — `inbox.messaging.ThreadId`.
    pub thread: ThreadId,
}

/// MessageHandled — the event `inbox.messaging.MessageHandled`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageHandled {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
}

/// MessagePosted — the event `inbox.messaging.MessagePosted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessagePosted {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
    /// `to` — `inbox.messaging.Address`.
    pub to: Address,
    /// `from` — `inbox.messaging.Principal`.
    pub from: Principal,
    /// `thread` — `inbox.messaging.ThreadId`.
    pub thread: ThreadId,
    /// `kind` — `inbox.messaging.Kind`.
    pub kind: Kind,
}

/// MessageRead — the event `inbox.messaging.MessageRead`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageRead {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
}

/// MessageRefused — the event `inbox.messaging.MessageRefused`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageRefused {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
    /// `to` — `inbox.messaging.Principal`.
    pub to: Principal,
    /// `thread` — `inbox.messaging.ThreadId`.
    pub thread: ThreadId,
}

/// The declared error `inbox.messaging.AlreadyExpired`.
///
/// `expires` is not after `created`, or the `expires` of a post or of a reply is not after the service's clock reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlreadyExpired;

/// The declared error `inbox.messaging.AlreadySettled`.
///
/// The message is already answered or refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlreadySettled {
    /// `state` — `inbox.messaging.Message.State`.
    pub state: MessageState,
}

/// The declared error `inbox.messaging.Duplicate`.
///
/// A live message of this sender already carries this id or this dedup key; nothing was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Duplicate;

/// The declared error `inbox.messaging.Expired`.
///
/// The message's `expires` is at or before the service's clock reading: it takes no answer, no refusal, no read mark and no delivery mark, and the expiry task decides it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expired;

/// The declared error `inbox.messaging.MessageIdTaken`.
///
/// A live message of another sender carries this id; nothing was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageIdTaken;

/// The declared error `inbox.messaging.NoAuthority`.
///
/// `on_behalf_of` differs from `from` and no valid delegation covers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoAuthority;

/// The declared error `inbox.messaging.NotAnswerable`.
///
/// A note takes no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotAnswerable;

/// The declared error `inbox.messaging.NotDue`.
///
/// `expires` is after the clock reading sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotDue;

/// The declared error `inbox.messaging.NotRecipient`.
///
/// The caller does not own the inbox the message was sent to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotRecipient;

/// The declared error `inbox.messaging.NotSender`.
///
/// The caller did not send the message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotSender;

/// The declared error `inbox.messaging.NotSettled`.
///
/// The message has no answer and no refusal to collect yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotSettled;

/// The declared error `inbox.messaging.ShapeMismatch`.
///
/// The envelope does not fit its kind, or the reply does not fit what the message asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeMismatch;

/// The declared error `inbox.messaging.SignatureInvalid`.
///
/// The SignatureVerifier port refused the signature: the key is unknown or lapsed, or the signature does not verify.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureInvalid;

/// The declared error `inbox.messaging.UnknownInbox`.
///
/// No open inbox has this address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownInbox;

/// The declared error `inbox.messaging.UnknownMessage`.
///
/// No live message has this id: it was never posted, it was handled, or it expired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownMessage;

/// Due — one row of the view `inbox.messaging.Due`.
///
/// Projects `inbox.messaging.Message` at `read_your_writes` consistency, containing instances where `expires <= param.now as timestamp`.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Due {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
    /// `from` — `inbox.messaging.Principal`.
    pub from: Principal,
    /// `on_behalf_of` — `inbox.messaging.Principal`.
    pub on_behalf_of: Principal,
    /// `authority` — `Optional<inbox.messaging.DelegationId>`.
    pub authority: Option<DelegationId>,
    /// `to` — `inbox.messaging.Address`.
    pub to: Address,
    /// `kind` — `inbox.messaging.Kind`.
    pub kind: Kind,
    /// `thread` — `inbox.messaging.ThreadId`.
    pub thread: ThreadId,
    /// `reply_shape` — `Optional<inbox.messaging.ReplyShape>`.
    pub reply_shape: Option<ReplyShape>,
    /// `decision` — `Optional<inbox.messaging.DecisionAsk>`.
    pub decision: Option<DecisionAsk>,
    /// `default_on_silence` — `Optional<inbox.messaging.OptionLetter>`.
    pub default_on_silence: Option<OptionLetter>,
    /// `created` — `Timestamp`.
    pub created: crate::primitives::Timestamp,
    /// `expires` — `Timestamp`.
    pub expires: crate::primitives::Timestamp,
    /// `dedup_key` — `inbox.messaging.DedupKey`.
    pub dedup_key: DedupKey,
    /// `body` — `inbox.messaging.Body`.
    pub body: Body,
    /// `signature` — `inbox.messaging.Signature`.
    pub signature: Signature,
    /// `answered_by` — `Optional<inbox.messaging.AnsweredBy>`.
    pub answered_by: Option<AnsweredBy>,
    /// `decided_option` — `Optional<inbox.messaging.OptionLetter>`.
    pub decided_option: Option<OptionLetter>,
    /// `reply` — `Optional<inbox.messaging.Reply>`.
    pub reply: Option<Reply>,
    /// `refusal` — `Optional<String>`.
    pub refusal: Option<String>,
    /// `state` — `inbox.messaging.Message.State`.
    pub state: MessageState,
}

/// Inboxes — one row of the view `inbox.messaging.Inboxes`.
///
/// Projects `inbox.messaging.Inbox` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inboxes {
    /// `address` — `inbox.messaging.Address`.
    pub address: Address,
    /// `owner` — `inbox.messaging.Principal`.
    pub owner: Principal,
    /// `adapter` — `inbox.messaging.Adapter`.
    pub adapter: Adapter,
    /// `state` — `inbox.messaging.Inbox.State`.
    pub state: InboxState,
}

/// Messages — one row of the view `inbox.messaging.Messages`.
///
/// Projects `inbox.messaging.Message` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Messages {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
    /// `from` — `inbox.messaging.Principal`.
    pub from: Principal,
    /// `on_behalf_of` — `inbox.messaging.Principal`.
    pub on_behalf_of: Principal,
    /// `authority` — `Optional<inbox.messaging.DelegationId>`.
    pub authority: Option<DelegationId>,
    /// `to` — `inbox.messaging.Address`.
    pub to: Address,
    /// `kind` — `inbox.messaging.Kind`.
    pub kind: Kind,
    /// `thread` — `inbox.messaging.ThreadId`.
    pub thread: ThreadId,
    /// `reply_shape` — `Optional<inbox.messaging.ReplyShape>`.
    pub reply_shape: Option<ReplyShape>,
    /// `decision` — `Optional<inbox.messaging.DecisionAsk>`.
    pub decision: Option<DecisionAsk>,
    /// `default_on_silence` — `Optional<inbox.messaging.OptionLetter>`.
    pub default_on_silence: Option<OptionLetter>,
    /// `created` — `Timestamp`.
    pub created: crate::primitives::Timestamp,
    /// `expires` — `Timestamp`.
    pub expires: crate::primitives::Timestamp,
    /// `dedup_key` — `inbox.messaging.DedupKey`.
    pub dedup_key: DedupKey,
    /// `body` — `inbox.messaging.Body`.
    pub body: Body,
    /// `signature` — `inbox.messaging.Signature`.
    pub signature: Signature,
    /// `answered_by` — `Optional<inbox.messaging.AnsweredBy>`.
    pub answered_by: Option<AnsweredBy>,
    /// `decided_option` — `Optional<inbox.messaging.OptionLetter>`.
    pub decided_option: Option<OptionLetter>,
    /// `reply` — `Optional<inbox.messaging.Reply>`.
    pub reply: Option<Reply>,
    /// `refusal` — `Optional<String>`.
    pub refusal: Option<String>,
    /// `state` — `inbox.messaging.Message.State`.
    pub state: MessageState,
}

/// Outbox — one row of the view `inbox.messaging.Outbox`.
///
/// Projects `inbox.messaging.Message` at `read_your_writes` consistency, containing instances where `from == param.sender`.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outbox {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
    /// `from` — `inbox.messaging.Principal`.
    pub from: Principal,
    /// `on_behalf_of` — `inbox.messaging.Principal`.
    pub on_behalf_of: Principal,
    /// `authority` — `Optional<inbox.messaging.DelegationId>`.
    pub authority: Option<DelegationId>,
    /// `to` — `inbox.messaging.Address`.
    pub to: Address,
    /// `kind` — `inbox.messaging.Kind`.
    pub kind: Kind,
    /// `thread` — `inbox.messaging.ThreadId`.
    pub thread: ThreadId,
    /// `reply_shape` — `Optional<inbox.messaging.ReplyShape>`.
    pub reply_shape: Option<ReplyShape>,
    /// `decision` — `Optional<inbox.messaging.DecisionAsk>`.
    pub decision: Option<DecisionAsk>,
    /// `default_on_silence` — `Optional<inbox.messaging.OptionLetter>`.
    pub default_on_silence: Option<OptionLetter>,
    /// `created` — `Timestamp`.
    pub created: crate::primitives::Timestamp,
    /// `expires` — `Timestamp`.
    pub expires: crate::primitives::Timestamp,
    /// `dedup_key` — `inbox.messaging.DedupKey`.
    pub dedup_key: DedupKey,
    /// `body` — `inbox.messaging.Body`.
    pub body: Body,
    /// `signature` — `inbox.messaging.Signature`.
    pub signature: Signature,
    /// `answered_by` — `Optional<inbox.messaging.AnsweredBy>`.
    pub answered_by: Option<AnsweredBy>,
    /// `decided_option` — `Optional<inbox.messaging.OptionLetter>`.
    pub decided_option: Option<OptionLetter>,
    /// `reply` — `Optional<inbox.messaging.Reply>`.
    pub reply: Option<Reply>,
    /// `refusal` — `Optional<String>`.
    pub refusal: Option<String>,
    /// `state` — `inbox.messaging.Message.State`.
    pub state: MessageState,
}

/// Pending — one row of the view `inbox.messaging.Pending`.
///
/// Projects `inbox.messaging.Message` at `read_your_writes` consistency, containing instances where `(to == param.inbox and (state == Sent or state == Delivered))`.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
    /// `from` — `inbox.messaging.Principal`.
    pub from: Principal,
    /// `on_behalf_of` — `inbox.messaging.Principal`.
    pub on_behalf_of: Principal,
    /// `authority` — `Optional<inbox.messaging.DelegationId>`.
    pub authority: Option<DelegationId>,
    /// `to` — `inbox.messaging.Address`.
    pub to: Address,
    /// `kind` — `inbox.messaging.Kind`.
    pub kind: Kind,
    /// `thread` — `inbox.messaging.ThreadId`.
    pub thread: ThreadId,
    /// `reply_shape` — `Optional<inbox.messaging.ReplyShape>`.
    pub reply_shape: Option<ReplyShape>,
    /// `decision` — `Optional<inbox.messaging.DecisionAsk>`.
    pub decision: Option<DecisionAsk>,
    /// `default_on_silence` — `Optional<inbox.messaging.OptionLetter>`.
    pub default_on_silence: Option<OptionLetter>,
    /// `created` — `Timestamp`.
    pub created: crate::primitives::Timestamp,
    /// `expires` — `Timestamp`.
    pub expires: crate::primitives::Timestamp,
    /// `dedup_key` — `inbox.messaging.DedupKey`.
    pub dedup_key: DedupKey,
    /// `body` — `inbox.messaging.Body`.
    pub body: Body,
    /// `signature` — `inbox.messaging.Signature`.
    pub signature: Signature,
    /// `state` — `inbox.messaging.Message.State`.
    pub state: MessageState,
}

/// Replies — one row of the view `inbox.messaging.Replies`.
///
/// Projects `inbox.messaging.Message` at `read_your_writes` consistency, containing instances where `(thread == param.thread and state == Answered)`.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replies {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
    /// `from` — `inbox.messaging.Principal`.
    pub from: Principal,
    /// `on_behalf_of` — `inbox.messaging.Principal`.
    pub on_behalf_of: Principal,
    /// `authority` — `Optional<inbox.messaging.DelegationId>`.
    pub authority: Option<DelegationId>,
    /// `to` — `inbox.messaging.Address`.
    pub to: Address,
    /// `kind` — `inbox.messaging.Kind`.
    pub kind: Kind,
    /// `thread` — `inbox.messaging.ThreadId`.
    pub thread: ThreadId,
    /// `reply_shape` — `Optional<inbox.messaging.ReplyShape>`.
    pub reply_shape: Option<ReplyShape>,
    /// `decision` — `Optional<inbox.messaging.DecisionAsk>`.
    pub decision: Option<DecisionAsk>,
    /// `default_on_silence` — `Optional<inbox.messaging.OptionLetter>`.
    pub default_on_silence: Option<OptionLetter>,
    /// `created` — `Timestamp`.
    pub created: crate::primitives::Timestamp,
    /// `expires` — `Timestamp`.
    pub expires: crate::primitives::Timestamp,
    /// `dedup_key` — `inbox.messaging.DedupKey`.
    pub dedup_key: DedupKey,
    /// `body` — `inbox.messaging.Body`.
    pub body: Body,
    /// `signature` — `inbox.messaging.Signature`.
    pub signature: Signature,
    /// `answered_by` — `Optional<inbox.messaging.AnsweredBy>`.
    pub answered_by: Option<AnsweredBy>,
    /// `decided_option` — `Optional<inbox.messaging.OptionLetter>`.
    pub decided_option: Option<OptionLetter>,
    /// `reply` — `Optional<inbox.messaging.Reply>`.
    pub reply: Option<Reply>,
    /// `state` — `inbox.messaging.Message.State`.
    pub state: MessageState,
}

/// Settled — one row of the view `inbox.messaging.Settled`.
///
/// Projects `inbox.messaging.Message` at `read_your_writes` consistency, containing instances where `(from == param.sender and (state == Answered or state == Refused))`.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settled {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
    /// `from` — `inbox.messaging.Principal`.
    pub from: Principal,
    /// `on_behalf_of` — `inbox.messaging.Principal`.
    pub on_behalf_of: Principal,
    /// `authority` — `Optional<inbox.messaging.DelegationId>`.
    pub authority: Option<DelegationId>,
    /// `to` — `inbox.messaging.Address`.
    pub to: Address,
    /// `kind` — `inbox.messaging.Kind`.
    pub kind: Kind,
    /// `thread` — `inbox.messaging.ThreadId`.
    pub thread: ThreadId,
    /// `reply_shape` — `Optional<inbox.messaging.ReplyShape>`.
    pub reply_shape: Option<ReplyShape>,
    /// `decision` — `Optional<inbox.messaging.DecisionAsk>`.
    pub decision: Option<DecisionAsk>,
    /// `default_on_silence` — `Optional<inbox.messaging.OptionLetter>`.
    pub default_on_silence: Option<OptionLetter>,
    /// `created` — `Timestamp`.
    pub created: crate::primitives::Timestamp,
    /// `expires` — `Timestamp`.
    pub expires: crate::primitives::Timestamp,
    /// `dedup_key` — `inbox.messaging.DedupKey`.
    pub dedup_key: DedupKey,
    /// `body` — `inbox.messaging.Body`.
    pub body: Body,
    /// `signature` — `inbox.messaging.Signature`.
    pub signature: Signature,
    /// `answered_by` — `Optional<inbox.messaging.AnsweredBy>`.
    pub answered_by: Option<AnsweredBy>,
    /// `decided_option` — `Optional<inbox.messaging.OptionLetter>`.
    pub decided_option: Option<OptionLetter>,
    /// `reply` — `Optional<inbox.messaging.Reply>`.
    pub reply: Option<Reply>,
    /// `refusal` — `Optional<String>`.
    pub refusal: Option<String>,
    /// `state` — `inbox.messaging.Message.State`.
    pub state: MessageState,
}

/// Thread — one row of the view `inbox.messaging.Thread`.
///
/// Projects `inbox.messaging.Message` at `read_your_writes` consistency, containing instances where `thread == param.thread`.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thread {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
    /// `from` — `inbox.messaging.Principal`.
    pub from: Principal,
    /// `on_behalf_of` — `inbox.messaging.Principal`.
    pub on_behalf_of: Principal,
    /// `authority` — `Optional<inbox.messaging.DelegationId>`.
    pub authority: Option<DelegationId>,
    /// `to` — `inbox.messaging.Address`.
    pub to: Address,
    /// `kind` — `inbox.messaging.Kind`.
    pub kind: Kind,
    /// `thread` — `inbox.messaging.ThreadId`.
    pub thread: ThreadId,
    /// `reply_shape` — `Optional<inbox.messaging.ReplyShape>`.
    pub reply_shape: Option<ReplyShape>,
    /// `decision` — `Optional<inbox.messaging.DecisionAsk>`.
    pub decision: Option<DecisionAsk>,
    /// `default_on_silence` — `Optional<inbox.messaging.OptionLetter>`.
    pub default_on_silence: Option<OptionLetter>,
    /// `created` — `Timestamp`.
    pub created: crate::primitives::Timestamp,
    /// `expires` — `Timestamp`.
    pub expires: crate::primitives::Timestamp,
    /// `dedup_key` — `inbox.messaging.DedupKey`.
    pub dedup_key: DedupKey,
    /// `body` — `inbox.messaging.Body`.
    pub body: Body,
    /// `signature` — `inbox.messaging.Signature`.
    pub signature: Signature,
    /// `answered_by` — `Optional<inbox.messaging.AnsweredBy>`.
    pub answered_by: Option<AnsweredBy>,
    /// `decided_option` — `Optional<inbox.messaging.OptionLetter>`.
    pub decided_option: Option<OptionLetter>,
    /// `reply` — `Optional<inbox.messaging.Reply>`.
    pub reply: Option<Reply>,
    /// `refusal` — `Optional<String>`.
    pub refusal: Option<String>,
    /// `state` — `inbox.messaging.Message.State`.
    pub state: MessageState,
}

/// Unanswered — one row of the view `inbox.messaging.Unanswered`.
///
/// Projects `inbox.messaging.Message` at `read_your_writes` consistency, containing instances where `(to == param.inbox and state == Read)`.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unanswered {
    /// `message_id` — `inbox.messaging.MessageId`.
    pub message_id: MessageId,
    /// `from` — `inbox.messaging.Principal`.
    pub from: Principal,
    /// `on_behalf_of` — `inbox.messaging.Principal`.
    pub on_behalf_of: Principal,
    /// `authority` — `Optional<inbox.messaging.DelegationId>`.
    pub authority: Option<DelegationId>,
    /// `to` — `inbox.messaging.Address`.
    pub to: Address,
    /// `kind` — `inbox.messaging.Kind`.
    pub kind: Kind,
    /// `thread` — `inbox.messaging.ThreadId`.
    pub thread: ThreadId,
    /// `reply_shape` — `Optional<inbox.messaging.ReplyShape>`.
    pub reply_shape: Option<ReplyShape>,
    /// `decision` — `Optional<inbox.messaging.DecisionAsk>`.
    pub decision: Option<DecisionAsk>,
    /// `default_on_silence` — `Optional<inbox.messaging.OptionLetter>`.
    pub default_on_silence: Option<OptionLetter>,
    /// `created` — `Timestamp`.
    pub created: crate::primitives::Timestamp,
    /// `expires` — `Timestamp`.
    pub expires: crate::primitives::Timestamp,
    /// `dedup_key` — `inbox.messaging.DedupKey`.
    pub dedup_key: DedupKey,
    /// `body` — `inbox.messaging.Body`.
    pub body: Body,
    /// `signature` — `inbox.messaging.Signature`.
    pub signature: Signature,
    /// `state` — `inbox.messaging.Message.State`.
    pub state: MessageState,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The behaviour `inbox.messaging.Answer` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait AnswerBehavior {
        /// Decides and enacts exactly one declared outcome of `inbox.messaging.Answer`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn answer(&mut self, input: super::Answer) -> Result<super::AnswerOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `inbox.messaging.CloseInbox` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait CloseInboxBehavior {
        /// Decides and enacts exactly one declared outcome of `inbox.messaging.CloseInbox`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn close_inbox(&mut self, input: super::CloseInbox) -> Result<super::CloseInboxOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `inbox.messaging.Collect` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a typed response (`response:`).
    ///
    /// Contract: given `inbox.messaging.Collect` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `unknown-message` for an identity no record carries, error `inbox.messaging.UnknownMessage`; `foreign-message` when the existing subject's stored fields satisfy `(to != caller.inbox and from != caller.principal)`, error `inbox.messaging.UnknownMessage`; `not-sender` when the existing subject's stored fields satisfy `from != caller.principal`, error `inbox.messaging.NotSender`; `collected` when the existing subject is in Answered or Refused, deletes `inbox.messaging.Message`, emits `inbox.messaging.MessageHandled`; `not-settled` otherwise, error `inbox.messaging.NotSettled`.
    pub trait CollectBehavior {
        /// Decides and enacts exactly one declared outcome of `inbox.messaging.Collect`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn collect(&mut self, input: super::Collect) -> Result<super::CollectOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `inbox.messaging.Expire` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a narrowed read of the subject's Optional `default_on_silence`, which its branch proves present, for `decided_option`, in `decided-on-silence`.
    ///
    /// Contract: given `inbox.messaging.Expire` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `unknown-message` for an identity no record carries, error `inbox.messaging.UnknownMessage`; `not-due` when the existing subject's stored fields satisfy `expires > input.now as timestamp`, error `inbox.messaging.NotDue`; `decided-on-silence` when the existing subject's stored fields satisfy `(kind == decision and state in [Sent, Delivered, Read] and defined(default_on_silence))`, takes `decide_on_silence` of `inbox.messaging.Message`, emits `inbox.messaging.DecisionTakenOnSilence`; `expired` otherwise, removes every `inbox.messaging.Message` where `message_id == input.message_id`, emits `inbox.messaging.MessageExpired`.
    pub trait ExpireBehavior {
        /// Decides and enacts exactly one declared outcome of `inbox.messaging.Expire`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn expire(&mut self, input: super::Expire) -> Result<super::ExpireOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `inbox.messaging.MarkDelivered` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait MarkDeliveredBehavior {
        /// Decides and enacts exactly one declared outcome of `inbox.messaging.MarkDelivered`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn mark_delivered(&mut self, input: super::MarkDelivered) -> Result<super::MarkDeliveredOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `inbox.messaging.MarkRead` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait MarkReadBehavior {
        /// Decides and enacts exactly one declared outcome of `inbox.messaging.MarkRead`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn mark_read(&mut self, input: super::MarkRead) -> Result<super::MarkReadOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `inbox.messaging.OpenInbox` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait OpenInboxBehavior {
        /// Decides and enacts exactly one declared outcome of `inbox.messaging.OpenInbox`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn open_inbox(&mut self, input: super::OpenInbox) -> Result<super::OpenInboxOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `inbox.messaging.PostMessage` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by `distinct o in decision.options by o.letter as string`, distinct list members no generated behaviour compares, in `unknown-option`.
    ///
    /// Contract: given `inbox.messaging.PostMessage` input, decide and enact exactly one outcome. Selection precedence: on commands with `when_related:`, check `existing_instance` then `exists: false` before input-guarded refusals; choose the first declared input refusal whose guard holds; then check addressed-row existence (`unknown_instance`, and `existing_instance` on commands without `when_related:`); then the held state (`when_subject_state` and `when_subject`), with `wrong_state` only if the selected branch moves from a state the row does not hold; then choose the present `when_related:` predicate refusal whose predicate and optional input guard hold; then accepting and external branches in declaration order. An accepting branch that moves nothing answers in every state. Related-presence predicates do not precede input-guarded refusals. Declared outcomes (declaration order, not selection precedence): `not-sender` when `from != caller.principal`, error `inbox.messaging.NotSender`; `no-authority` when `(on_behalf_of != {fact: from} and not (defined(authority)))`, error `inbox.messaging.NoAuthority`; `already-expired` when `expires <= {fact: created} as timestamp`, error `inbox.messaging.AlreadyExpired`; `request-without-shape` when `(kind == request and not (defined(reply_shape)))`, error `inbox.messaging.ShapeMismatch`; `decision-without-options` when `(kind == decision and (not (defined(decision)) or decision.options.count == 0))`, error `inbox.messaging.ShapeMismatch`; `unknown-option` when `((defined(default_on_silence) and not (defined(decision))) or (defined(decision) and (not (distinct o in decision.options by o.letter as string) or (defined(decision.recommended_option) and not (exists o in decision.options: (o.letter == decision.recommended_option))) or (defined(default_on_silence) and not (exists o in decision.options: (o.letter == {fact: default_on_silence}))))))`, error `inbox.messaging.ShapeMismatch`; `expired-on-arrival` externally decided (the service's Clock port reads at or after the post's `expires`), error `inbox.messaging.AlreadyExpired`; `signature-invalid` externally decided (the SignatureVerifier port does not know the key, finds it lapsed, or finds that the signature does not verify), error `inbox.messaging.SignatureInvalid`; `authority-invalid` externally decided (the SignatureVerifier port finds no live delegation with this id that lets `from` speak for `on_behalf_of`), error `inbox.messaging.NoAuthority`; `duplicate` when, of the `inbox.messaging.Message` rows satisfying `(message_id == input.message_id and from == input.from)`, at least one row is selected, error `inbox.messaging.Duplicate`; `duplicate-key` when, of the `inbox.messaging.Message` rows satisfying `(dedup_key == input.dedup_key and from == input.from)`, at least one row is selected, error `inbox.messaging.Duplicate`; `unknown-inbox` when no `inbox.messaging.Inbox` carries the identity `input.to` names, error `inbox.messaging.UnknownInbox`; `inbox-closed` when the `inbox.messaging.Inbox` that `input.to` names satisfies `state == Closed`, error `inbox.messaging.UnknownInbox`; `id-taken` when, of the `inbox.messaging.Message` rows satisfying `message_id == input.message_id`, at least one row is selected, error `inbox.messaging.MessageIdTaken`; `posted` otherwise, creates `inbox.messaging.Message`, emits `inbox.messaging.MessagePosted`.
    pub trait PostMessageBehavior {
        /// Decides and enacts exactly one declared outcome of `inbox.messaging.PostMessage`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn post_message(&mut self, input: super::PostMessage) -> Result<super::PostMessageOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `inbox.messaging.Refuse` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RefuseBehavior {
        /// Decides and enacts exactly one declared outcome of `inbox.messaging.Refuse`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn refuse(&mut self, input: super::Refuse) -> Result<super::RefuseOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `inbox.messaging.Due` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait DueQuery {
        /// Serves `inbox.messaging.Due` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn due(&self, now: crate::primitives::Timestamp) -> Result<Vec<super::Due>, crate::obligation::UnmetObligation>;
    }

    /// The query `inbox.messaging.Inboxes` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait InboxesQuery {
        /// Serves `inbox.messaging.Inboxes` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn inboxes(&self) -> Result<Vec<super::Inboxes>, crate::obligation::UnmetObligation>;
    }

    /// The query `inbox.messaging.Messages` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait MessagesQuery {
        /// Serves `inbox.messaging.Messages` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn messages(&self) -> Result<Vec<super::Messages>, crate::obligation::UnmetObligation>;
    }

    /// The query `inbox.messaging.Outbox` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait OutboxQuery {
        /// Serves `inbox.messaging.Outbox` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn outbox(&self, sender: crate::messaging::Principal) -> Result<Vec<super::Outbox>, crate::obligation::UnmetObligation>;
    }

    /// The query `inbox.messaging.Pending` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait PendingQuery {
        /// Serves `inbox.messaging.Pending` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn pending(&self, inbox: crate::messaging::Address) -> Result<Vec<super::Pending>, crate::obligation::UnmetObligation>;
    }

    /// The query `inbox.messaging.Replies` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait RepliesQuery {
        /// Serves `inbox.messaging.Replies` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn replies(&self, thread: crate::messaging::ThreadId) -> Result<Vec<super::Replies>, crate::obligation::UnmetObligation>;
    }

    /// The query `inbox.messaging.Settled` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait SettledQuery {
        /// Serves `inbox.messaging.Settled` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn settled(&self, sender: crate::messaging::Principal) -> Result<Vec<super::Settled>, crate::obligation::UnmetObligation>;
    }

    /// The query `inbox.messaging.Thread` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait ThreadQuery {
        /// Serves `inbox.messaging.Thread` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn thread(&self, thread: crate::messaging::ThreadId) -> Result<Vec<super::Thread>, crate::obligation::UnmetObligation>;
    }

    /// The query `inbox.messaging.Unanswered` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait UnansweredQuery {
        /// Serves `inbox.messaging.Unanswered` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn unanswered(&self, inbox: crate::messaging::Address) -> Result<Vec<super::Unanswered>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl CollectBehavior for Unimplemented {
        fn collect(&mut self, _input: super::Collect) -> Result<super::CollectOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "inbox.messaging.Collect" })
        }
    }

    impl ExpireBehavior for Unimplemented {
        fn expire(&mut self, _input: super::Expire) -> Result<super::ExpireOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "inbox.messaging.Expire" })
        }
    }

    impl PostMessageBehavior for Unimplemented {
        fn post_message(&mut self, _input: super::PostMessage) -> Result<super::PostMessageOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "inbox.messaging.PostMessage" })
        }
    }
}
