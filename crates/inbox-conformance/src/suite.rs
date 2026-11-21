//! The synthesized suite, read into the steps this runner interprets.
//!
//! Nothing the suite states is silently ignored. What the runner does with a key or construct it
//! does not know depends on who owns it:
//!
//! - at the top level of the suite or in `provenance`, no single scenario owns it: [`read`]
//!   refuses the whole suite, naming the key (the binary exits 2); so it does with a member name
//!   an object states twice, at any depth, a suite ESS itself would not read;
//! - in a scenario, a step, an expectation, a value reference, a predicate or a declaration (a
//!   step kind, a key, a shape it has no reading for): the scenario is admitted as
//!   [`Scenario::unsupported`] with the reason, and no step of it is executed.
//!
//! The suite's `coverage` block is read into [`Coverage`] and held to the suite (see there); it
//! names no step, so nothing of it is executed.

use std::collections::BTreeMap;

use inbox::server::json::{self, Value};

/// The suite format this runner is written against: the one `ess verify conform synthesize
/// --suite-format 5` of ESS 0.96.0 writes for `ess/`, the ordinary scenarios with a declared
/// `coverage` block. The ordinary `ess-conformance/52` (no coverage) is refused like any other
/// format: no task synthesizes it any more.
pub const SUITE_VERSION: &str = "ess-conformance/65";

/// A suite this runner read.
#[derive(Debug, Clone)]
pub struct Suite {
    /// Every scenario, in the suite's order.
    pub scenarios: Vec<Scenario>,
    /// What the suite declares about its own coverage.
    pub coverage: Coverage,
}

/// The suite's `coverage` block: which of its scenarios are authored and which generated, and
/// what synthesis refused to write.
///
/// The runner reads it as ESS's own reader does on 0.96.0 (measured with `ess verify conform run
/// --target interpreted` on altered copies of the suite `task coverage` writes), and refuses the
/// suite where:
///
/// - `coverage` is absent, or carries a member other than the 8 of the format (`authored`,
///   `authored_sources`, `counts`, `generated`, `knowledge`, `outside`, `refused`, `selection`),
///   or lacks one;
/// - `authored` and `generated` together do not name each scenario of the suite exactly once, or
///   name one the suite does not hold (ESS: "selected origin union differs from scenarios");
/// - a count in `counts` differs from the length of its list (ESS: "inventory counts differ from
///   array lengths");
/// - `knowledge` is neither `complete_inventory` nor `unknown`.
///
/// `refused` (each an object), `outside`, `authored_sources` and `selection` are read for their
/// shape only. They speak of scenarios synthesis did not write, or of where the written ones came
/// from, so no target can run anything of them; `ess verify conform report` reads them from the
/// suite itself when it decides `conformance_status`. ESS additionally checks that each
/// `covered_by` of `refused` is spelled as a scenario id; the runner does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coverage {
    /// The authored scenarios, by id.
    pub authored: Vec<String>,
    /// The generated scenarios, by id.
    pub generated: Vec<String>,
    /// How many scenarios synthesis refused to write.
    pub refused: usize,
    /// `complete_inventory` or `unknown`.
    pub knowledge: String,
}

/// One scenario: its steps, or why the runner cannot interpret it.
#[derive(Debug, Clone)]
pub struct Scenario {
    /// The scenario's id.
    pub id: String,
    /// The steps, with `absent_events` and `unpublished_events` expanded into
    /// [`Step::ExpectNoEvent`] steps; `Err` names the construct the runner does not interpret.
    pub steps: Result<Vec<Step>, String>,
}

impl Scenario {
    /// The reason the runner does not interpret this scenario, if it does not.
    pub fn unsupported(&self) -> Option<&str> {
        self.steps.as_ref().err().map(String::as_str)
    }
}

/// A value a step names: a literal, or an instance an earlier step captured.
#[derive(Debug, Clone, PartialEq)]
pub enum Ref {
    /// `{kind: literal, value}`.
    Literal(Value),
    /// `{kind: instance, instance}`: the value `capture_instance` bound to the name.
    Instance(String),
}

/// Named values: the fields of an input, a view's parameters, a row claim or a subject.
pub type Fields = Vec<(String, Ref)>;

/// The type an event payload member holds, as the step's `shape` declares it.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// `{holds: primitive, kind: string}`.
    Text,
    /// `{holds: enum, variants}`.
    Enum(Vec<String>),
}

/// A comparison operator of a predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// `==`, `eq`.
    Eq,
    /// `!=`, `ne`.
    Ne,
    /// `<`, `lt`.
    Lt,
    /// `<=`, `lte`.
    Le,
    /// `>`, `gt`.
    Gt,
    /// `>=`, `gte`.
    Ge,
}

impl Op {
    /// Whether `left op right` holds.
    pub fn holds(self, left: f64, right: f64) -> bool {
        match self {
            Self::Eq => left == right,
            Self::Ne => left != right,
            Self::Lt => left < right,
            Self::Le => left <= right,
            Self::Gt => left > right,
            Self::Ge => left >= right,
        }
    }

    fn symbol(word: &str) -> Option<Self> {
        Some(match word {
            "==" | "eq" => Self::Eq,
            "!=" | "ne" => Self::Ne,
            "<" | "lt" => Self::Lt,
            "<=" | "lte" => Self::Le,
            ">" | "gt" => Self::Gt,
            ">=" | "gte" => Self::Ge,
            _ => return None,
        })
    }
}

/// A predicate every row of a view satisfies (`expect: satisfies`).
#[derive(Debug, Clone, PartialEq)]
pub enum Predicate {
    /// `<field>.count <op> <n>`: a text's Unicode scalar values, or a list's elements.
    Count {
        /// The row member.
        field: String,
        /// The operator.
        op: Op,
        /// The bound.
        bound: f64,
    },
    /// `{compare: {left: {utf8_bytes: <field>}, op, right}}`: a text's UTF-8 bytes.
    Utf8Bytes {
        /// The row member.
        field: String,
        /// The operator.
        op: Op,
        /// The bound.
        bound: f64,
    },
}

pub use crate::invariants::{Body, PathOp, Primitive, TypeRef, ValueInvariants};

/// What a step expects of the rows the last read answered.
#[derive(Debug, Clone, PartialEq)]
pub enum Expectation {
    /// Some row holds every field.
    Contains(Fields),
    /// No row holds every field.
    Excludes(Fields),
    /// The number of rows is within the bounds.
    Counts {
        /// The least.
        at_least: Option<u64>,
        /// The most.
        at_most: Option<u64>,
    },
    /// Every row satisfies the predicate.
    Satisfies(Predicate),
    /// The values of a newtype in the witness row satisfy the type.
    ValueInvariants(ValueInvariants),
}

/// One step, as the runner interprets it.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// `execute_command`.
    Execute {
        /// The command's qualified name.
        command: String,
        /// The actor the command is sent as; `None` for no actor.
        actor: Option<String>,
        /// The caller's attributes, by name.
        caller: Vec<(String, String)>,
        /// The input.
        input: Fields,
    },
    /// `expect_outcome`.
    ExpectOutcome {
        /// The command.
        command: String,
        /// The outcome.
        outcome: String,
    },
    /// `expect_no_error`.
    ExpectNoError,
    /// `expect_error`.
    ExpectError {
        /// The error's qualified name.
        error: String,
    },
    /// `expect_event` and `expect_event_values`: one occurrence of the event, with exactly this
    /// payload, among what the last command published.
    ExpectEvent {
        /// The event.
        event: String,
        /// The payload.
        payload: Fields,
        /// The type of each payload member, where the step declares it.
        shape: Option<Vec<(String, Shape)>>,
    },
    /// `expect_no_event`, and each event `absent_events` and `unpublished_events` leave out.
    ExpectNoEvent {
        /// The event.
        event: String,
    },
    /// `expect_no_events`.
    ExpectNoEvents,
    /// `expect_not_granted`: the last command or read was refused before it ran.
    ExpectNotGranted {
        /// The actor the refusal names; `None` for no actor.
        actor: Option<String>,
    },
    /// `capture_instance`: bind a payload member of the event the last command published.
    Capture {
        /// The name.
        instance: String,
        /// The event.
        event: String,
        /// The payload member.
        field: String,
    },
    /// `configure_external_outcome`: the external branch the next send of the command takes.
    ConfigureExternal {
        /// The command.
        command: String,
        /// The outcome forced.
        outcome: String,
    },
    /// `read_as`: the actor later reads are sent as; `None` for no actor.
    ReadAs {
        /// The actor.
        actor: Option<String>,
    },
    /// `query_view`.
    Query {
        /// The view's qualified name.
        view: String,
        /// The parameters.
        params: Fields,
    },
    /// `expect_view`.
    ExpectView {
        /// The view.
        view: String,
        /// What its rows hold.
        expectation: Expectation,
    },
    /// `snapshot_subject` and `snapshot_complete_subject`: keep the row the subject names from
    /// the last read.
    Snapshot {
        /// The view.
        view: String,
        /// The subject's fields.
        subject: Fields,
        /// The row members compared later, for a complete snapshot; the whole row otherwise.
        complete: Option<Vec<String>>,
    },
    /// `expect_subject_unchanged` and `expect_complete_subject_unchanged`.
    ExpectUnchanged {
        /// The view.
        view: String,
        /// Whether the snapshot is a complete one.
        complete: bool,
    },
}

/// `Err` naming the first member an object of `value` states twice: ESS reads the suite with
/// serde, which refuses a field stated twice, and the JSON reader here keeps both.
fn stated_once(value: &Value, location: &str) -> Result<(), String> {
    match value {
        Value::Object(members) => {
            for (at, (key, member)) in members.iter().enumerate() {
                if members[..at].iter().any(|(earlier, _)| earlier == key) {
                    return Err(format!("{location} states the member `{key}` twice"));
                }
                stated_once(member, &format!("{location}.{key}"))?;
            }
            Ok(())
        }
        Value::Array(items) => items
            .iter()
            .enumerate()
            .try_for_each(|(index, item)| stated_once(item, &format!("{location}[{index}]"))),
        _ => Ok(()),
    }
}

/// Reads a suite document.
///
/// # Errors
///
/// A document that is not JSON, not a suite of [`SUITE_VERSION`], not one that starts every
/// scenario from an empty system, one carrying a top-level or `provenance` key this runner does
/// not know, an object that states one member name twice, at any depth, or a `coverage` block
/// that does not account for the suite ([`Coverage`]). A scenario this runner cannot interpret is
/// no error: it is read as unsupported.
pub fn read(text: &str) -> Result<Suite, String> {
    let document = json::parse_written(text).map_err(|error| format!("the suite: {error}"))?;
    stated_once(&document, "the suite")?;
    let root = object(&document, "the suite")?;
    only(
        root,
        &["coverage", "provenance", "events", "scenarios"],
        "the suite",
    )?;
    let provenance = object(member(root, "provenance", "the suite")?, "provenance")?;
    only(
        provenance,
        &[
            "suite_version",
            "system",
            "specification_version",
            "spec_digest",
            "contract_digest",
            "scenario_initial_state",
        ],
        "provenance",
    )?;
    let version = text_member(provenance, "suite_version", "provenance")?;
    if version != SUITE_VERSION {
        return Err(format!(
            "the suite is `{version}`; this runner reads `{SUITE_VERSION}`: regenerate it with the `ess` release `ess-inputs.yaml` pins, or teach the runner the new format"
        ));
    }
    let initial = text_member(provenance, "scenario_initial_state", "provenance")?;
    if initial != "empty" {
        return Err(format!(
            "the suite starts its scenarios from `{initial}`; this runner starts each from an empty store"
        ));
    }
    let catalog = items(member(root, "events", "the suite")?, "events")?
        .iter()
        .map(|event| as_text(event, "an event of the catalog").map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    let scenarios: Vec<Scenario> = object(member(root, "scenarios", "the suite")?, "scenarios")?
        .iter()
        .map(|(id, body)| Scenario {
            id: id.clone(),
            steps: scenario(body, &catalog),
        })
        .collect();
    let ids: Vec<&str> = scenarios
        .iter()
        .map(|scenario| scenario.id.as_str())
        .collect();
    let coverage = coverage(member(root, "coverage", "the suite")?, &ids)?;
    Ok(Suite {
        scenarios,
        coverage,
    })
}

/// The members of a `coverage` block.
const COVERAGE: [&str; 8] = [
    "authored",
    "authored_sources",
    "counts",
    "generated",
    "knowledge",
    "outside",
    "refused",
    "selection",
];

/// Reads the `coverage` block and holds it to the scenario ids of the suite ([`Coverage`]).
fn coverage(value: &Value, ids: &[&str]) -> Result<Coverage, String> {
    let fields = object(value, "coverage")?;
    only(fields, &COVERAGE, "coverage")?;
    for name in COVERAGE {
        member(fields, name, "coverage")?;
    }
    let list = |name: &str| -> Result<&[Value], String> {
        items(
            member(fields, name, "coverage")?,
            &format!("coverage `{name}`"),
        )
    };
    let names = |name: &str| -> Result<Vec<String>, String> {
        list(name)?
            .iter()
            .map(|id| as_text(id, &format!("an id of coverage `{name}`")).map(str::to_owned))
            .collect()
    };
    let authored = names("authored")?;
    let generated = names("generated")?;
    let outside = list("outside")?.len();
    let refused = list("refused")?;
    for entry in refused {
        object(entry, "an entry of coverage `refused`")?;
    }
    object(
        member(fields, "authored_sources", "coverage")?,
        "coverage `authored_sources`",
    )?;
    object(
        member(fields, "selection", "coverage")?,
        "coverage `selection`",
    )?;
    let counts = object(member(fields, "counts", "coverage")?, "coverage `counts`")?;
    let lengths = [
        ("authored", authored.len()),
        ("generated", generated.len()),
        ("outside", outside),
        ("refused", refused.len()),
    ];
    only(counts, &lengths.map(|(name, _)| name), "coverage `counts`")?;
    for (name, length) in lengths {
        let stated = count(
            member(counts, name, "coverage `counts`")?,
            &format!("coverage `counts.{name}`"),
        )?;
        if stated != length as u64 {
            return Err(format!(
                "coverage `counts.{name}` is {stated}, and coverage `{name}` lists {length}"
            ));
        }
    }
    let knowledge = text_member(fields, "knowledge", "coverage")?;
    if !["complete_inventory", "unknown"].contains(&knowledge) {
        return Err(format!(
            "coverage `knowledge` is `{knowledge}`, which is neither `complete_inventory` nor `unknown`"
        ));
    }
    let mut named: BTreeMap<&str, usize> = BTreeMap::new();
    for id in authored.iter().chain(&generated) {
        *named.entry(id.as_str()).or_default() += 1;
    }
    for id in ids {
        match named.get(id) {
            None => {
                return Err(format!(
                    "coverage names the scenario `{id}` neither authored nor generated"
                ));
            }
            Some(1) => {}
            Some(times) => {
                return Err(format!("coverage names the scenario `{id}` {times} times"));
            }
        }
    }
    if let Some(stray) = named.keys().find(|id| !ids.contains(id)) {
        return Err(format!(
            "coverage names `{stray}`, which is no scenario of the suite"
        ));
    }
    Ok(Coverage {
        authored,
        generated,
        refused: refused.len(),
        knowledge: knowledge.to_owned(),
    })
}

fn scenario(body: &Value, catalog: &[String]) -> Result<Vec<Step>, String> {
    let body = object(body, "the scenario")?;
    only(
        body,
        &[
            "purpose",
            "source",
            "steps",
            "absent_events",
            "unpublished_events",
        ],
        "the scenario",
    )?;
    let raw = items(member(body, "steps", "the scenario")?, "steps")?;
    let mut inserted: BTreeMap<usize, Vec<Step>> = BTreeMap::new();
    for (key, position, offset) in [
        ("absent_events", "before", 0),
        ("unpublished_events", "step", 1),
    ] {
        let Some(runs) = body.iter().find(|(name, _)| name == key) else {
            continue;
        };
        for run in items(&runs.1, key)? {
            let run = object(run, key)?;
            only(run, &[position, "except"], key)?;
            let at = index(member(run, position, key)?, key)? + offset;
            if at > raw.len() {
                return Err(format!("`{key}` names step {at}, past the scenario's last"));
            }
            let except = items(member(run, "except", key)?, key)?
                .iter()
                .map(|event| as_text(event, key))
                .collect::<Result<Vec<_>, _>>()?;
            inserted.entry(at).or_default().extend(
                catalog
                    .iter()
                    .filter(|event| !except.contains(&event.as_str()))
                    .map(|event| Step::ExpectNoEvent {
                        event: event.clone(),
                    }),
            );
        }
    }
    let mut steps = Vec::new();
    for (at, value) in raw.iter().enumerate() {
        if let Some(more) = inserted.remove(&at) {
            steps.extend(more);
        }
        steps.push(step(value).map_err(|reason| format!("step {at}: {reason}"))?);
    }
    if let Some(more) = inserted.remove(&raw.len()) {
        steps.extend(more);
    }
    Ok(steps)
}

fn step(value: &Value) -> Result<Step, String> {
    let fields = object(value, "a step")?;
    let kind = text_member(fields, "step", "a step")?;
    let what = format!("`{kind}`");
    let keys = |allowed: &[&str]| {
        let mut all = vec!["step"];
        all.extend_from_slice(allowed);
        only(fields, &all, &what)
    };
    let text = |name: &str| text_member(fields, name, &what).map(str::to_owned);
    Ok(match kind {
        "execute_command" => {
            keys(&["command", "actor", "caller", "input"])?;
            let caller = match fields.iter().find(|(name, _)| name == "caller") {
                None => Vec::new(),
                Some((_, caller)) => object(caller, "caller")?
                    .iter()
                    .map(|(name, value)| Ok((name.clone(), as_text(value, "caller")?.to_owned())))
                    .collect::<Result<_, String>>()?,
            };
            Step::Execute {
                command: text("command")?,
                actor: actor(fields, &what)?,
                caller,
                input: refs(member(fields, "input", &what)?, "input")?,
            }
        }
        "expect_outcome" => {
            keys(&["outcome"])?;
            let outcome = object(member(fields, "outcome", &what)?, "outcome")?;
            only(outcome, &["command", "outcome"], "outcome")?;
            Step::ExpectOutcome {
                command: text_member(outcome, "command", "outcome")?.to_owned(),
                outcome: text_member(outcome, "outcome", "outcome")?.to_owned(),
            }
        }
        "expect_no_error" => {
            keys(&[])?;
            Step::ExpectNoError
        }
        "expect_error" => {
            keys(&["error"])?;
            Step::ExpectError {
                error: text("error")?,
            }
        }
        "expect_event" | "expect_event_values" => {
            keys(&["event", "payload", "shape"])?;
            let payload = member(fields, "payload", &what)?;
            let payload = if kind == "expect_event" {
                object(payload, "payload")?
                    .iter()
                    .map(|(name, value)| (name.clone(), Ref::Literal(value.clone())))
                    .collect()
            } else {
                refs(payload, "payload")?
            };
            let shape = match fields.iter().find(|(name, _)| name == "shape") {
                None => None,
                Some((_, shape)) => Some(shapes(shape)?),
            };
            Step::ExpectEvent {
                event: text("event")?,
                payload,
                shape,
            }
        }
        "expect_no_event" => {
            keys(&["event"])?;
            Step::ExpectNoEvent {
                event: text("event")?,
            }
        }
        "expect_no_events" => {
            keys(&[])?;
            Step::ExpectNoEvents
        }
        "expect_not_granted" => {
            keys(&["actor"])?;
            Step::ExpectNotGranted {
                actor: actor(fields, &what)?,
            }
        }
        "capture_instance" => {
            keys(&["instance", "entity", "event", "field"])?;
            text("entity")?;
            Step::Capture {
                instance: text("instance")?,
                event: text("event")?,
                field: text("field")?,
            }
        }
        "configure_external_outcome" => {
            keys(&["force"])?;
            let force = object(member(fields, "force", &what)?, "force")?;
            only(force, &["command", "outcome"], "force")?;
            Step::ConfigureExternal {
                command: text_member(force, "command", "force")?.to_owned(),
                outcome: text_member(force, "outcome", "force")?.to_owned(),
            }
        }
        "read_as" => {
            keys(&["actor"])?;
            Step::ReadAs {
                actor: actor(fields, &what)?,
            }
        }
        "query_view" => {
            keys(&["view", "params"])?;
            let params = match fields.iter().find(|(name, _)| name == "params") {
                None => Vec::new(),
                Some((_, params)) => refs(params, "params")?,
            };
            Step::Query {
                view: text("view")?,
                params,
            }
        }
        "expect_view" => {
            keys(&["view", "expectation"])?;
            Step::ExpectView {
                view: text("view")?,
                expectation: expectation(member(fields, "expectation", &what)?)?,
            }
        }
        "snapshot_subject" => {
            keys(&["view", "subject"])?;
            Step::Snapshot {
                view: text("view")?,
                subject: subject(member(fields, "subject", &what)?)?,
                complete: None,
            }
        }
        "snapshot_complete_subject" => {
            keys(&["view", "subject", "shape"])?;
            let shape = object(member(fields, "shape", &what)?, "shape")?;
            only(
                shape,
                &["identity_field", "fields", "declarations"],
                "shape",
            )?;
            let names = items(member(shape, "fields", "shape")?, "shape fields")?
                .iter()
                .map(|field| {
                    let field = object(field, "a shape field")?;
                    only(field, &["name", "type"], "a shape field")?;
                    text_member(field, "name", "a shape field").map(str::to_owned)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Step::Snapshot {
                view: text("view")?,
                subject: subject(member(fields, "subject", &what)?)?,
                complete: Some(names),
            }
        }
        "expect_subject_unchanged" | "expect_complete_subject_unchanged" => {
            keys(&["view"])?;
            Step::ExpectUnchanged {
                view: text("view")?,
                complete: kind == "expect_complete_subject_unchanged",
            }
        }
        other => {
            return Err(format!(
                "the step kind `{other}` is not interpreted by this runner"
            ));
        }
    })
}

fn expectation(value: &Value) -> Result<Expectation, String> {
    let fields = object(value, "expectation")?;
    let kind = text_member(fields, "expect", "expectation")?;
    let what = format!("expectation `{kind}`");
    let keys = |allowed: &[&str]| {
        let mut all = vec!["expect"];
        all.extend_from_slice(allowed);
        only(fields, &all, &what)
    };
    Ok(match kind {
        "contains" | "excludes" => {
            keys(&["fields"])?;
            let claimed = refs(member(fields, "fields", &what)?, "fields")?;
            if let Some((name, _)) = claimed
                .iter()
                .find(|(name, _)| name.split('.').any(str::is_empty))
            {
                return Err(format!(
                    "the row path `{name}` names an empty member, which no row carries"
                ));
            }
            if kind == "contains" {
                Expectation::Contains(claimed)
            } else {
                Expectation::Excludes(claimed)
            }
        }
        "counts" => {
            keys(&["at_least", "at_most"])?;
            let bound = |name: &str| {
                fields
                    .iter()
                    .find(|(key, _)| key == name)
                    .map(|(_, value)| count(value, name))
                    .transpose()
            };
            Expectation::Counts {
                at_least: bound("at_least")?,
                at_most: bound("at_most")?,
            }
        }
        "satisfies" => {
            keys(&["predicate"])?;
            Expectation::Satisfies(predicate(member(fields, "predicate", &what)?)?)
        }
        "value_invariants" => {
            keys(&[
                "root",
                "declarations",
                "value",
                "position",
                "witness",
                "work_limit",
            ])?;
            Expectation::ValueInvariants(value_invariants(fields)?)
        }
        other => {
            return Err(format!(
                "the view expectation `{other}` is not interpreted by this runner"
            ));
        }
    })
}

fn predicate(value: &Value) -> Result<Predicate, String> {
    match value {
        Value::Text(text) => {
            let (field, op, bound) = counted(text).ok_or_else(|| {
                format!("the predicate `{text}` is not one this runner interprets")
            })?;
            Ok(Predicate::Count {
                field: field.to_owned(),
                op,
                bound,
            })
        }
        Value::Object(fields) => {
            only(fields, &["compare"], "predicate")?;
            let compare = object(member(fields, "compare", "predicate")?, "compare")?;
            only(compare, &["left", "op", "right"], "compare")?;
            let left = object(member(compare, "left", "compare")?, "compare left")?;
            only(left, &["utf8_bytes"], "compare left")?;
            let field = text_member(left, "utf8_bytes", "compare left")?.to_owned();
            let op_word = text_member(compare, "op", "compare")?;
            let op = Op::symbol(op_word)
                .ok_or_else(|| format!("the operator `{op_word}` is not one this runner reads"))?;
            let bound = finite(member(compare, "right", "compare")?, "compare right")?;
            Ok(Predicate::Utf8Bytes { field, op, bound })
        }
        other => Err(format!(
            "a predicate written as {} is not one this runner interprets",
            other.describes()
        )),
    }
}

/// `<name>.count <op> <integer>`, the one textual predicate form this runner reads.
fn counted(text: &str) -> Option<(&str, Op, f64)> {
    let mut words = text.split(' ');
    let subject = words.next()?;
    let op = Op::symbol(words.next()?)?;
    let bound = words.next()?;
    if words.next().is_some() {
        return None;
    }
    let field = subject.strip_suffix(".count")?;
    let named = !field.is_empty()
        && field
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_');
    let integral = !bound.is_empty() && bound.chars().all(|character| character.is_ascii_digit());
    if !named || !integral {
        return None;
    }
    Some((field, op, bound.parse::<f64>().ok()?))
}

fn value_invariants(fields: &[(String, Value)]) -> Result<ValueInvariants, String> {
    let what = "expectation `value_invariants`";
    let root = object(member(fields, "root", what)?, "root")?;
    only(root, &["name", "type"], "root")?;
    let value_type = text_member(fields, "value", what)?.to_owned();
    let declarations = object(member(fields, "declarations", what)?, "declarations")?;
    let mut types = BTreeMap::new();
    for (name, declaration) in declarations {
        types.insert(name.clone(), body(name, declaration)?);
    }
    let root_type = crate::invariants::parse_type(text_member(root, "type", "root")?)?;
    let identity_type = {
        let witness = object(member(fields, "witness", what)?, "witness")?;
        let selector = object(member(witness, "selector", "witness")?, "selector")?;
        let identity = object(member(selector, "identity", "selector")?, "identity")?;
        crate::invariants::parse_type(text_member(identity, "type", "identity")?)?
    };
    let broken = crate::invariants::closure(vec![root_type.clone(), identity_type], &types).err();
    let invariants = || -> Result<&[Value], String> {
        let declaration = object(
            member(declarations, &value_type, "declarations")?,
            &value_type,
        )?;
        items(
            member(declaration, "invariants", &value_type)?,
            "invariants",
        )
    };
    let asserted = match types.get(&value_type) {
        Some(Body::Newtype { .. } | Body::Struct { .. }) => invariants()?.len(),
        _ => 0,
    };
    let broken = broken.or_else(|| {
        (asserted == 0)
            .then(|| format!("`{value_type}` is no declared newtype or struct with invariants"))
    });
    let counts = if broken.is_some() {
        Vec::new()
    } else {
        if !matches!(
            types.get(&value_type),
            Some(Body::Newtype {
                of: TypeRef::Primitive(Primitive::Text),
                ..
            })
        ) {
            return Err(format!(
                "`{value_type}` is not a newtype over text, the one value type this runner checks"
            ));
        }
        invariants()?
            .iter()
            .map(|invariant| {
                let invariant = object(invariant, "an invariant")?;
                let text = text_member(invariant, "predicate", "an invariant")?;
                let statement = text_member(invariant, "statement", "an invariant")?;
                match counted(text) {
                    Some(("value", op, bound)) => Ok((statement.to_owned(), op, bound)),
                    _ => Err(format!(
                        "the invariant `{text}` of `{value_type}` is not one this runner interprets"
                    )),
                }
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    let position = object(member(fields, "position", what)?, "position")?;
    only(position, &["kind", "path"], "position")?;
    if text_member(position, "kind", "position")? != "site" {
        return Err("a position other than `site` is not interpreted by this runner".to_owned());
    }
    let path = items(member(position, "path", "position")?, "path")?
        .iter()
        .map(|op| {
            let op = object(op, "a path op")?;
            match text_member(op, "op", "a path op")? {
                "present" => only(op, &["op"], "a path op").map(|()| PathOp::Present),
                "element" => only(op, &["op"], "a path op").map(|()| PathOp::Element),
                "field" => {
                    only(op, &["op", "name"], "a path op")?;
                    Ok(PathOp::Field(
                        text_member(op, "name", "a path op")?.to_owned(),
                    ))
                }
                other => Err(format!(
                    "the path op `{other}` is not interpreted by this runner"
                )),
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let witness = object(member(fields, "witness", what)?, "witness")?;
    only(witness, &["kind", "selector"], "witness")?;
    if text_member(witness, "kind", "witness")? != "arranged_row" {
        return Err(
            "a witness other than `arranged_row` is not interpreted by this runner".to_owned(),
        );
    }
    let selector = object(member(witness, "selector", "witness")?, "selector")?;
    only(
        selector,
        &["kind", "entity", "identity", "expected"],
        "selector",
    )?;
    if text_member(selector, "kind", "selector")? != "projected_identity" {
        return Err(
            "a selector other than `projected_identity` is not interpreted by this runner"
                .to_owned(),
        );
    }
    text_member(selector, "entity", "selector")?;
    let identity = object(member(selector, "identity", "selector")?, "identity")?;
    only(identity, &["name", "type"], "identity")?;
    let expected = object(member(selector, "expected", "selector")?, "expected")?;
    only(expected, &["kind", "event", "field"], "expected")?;
    if text_member(expected, "kind", "expected")? != "observed" {
        return Err(
            "a witness identity other than `observed` is not interpreted by this runner".to_owned(),
        );
    }
    Ok(ValueInvariants {
        root: text_member(root, "name", "root")?.to_owned(),
        root_type,
        types,
        value_type,
        counts,
        path,
        identity: text_member(identity, "name", "identity")?.to_owned(),
        event: text_member(expected, "event", "expected")?.to_owned(),
        field: text_member(expected, "field", "expected")?.to_owned(),
        broken,
    })
}

/// One declaration of a `value_invariants` expectation. The invariants of a declaration other
/// than the checked value type are read as statements only: ESS evaluates none of them there.
fn body(name: &str, declaration: &Value) -> Result<Body, String> {
    let declaration = object(declaration, name)?;
    let invariants = |declaration: &[(String, Value)]| -> Result<(), String> {
        for invariant in items(member(declaration, "invariants", name)?, "invariants")? {
            let invariant = object(invariant, "an invariant")?;
            only(invariant, &["statement", "predicate"], "an invariant")?;
            text_member(invariant, "statement", "an invariant")?;
        }
        Ok(())
    };
    let optional_text = |key: &str| -> Result<Option<String>, String> {
        declaration
            .iter()
            .find(|(member, _)| member == key)
            .map(|(_, value)| as_text(value, key).map(str::to_owned))
            .transpose()
    };
    match text_member(declaration, "kind", name)? {
        "newtype" => {
            only(
                declaration,
                &["kind", "of", "invariants", "alphabet", "prefix"],
                name,
            )?;
            invariants(declaration)?;
            Ok(Body::Newtype {
                of: crate::invariants::parse_type(text_member(declaration, "of", name)?)?,
                alphabet: optional_text("alphabet")?,
                prefix: optional_text("prefix")?,
            })
        }
        "struct" => {
            only(declaration, &["kind", "fields", "invariants"], name)?;
            invariants(declaration)?;
            let fields = items(member(declaration, "fields", name)?, "fields")?
                .iter()
                .map(|field| {
                    let field = object(field, "a declared field")?;
                    only(field, &["name", "type"], "a declared field")?;
                    Ok((
                        text_member(field, "name", "a declared field")?.to_owned(),
                        crate::invariants::parse_type(text_member(
                            field,
                            "type",
                            "a declared field",
                        )?)?,
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(Body::Struct { fields })
        }
        "enum" => {
            only(declaration, &["kind", "variants"], name)?;
            let variants = items(member(declaration, "variants", name)?, "variants")?
                .iter()
                .map(|variant| as_text(variant, "variants").map(str::to_owned))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Body::Enum { variants })
        }
        other => Err(format!(
            "the declaration kind `{other}` of `{name}` is not one this runner checks"
        )),
    }
}

fn shapes(value: &Value) -> Result<Vec<(String, Shape)>, String> {
    object(value, "shape")?
        .iter()
        .map(|(name, shape)| {
            let shape = object(shape, "a payload shape")?;
            let holds = text_member(shape, "holds", "a payload shape")?;
            let read = match holds {
                "primitive" => {
                    only(shape, &["holds", "kind"], "a payload shape")?;
                    match text_member(shape, "kind", "a payload shape")? {
                        "string" => Shape::Text,
                        other => {
                            return Err(format!(
                                "the payload shape `primitive {other}` is not interpreted by this runner"
                            ));
                        }
                    }
                }
                "enum" => {
                    only(shape, &["holds", "variants"], "a payload shape")?;
                    Shape::Enum(
                        items(member(shape, "variants", "a payload shape")?, "variants")?
                            .iter()
                            .map(|variant| as_text(variant, "variants").map(str::to_owned))
                            .collect::<Result<_, _>>()?,
                    )
                }
                other => {
                    return Err(format!(
                        "the payload shape `{other}` is not interpreted by this runner"
                    ));
                }
            };
            Ok((name.clone(), read))
        })
        .collect()
}

/// The fields of a snapshot's subject. A subject selects its row by top-level members compared
/// exactly; a dotted path there is not interpreted by this runner.
fn subject(value: &Value) -> Result<Fields, String> {
    let subject = refs(value, "subject")?;
    if let Some((name, _)) = subject.iter().find(|(name, _)| name.contains('.')) {
        return Err(format!(
            "the dotted subject path `{name}` is not interpreted by this runner"
        ));
    }
    Ok(subject)
}

fn refs(value: &Value, what: &str) -> Result<Fields, String> {
    object(value, what)?
        .iter()
        .map(|(name, reference)| {
            let reference = object(reference, what)?;
            let read = match text_member(reference, "kind", what)? {
                "literal" => {
                    only(reference, &["kind", "value"], what)?;
                    Ref::Literal(member(reference, "value", what)?.clone())
                }
                "instance" => {
                    only(reference, &["kind", "instance"], what)?;
                    Ref::Instance(text_member(reference, "instance", what)?.to_owned())
                }
                other => {
                    return Err(format!(
                        "the value reference `{other}` in {what} is not interpreted by this runner"
                    ));
                }
            };
            Ok((name.clone(), read))
        })
        .collect()
}

fn actor(fields: &[(String, Value)], what: &str) -> Result<Option<String>, String> {
    match member(fields, "actor", what)? {
        Value::Null => Ok(None),
        Value::Text(name) => Ok(Some(name.clone())),
        other => Err(format!("{what}: the actor is {}", other.describes())),
    }
}

fn only(fields: &[(String, Value)], allowed: &[&str], what: &str) -> Result<(), String> {
    match fields
        .iter()
        .find(|(name, _)| !allowed.contains(&name.as_str()))
    {
        Some((name, _)) => Err(format!(
            "{what}: the key `{name}` is not interpreted by this runner"
        )),
        None => Ok(()),
    }
}

fn object<'a>(value: &'a Value, what: &str) -> Result<&'a [(String, Value)], String> {
    match value {
        Value::Object(fields) => Ok(fields),
        other => Err(format!("{what} is {}, not an object", other.describes())),
    }
}

fn items<'a>(value: &'a Value, what: &str) -> Result<&'a [Value], String> {
    match value {
        Value::Array(items) => Ok(items),
        other => Err(format!("{what} is {}, not an array", other.describes())),
    }
}

fn member<'a>(fields: &'a [(String, Value)], name: &str, what: &str) -> Result<&'a Value, String> {
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
        .ok_or_else(|| format!("{what} has no `{name}`"))
}

fn as_text<'a>(value: &'a Value, what: &str) -> Result<&'a str, String> {
    match value {
        Value::Text(text) => Ok(text),
        other => Err(format!("{what} is {}, not a string", other.describes())),
    }
}

fn text_member<'a>(
    fields: &'a [(String, Value)],
    name: &str,
    what: &str,
) -> Result<&'a str, String> {
    as_text(member(fields, name, what)?, &format!("{what} `{name}`"))
}

fn finite(value: &Value, what: &str) -> Result<f64, String> {
    match value {
        Value::Number(spelling) => spelling
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())
            .ok_or_else(|| format!("{what} `{spelling}` is not a finite number")),
        other => Err(format!("{what} is {}, not a number", other.describes())),
    }
}

fn count(value: &Value, what: &str) -> Result<u64, String> {
    let number = finite(value, what)?;
    if number < 0.0 || number.fract() != 0.0 || number > 9.0e15 {
        return Err(format!("{what} is not a count of rows"));
    }
    Ok(number as u64)
}

fn index(value: &Value, what: &str) -> Result<usize, String> {
    usize::try_from(count(value, what)?).map_err(|_| format!("{what} is past any step"))
}
