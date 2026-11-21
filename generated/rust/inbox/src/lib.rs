// generated from inbox v1
// contract digest 0c82df6c630e3857fe5406f8adc42b037d2ff1786bd61c52ae04e85fd2828fd9
// do not edit: regenerate with `ess synthesize --layout crate`

//! Semantic types synthesised from the `inbox` specification, v1.
//!
//! Generated, not written: the specification is the source of truth, and the door to changing
//! anything here is `ess synthesize`. What is deliberately absent — behaviour, queries,
//! escalations — is listed with reasons in the `PLAN.md` beside this workspace, and every entry
//! there is owed through a typed seam in an `obligations` module here.

// `deny`, not the source workspace's lint set: this crate must hold on its own, and an undocumented
// public item here is an emitter defect worth failing the gate over.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod actor;
pub mod behaviour;
pub mod messaging;
pub mod obligation;
pub mod primitives;

pub mod ports;
pub mod system;
#[cfg(feature = "server")]
pub mod server;
