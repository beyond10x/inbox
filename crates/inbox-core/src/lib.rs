//! The core of the inbox service: what the synthesized crate `inbox` leaves open, supplied once.
//!
//! `generated/rust/inbox` implements 7 of the 9 command behaviours and all 9 view queries over
//! storage and context ports it does not implement, and owes 2 obligations (`PLAN.md`). This crate
//! supplies the rest and restates none of the specification's rules:
//!
//! - [`Core`]: the behaviours of `PostMessage` and `Expire` (the obligations), over a backend `B`
//!   and a context `C`. Handed to [`inbox::behaviour::Generated`], it completes the system.
//! - [`ServiceContext`]: the generated context port ([`inbox::behaviour::TryContext`]) and
//!   [`PostExternals`], built from a caller, a [`SignatureVerifier`], the backend and the service's
//!   [`CommandClock`]. It answers the 9 `external:` branches: 3 of `PostMessage`, 5 of `Answer`, 1
//!   of `Refuse`. The clock is part of the context the embedding passes per request, so
//!   [`handle`] applies the deadline of a message as the HTTP service does.
//! - [`MemoryBackend`]: both generated storage ports, in memory. Its clones share one store, and it
//!   is `Send` and `Sync`.
//!
//! [`handle`] runs one command or view for one caller, one command at a time per shared store
//! ([`Exclusive`]); it is the entry for an embedding with another caller per request. [`system`]
//! assembles the generated [`inbox::system::System`] and serializes nothing.

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

use inbox::actor::Caller;
use inbox::behaviour::{Generated, InboxStorage, MessageStorage, TryContext};
use inbox::ports::inbox_service::InboxService;
use inbox::server::entry::Refused;
use inbox::server::inbox_service;
use inbox::server::json::Value;
use inbox::system::{System, SystemEvent};

/// The generated system over the behaviours of [`Core`]: what the generated `handle` and
/// `dispatch` run.
pub type Service<B, C> = System<Generated<Core<B, C>>>;

/// Assembles the generated system: the 7 generated behaviours and 9 view queries, and the 2
/// obligations of [`Core`], over `backend` and `context`.
///
/// The generated `InboxService` gives no access to its behaviours after construction, so the
/// context is fixed for the life of the system.
///
/// `system` alone serializes nothing: two systems over clones of one backend, run at once, each
/// read before the other writes, and both decide on the same rows. An embedding with another
/// caller per request goes through [`handle`], which runs each request inside the backend's
/// [`Exclusive`] section.
pub fn system<B, C>(backend: B, context: C) -> Service<B, C> {
    System::new(InboxService::new(Generated::new(Core::new(
        backend, context,
    ))))
}

/// Runs one command or view by its qualified name for one request, as the generated
/// [`inbox_service::handle`] does, inside the backend's [`Exclusive`] section.
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
    backend.exclusive(|| {
        let mut service = system(backend.clone(), context);
        inbox_service::handle_with_events(&mut service, caller, name, input)
    })
}

const _: fn() = || {
    fn shared<T: Send + Sync + Clone + 'static>() {}
    shared::<MemoryBackend>();
};
