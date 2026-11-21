//! The core of the inbox service: what the synthesized crate `inbox` leaves open, supplied once.
//!
//! `generated/rust/inbox` implements 7 of the 9 command behaviours and all 9 view queries over
//! storage and context ports it does not implement, and owes 2 obligations (`PLAN.md`). This crate
//! supplies the rest and restates none of the specification's rules:
//!
//! - [`Core`]: the behaviours of `PostMessage` and `Expire` (the obligations), over a backend `B`
//!   and a context `C`. Handed to [`inbox::behaviour::Generated`], it completes the system.
//! - [`ServiceContext`]: the generated context port ([`inbox::behaviour::TryContext`]) and
//!   [`PostExternals`], built from a caller, a [`SignatureVerifier`] and the backend. It answers the
//!   6 `external:` branches: 3 of `PostMessage`, 3 of `Answer`.
//! - [`MemoryBackend`]: both generated storage ports, in memory. Its clones share one store, and it
//!   is `Send` and `Sync`.
//!
//! [`system`] assembles the generated [`inbox::system::System`] over a backend and a context.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod backend;
mod context;
mod obligations;

pub use backend::MemoryBackend;
pub use context::{CallerAttributes, Envelope, PostExternals, ServiceContext, SignatureVerifier};
pub use obligations::Core;

use inbox::behaviour::Generated;
use inbox::ports::inbox_service::InboxService;
use inbox::system::System;

/// The generated system over the behaviours of [`Core`]: what the generated `handle` and
/// `dispatch` run.
pub type Service<B, C> = System<Generated<Core<B, C>>>;

/// Assembles the generated system: the 7 generated behaviours and 9 view queries, and the 2
/// obligations of [`Core`], over `backend` and `context`.
///
/// The generated `InboxService` gives no access to its behaviours after construction, so the
/// context is fixed for the life of the system. An embedding with another caller per request
/// builds one per request over a clone of the backend.
pub fn system<B, C>(backend: B, context: C) -> Service<B, C> {
    System::new(InboxService::new(Generated::new(Core::new(
        backend, context,
    ))))
}

const _: fn() = || {
    fn shared<T: Send + Sync + Clone + 'static>() {}
    shared::<MemoryBackend>();
};
