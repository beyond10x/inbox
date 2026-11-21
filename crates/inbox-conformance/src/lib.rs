//! The synthesized ESS conformance suite, run against the core of the inbox in process.
//!
//! [`suite::read`] reads an `ess-conformance/52` suite; a scenario holding a construct this
//! runner does not interpret is admitted as unsupported with its reason and not run.
//! [`run_suite`] interprets every other scenario against a [`Target`], each from an empty system,
//! and gives each one terminal status: passed, failed, error or unsupported. [`CoreTarget`] is
//! the one target: the generated `handle` over the 2 obligations of `inbox-core` and its
//! `MemoryBackend`, with a [`StepContext`] that carries the step's caller and answers each of the
//! 6 `external:` branches as the step's expected outcome says.
//!
//! [`results::document`] writes the `ess-conformance-results/1` document `ess verify conform
//! report` turns into a report/2, and [`gate::verdict`] holds that report to `unsupported.tsv`.
//! `task conform` runs the three.
//!
//! What the runner compares, as far as the suite states it: the outcome and the command it
//! answers, the declared error or its absence, a refusal as not granted and the actor it names,
//! each claimed event by name and exact payload (matched to distinct occurrences, and checked
//! against the step's payload shape), the absence of an event or of all events, view rows
//! (`contains`, `excludes`, `counts`, `satisfies`, `value_invariants`) and a subject row kept
//! unchanged. It does not compare a declared error's payload, and no step of the suite claims
//! one.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod core_target;
pub mod gate;
pub mod results;
mod runner;
pub mod suite;
pub mod value;

pub use core_target::{CoreTarget, StepContext};
pub use runner::{
    Answer, Command, Event, Executed, Read, Reader, ScenarioResult, Status, Target, run_scenario,
    run_suite,
};
