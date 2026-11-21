//! The 2 obligations of `PLAN.md`: the behaviours of `PostMessage` and `Expire`.

use inbox::behaviour::{ExternalCommand, InboxStorage, MessageStorage, TryContext};
use inbox::messaging::obligations::{ExpireBehavior, PostMessageBehavior};
use inbox::messaging::{
    Address, AlreadyExpired, AnsweredBy, AnyMessage, DecisionTakenOnSilence, Duplicate, Expire,
    ExpireOutcome, InboxSnapshot, InboxState, Kind, Message, MessageData, MessageExpired,
    MessageId, MessagePosted, MessageSnapshot, MessageState, NoAuthority, NotDue, PostMessage,
    PostMessageOutcome, Principal, ShapeMismatch, SignatureInvalid, UnknownInbox, UnknownMessage,
};
use inbox::obligation::UnmetObligation;
use inbox::primitives::Timestamp;
use inbox::primitives::invariant::{self, Fact, Op};

use crate::context::PostExternals;

/// The ports [`inbox::behaviour::Generated`] runs over: a backend `B`, a context `C`, and the
/// behaviours of `PostMessage` and `Expire`, which read and write through `B`.
///
/// `B` implements both storage ports. `C` implements the generated context port and
/// [`PostExternals`]: [`crate::ServiceContext`] in the service, a harness's own type where a test
/// forces a branch.
#[derive(Debug, Clone)]
pub struct Core<B, C> {
    /// The storage ports.
    pub backend: B,
    /// The context port and the answers to the `external:` branches of `PostMessage`.
    pub context: C,
}

impl<B, C> Core<B, C> {
    /// The ports over `backend` and `context`.
    pub fn new(backend: B, context: C) -> Self {
        Self { backend, context }
    }
}

impl<B: InboxStorage, C> InboxStorage for Core<B, C> {
    fn get(&self, identity: &Address) -> Option<InboxSnapshot> {
        self.backend.get(identity)
    }

    fn put(&mut self, snapshot: InboxSnapshot) {
        self.backend.put(snapshot);
    }

    fn delete(&mut self, identity: &Address) {
        self.backend.delete(identity);
    }

    fn list(&self) -> Vec<InboxSnapshot> {
        self.backend.list()
    }
}

impl<B: MessageStorage, C> MessageStorage for Core<B, C> {
    fn get(&self, identity: &MessageId) -> Option<MessageSnapshot> {
        self.backend.get(identity)
    }

    fn put(&mut self, snapshot: MessageSnapshot) {
        self.backend.put(snapshot);
    }

    fn delete(&mut self, identity: &MessageId) {
        self.backend.delete(identity);
    }

    fn list(&self) -> Vec<MessageSnapshot> {
        self.backend.list()
    }
}

impl<B, C: TryContext> TryContext for Core<B, C> {
    fn try_caller_inbox(&self) -> Result<Option<Address>, UnmetObligation> {
        self.context.try_caller_inbox()
    }

    fn try_caller_principal(&self) -> Result<Option<Principal>, UnmetObligation> {
        self.context.try_caller_principal()
    }

    fn try_external(
        &mut self,
        command: ExternalCommand<'_>,
        outcome: &'static str,
    ) -> Result<bool, UnmetObligation> {
        self.context.try_external(command, outcome)
    }
}

/// The typed refusal the generated behaviours answer where the model declares no outcome: a guard
/// that reads an instant that names none is Unknown and selects no branch.
fn undeclared(source: &'static str) -> UnmetObligation {
    UnmetObligation {
        capability: "command behaviour",
        source,
    }
}

/// A guard over two instants, read as the generated invariants and the conformance interpreter
/// read it.
fn instants(
    left: &Timestamp,
    op: Op,
    right: &Timestamp,
    source: &'static str,
) -> Result<bool, UnmetObligation> {
    invariant::compare(
        Some(Fact::text(&left.0)),
        op,
        Some(Fact::text(&right.0)),
        true,
        false,
    )
    .ok_or_else(|| undeclared(source))
}

const POST_MESSAGE: &str = "inbox.messaging.PostMessage";
const EXPIRE: &str = "inbox.messaging.Expire";

/// `inbox.messaging.PostMessage`, in the order the ESS 0.96.0 interpreter and synthesizer select
/// its outcomes:
///
/// 1. the input-guarded refusals in declaration order: `no-authority`, `already-expired`,
///    `request-without-shape`, `decision-without-options`;
/// 2. the external branches in declaration order, each asked of [`PostExternals`] before any row
///    is read (`before_row: true` in `ess/domains/messaging.yaml`): `unknown-option`,
///    `signature-invalid`, `authority-invalid`;
/// 3. the row refusals: `duplicate` (`existing_instance`), `unknown-inbox` (`exists: false`),
///    `duplicate-key` (over `list`), `inbox-closed`;
/// 4. `posted`.
///
/// Where the contract text in `PLAN.md` differs (it puts `duplicate` and `unknown-inbox` before
/// the input-guarded refusals and the external branches after every row refusal), this follows
/// the interpreter, which the conformance suite runs against. The order inside step 3 is
/// measured on the interpreted target too, pair by pair: `duplicate` before `unknown-inbox`,
/// `unknown-inbox` before `duplicate-key`, `duplicate-key` before `inbox-closed`, `duplicate`
/// before `inbox-closed` (`tests/adversary.rs` holds each pair).
///
/// Read-then-write, like every generated behaviour: two posts over one store at once are told
/// apart only where the backend's [`crate::Exclusive`] section runs them one at a time, which
/// [`crate::handle`] does.
impl<B: InboxStorage + MessageStorage, C: PostExternals> PostMessageBehavior for Core<B, C> {
    fn post_message(&mut self, input: PostMessage) -> Result<PostMessageOutcome, UnmetObligation> {
        if input.on_behalf_of != input.from && input.authority.is_none() {
            return Ok(PostMessageOutcome::NoAuthority { error: NoAuthority });
        }
        if instants(&input.expires, Op::Le, &input.created, POST_MESSAGE)? {
            return Ok(PostMessageOutcome::AlreadyExpired {
                error: AlreadyExpired,
            });
        }
        if input.kind == Kind::Request && input.reply_shape.is_none() {
            return Ok(PostMessageOutcome::RequestWithoutShape {
                error: ShapeMismatch,
            });
        }
        if input.kind == Kind::Decision && input.decision.is_none() {
            return Ok(PostMessageOutcome::DecisionWithoutOptions {
                error: ShapeMismatch,
            });
        }

        if self.context.unknown_option(&input) {
            return Ok(PostMessageOutcome::UnknownOption {
                error: ShapeMismatch,
            });
        }
        if self.context.signature_invalid(&input) {
            return Ok(PostMessageOutcome::SignatureInvalid {
                error: SignatureInvalid,
            });
        }
        if self.context.authority_invalid(&input) {
            return Ok(PostMessageOutcome::AuthorityInvalid { error: NoAuthority });
        }

        if MessageStorage::get(&self.backend, &input.message_id).is_some() {
            return Ok(PostMessageOutcome::Duplicate { error: Duplicate });
        }
        let Some(inbox) = InboxStorage::get(&self.backend, &input.to) else {
            return Ok(PostMessageOutcome::UnknownInbox {
                error: UnknownInbox,
            });
        };
        if MessageStorage::list(&self.backend)
            .iter()
            .any(|row| row.data.dedup_key == input.dedup_key)
        {
            return Ok(PostMessageOutcome::DuplicateKey { error: Duplicate });
        }
        if inbox.state == InboxState::Closed {
            return Ok(PostMessageOutcome::InboxClosed {
                error: UnknownInbox,
            });
        }

        let posted = MessagePosted {
            message_id: input.message_id.clone(),
            to: input.to.clone(),
            from: input.from.clone(),
            thread: input.thread.clone(),
            kind: input.kind,
        };
        let data = MessageData {
            message_id: input.message_id,
            from: input.from,
            on_behalf_of: input.on_behalf_of,
            authority: input.authority,
            to: input.to,
            kind: input.kind,
            thread: input.thread,
            reply_shape: input.reply_shape,
            decision: input.decision,
            default_on_silence: input.default_on_silence,
            created: input.created,
            expires: input.expires,
            dedup_key: input.dedup_key,
            body: input.body,
            signature: input.signature,
            answered_by: None,
            decided_option: None,
            reply: None,
            refusal: None,
        };
        MessageStorage::put(
            &mut self.backend,
            AnyMessage::Sent(Message::new(data)).snapshot(),
        );
        Ok(PostMessageOutcome::Posted {
            message_posted: posted,
        })
    }
}

/// `inbox.messaging.Expire`: `unknown-message` for an id no row carries, then the guards on the
/// stored row in declaration order: `not-due`, `decided-on-silence`, and `expired` otherwise.
impl<B: MessageStorage, C> ExpireBehavior for Core<B, C> {
    fn expire(&mut self, input: Expire) -> Result<ExpireOutcome, UnmetObligation> {
        let Some(held) = MessageStorage::get(&self.backend, &input.message_id) else {
            return Ok(ExpireOutcome::UnknownMessage {
                error: UnknownMessage,
            });
        };
        if instants(&held.data.expires, Op::Gt, &input.now, EXPIRE)? {
            return Ok(ExpireOutcome::NotDue { error: NotDue });
        }

        let open = matches!(
            held.state,
            MessageState::Sent | MessageState::Delivered | MessageState::Read
        );
        if held.data.kind == Kind::Decision && open && held.data.default_on_silence.is_some() {
            let decided = held.data.default_on_silence.clone();
            let taken = DecisionTakenOnSilence {
                message_id: input.message_id.clone(),
                to: held.data.from.clone(),
                thread: held.data.thread.clone(),
            };
            let answered = match held.refine() {
                AnyMessage::Sent(message) => message.decide_on_silence(),
                AnyMessage::Delivered(message) => message.decide_on_silence(),
                AnyMessage::Read(message) => message.decide_on_silence(),
                AnyMessage::Answered(_) | AnyMessage::Refused(_) => return Err(undeclared(EXPIRE)),
            };
            let mut next = AnyMessage::Answered(answered).snapshot();
            next.data.expires = input.keep_until;
            next.data.answered_by = Some(AnsweredBy::Silence);
            next.data.decided_option = decided;
            MessageStorage::put(&mut self.backend, next);
            return Ok(ExpireOutcome::DecidedOnSilence {
                decision_taken_on_silence: taken,
            });
        }

        MessageStorage::delete(&mut self.backend, &input.message_id);
        Ok(ExpireOutcome::Expired {
            message_expired: MessageExpired {
                message_id: input.message_id,
            },
        })
    }
}
