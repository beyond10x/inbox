//! The synthesized ESS conformance suite, run against the core of the inbox in process.
//!
//! [`suite::read`] reads an `ess-conformance/52` suite. Nothing the suite states is silently
//! ignored:
//!
//! - a key the runner does not know at the top level of the suite or in `provenance` belongs to
//!   no single scenario, so the whole suite is refused, naming the key (`run` exits 2), as is a
//!   suite of another format, one not starting from an empty system, or one with an object that
//!   states a member name twice at any depth (ESS's own reader refuses that too);
//! - a step kind, key, value reference, predicate, shape or declaration the runner does not
//!   interpret inside a scenario makes that scenario unsupported, with the reason, and none of its
//!   steps runs; `unsupported.tsv` must list it for the gate to pass.
//!
//! [`run_suite`] interprets every other scenario against a [`Target`], each from an empty system,
//! and gives each one terminal status: passed, failed, error or unsupported. [`CoreTarget`] is
//! the one target: the generated `handle` over the 2 obligations of `inbox-core` and its
//! `MemoryBackend`, with a [`StepContext`] that carries the step's caller and answers each of the
//! 6 `external:` branches as the step's expected outcome says.
//!
//! [`results::document`] writes the `ess-conformance-results/1` document `ess verify conform
//! report` turns into a report/2, and [`gate::verdict`] and [`gate::bound`] hold that report to
//! the suite it was made for (its digest and its scenario ids, each once), to the implementation
//! it names where `gate --implementation` is given, and to `unsupported.tsv`. `task conform` runs
//! the three, with one name for the implementation the report is written for and the gate holds.
//!
//! What the runner compares, as far as the suite states it and as the ESS 0.96.0 runner compares
//! it: the outcome and the command it answers, the declared error or its absence, a refusal as
//! not granted and the actor it names, each claimed event by name and exact payload (matched to
//! distinct occurrences, and checked against the step's payload shape), the absence of an event
//! or of all events, view rows (`contains` and `excludes`, `counts`, `satisfies`,
//! `value_invariants` over every row as `Contract::observe` does, an error where its
//! declarations are not the closure of the types it reaches, as ESS's `Contract::admit` refuses
//! them) and a subject row, exactly one, kept exactly unchanged. It does not compare a declared
//! error's payload, and no step of the suite claims one; it does not check the declared types of
//! a complete subject's fields (ESS `admit_row`).
//!
//! One departure from ESS: a top-level member a view row leaves out and one it carries as `null`
//! are one value ([`value::field_matches`]), where ESS compares a plain top-level field exactly,
//! because the generated server leaves an unset `Optional` member out (beyond10x/ess#799).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod core_target;
pub mod gate;
pub mod invariants;
pub mod results;
mod runner;
pub mod suite;
pub mod value;

pub use core_target::{CoreTarget, StepContext};
pub use runner::{
    Answer, Command, Event, Executed, Read, Reader, ScenarioResult, Status, Target, run_scenario,
    run_suite,
};
