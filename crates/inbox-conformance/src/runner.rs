//! Interprets the steps of a scenario against a [`Target`], and gives each scenario one terminal
//! status.

use std::collections::HashMap;

use inbox::server::json::Value;

use crate::suite::{Expectation, Fields, PathOp, Predicate, Ref, Scenario, Shape, Step, Suite};
use crate::value::{brief, exact_equal, render, view_equal};

/// What a target answered a command with.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    /// A declared outcome, and the declared error it carries, if any.
    Outcome {
        /// The outcome's name.
        outcome: String,
        /// The error's qualified name.
        error: Option<String>,
    },
    /// Refused before it ran: the caller is no actor (`None`) or one not granted the command.
    NotGranted(Option<String>),
    /// Refused with no declared outcome: an unknown name, an input the surface does not read, an
    /// unmet obligation. A step that expects anything of the answer fails on it.
    Refused(String),
    /// The target could not send the step at all; the scenario is an `error`, not a failure.
    Broken(String),
}

/// One published event: its qualified name and its payload.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// The event's qualified name.
    pub name: String,
    /// The payload, as the wire encodes it.
    pub payload: Value,
}

/// A command's answer and every event it published, in log order.
#[derive(Debug, Clone, PartialEq)]
pub struct Executed {
    /// The answer.
    pub answer: Answer,
    /// The events.
    pub events: Vec<Event>,
}

/// What a target answered a read with.
#[derive(Debug, Clone, PartialEq)]
pub enum Read {
    /// The rows.
    Rows(Vec<Value>),
    /// Refused before anything was read.
    NotGranted(Option<String>),
    /// Refused with no rows: see [`Answer::Refused`].
    Refused(String),
    /// The target could not send the read: see [`Answer::Broken`].
    Broken(String),
}

/// Whom a read is sent as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reader {
    /// No `read_as` step came before the read: the scenario reads the rows, and states no reader.
    /// The target sends it as an actor whose grant admits the view.
    Unstated,
    /// The actor the last `read_as` step named; `None` for no actor.
    As(Option<String>),
}

/// One command, as a step sends it.
#[derive(Debug, Clone, PartialEq)]
pub struct Command<'a> {
    /// The command's qualified name.
    pub name: &'a str,
    /// The actor the command is sent as; `None` for no actor.
    pub actor: Option<&'a str>,
    /// The caller's attributes, by name.
    pub caller: &'a [(String, String)],
    /// The input, every instance reference resolved.
    pub input: Value,
    /// The outcome the step expects: the target answers each `external:` branch of the command
    /// as taken exactly when it is this outcome.
    pub expected: Option<&'a str>,
}

/// The implementation the suite runs against, one scenario at a time.
pub trait Target {
    /// Forgets everything: the next scenario starts from an empty system.
    fn reset(&mut self);

    /// Sends one command.
    fn execute(&mut self, command: &Command<'_>) -> Executed;

    /// Reads one view with `params`, an object keyed by the parameters' names.
    fn query(&mut self, view: &str, reader: &Reader, params: Value) -> Read;
}

/// The terminal status of a scenario, as `ess-conformance-results/1` spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Every step held.
    Passed,
    /// The target answered a step wrongly.
    Failed,
    /// The runner or the target could not carry a step out.
    Error,
    /// The runner does not interpret a construct of the scenario; nothing of it ran.
    Unsupported,
}

impl Status {
    /// The status's name in the results document.
    pub fn name(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Error => "error",
            Self::Unsupported => "unsupported",
        }
    }
}

/// A scenario's status, and why where it did not pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioResult {
    /// The scenario's id.
    pub id: String,
    /// Its status.
    pub status: Status,
    /// Why it did not pass: the step and what the target answered.
    pub message: Option<String>,
}

/// Runs every scenario of `suite` against `target`, each from an empty system, in suite order.
pub fn run_suite(suite: &Suite, target: &mut dyn Target) -> Vec<ScenarioResult> {
    suite
        .scenarios
        .iter()
        .map(|scenario| run_scenario(scenario, target))
        .collect()
}

/// Runs one scenario from an empty system.
pub fn run_scenario(scenario: &Scenario, target: &mut dyn Target) -> ScenarioResult {
    let result = |status, message| ScenarioResult {
        id: scenario.id.clone(),
        status,
        message,
    };
    let steps = match &scenario.steps {
        Ok(steps) => steps,
        Err(reason) => return result(Status::Unsupported, Some(reason.clone())),
    };
    target.reset();
    let mut run = Run::default();
    for (at, step) in steps.iter().enumerate() {
        match run.step(target, steps, at, step) {
            Ok(()) => {}
            Err(Halt::Failed(reason)) => {
                return result(Status::Failed, Some(format!("step {at}: {reason}")));
            }
            Err(Halt::Error(reason)) => {
                return result(Status::Error, Some(format!("step {at}: {reason}")));
            }
        }
    }
    result(Status::Passed, None)
}

/// Why a scenario stopped.
enum Halt {
    Failed(String),
    Error(String),
}

fn failed<T>(reason: impl Into<String>) -> Result<T, Halt> {
    Err(Halt::Failed(reason.into()))
}

fn error<T>(reason: impl Into<String>) -> Result<T, Halt> {
    Err(Halt::Error(reason.into()))
}

/// The last command a scenario sent, and which of its events a claim has matched.
struct Sent {
    command: String,
    executed: Executed,
    claimed: Vec<bool>,
}

/// A row kept by a snapshot step.
struct Kept {
    subject: Vec<(String, Value)>,
    row: Option<Value>,
    complete: Option<Vec<String>>,
}

/// Which of the last command and the last read came later.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Latest {
    Command,
    Read,
}

/// What a scenario has done so far.
struct Run {
    instances: HashMap<String, Value>,
    sent: Option<Sent>,
    read: Option<(String, Read)>,
    latest: Option<Latest>,
    reader: Reader,
    forces: HashMap<String, String>,
    kept: HashMap<String, Kept>,
    published: Vec<Event>,
}

impl Default for Run {
    fn default() -> Self {
        Self {
            instances: HashMap::new(),
            sent: None,
            read: None,
            latest: None,
            reader: Reader::Unstated,
            forces: HashMap::new(),
            kept: HashMap::new(),
            published: Vec::new(),
        }
    }
}

impl Run {
    fn step(
        &mut self,
        target: &mut dyn Target,
        steps: &[Step],
        at: usize,
        step: &Step,
    ) -> Result<(), Halt> {
        match step {
            Step::Execute {
                command,
                actor,
                caller,
                input,
            } => {
                let expected = expected_outcome(steps, at, command);
                if let Some(forced) = self.forces.remove(command)
                    && expected != Some(forced.as_str())
                {
                    return error(format!(
                        "configure_external_outcome forces `{forced}` on `{command}`, and the step expects {}",
                        expected.map_or("no outcome".to_owned(), |outcome| format!("`{outcome}`"))
                    ));
                }
                let input = self.resolve(input)?;
                let executed = target.execute(&Command {
                    name: command,
                    actor: actor.as_deref(),
                    caller,
                    input,
                    expected,
                });
                if let Answer::Broken(reason) = &executed.answer {
                    return error(format!("the target could not send `{command}`: {reason}"));
                }
                self.published.extend(executed.events.iter().cloned());
                self.sent = Some(Sent {
                    command: command.clone(),
                    claimed: vec![false; executed.events.len()],
                    executed,
                });
                self.latest = Some(Latest::Command);
                Ok(())
            }
            Step::ExpectOutcome { command, outcome } => {
                let sent = self.sent(command)?;
                match &sent.executed.answer {
                    Answer::Outcome { outcome: got, .. } if got == outcome => Ok(()),
                    other => failed(format!(
                        "`{command}` should answer `{outcome}`; it answered {}",
                        describe(other)
                    )),
                }
            }
            Step::ExpectNoError => {
                let sent = self.last_sent()?;
                match &sent.executed.answer {
                    Answer::Outcome { error: None, .. } => Ok(()),
                    other => failed(format!(
                        "`{}` should answer no error; it answered {}",
                        sent.command,
                        describe(other)
                    )),
                }
            }
            Step::ExpectError { error: name } => {
                let sent = self.last_sent()?;
                match &sent.executed.answer {
                    Answer::Outcome {
                        error: Some(got), ..
                    } if got == name => Ok(()),
                    other => failed(format!(
                        "`{}` should answer the error `{name}`; it answered {}",
                        sent.command,
                        describe(other)
                    )),
                }
            }
            Step::ExpectEvent {
                event,
                payload,
                shape,
            } => {
                let claim = Value::Object(self.resolve_fields(payload)?);
                let sent = self.last_sent_mut()?;
                let found = sent.executed.events.iter().zip(&sent.claimed).position(
                    |(published, claimed)| {
                        !claimed
                            && published.name == *event
                            && exact_equal(&published.payload, &claim)
                    },
                );
                let Some(found) = found else {
                    return failed(format!(
                        "`{}` should publish `{event}` {}; it published {}",
                        sent.command,
                        render(&claim),
                        events(&sent.executed.events)
                    ));
                };
                sent.claimed[found] = true;
                if let Some(shape) = shape {
                    conforms(&sent.executed.events[found], shape)?;
                }
                Ok(())
            }
            Step::ExpectNoEvent { event } => {
                let sent = self.last_sent()?;
                if sent
                    .executed
                    .events
                    .iter()
                    .any(|published| published.name == *event)
                {
                    return failed(format!(
                        "`{}` should not publish `{event}`; it published {}",
                        sent.command,
                        events(&sent.executed.events)
                    ));
                }
                Ok(())
            }
            Step::ExpectNoEvents => {
                let sent = self.last_sent()?;
                if !sent.executed.events.is_empty() {
                    return failed(format!(
                        "`{}` should publish nothing; it published {}",
                        sent.command,
                        events(&sent.executed.events)
                    ));
                }
                Ok(())
            }
            Step::ExpectNotGranted { actor } => {
                let refused = match self.latest {
                    Some(Latest::Command) => {
                        self.last_sent().map(|sent| match &sent.executed.answer {
                            Answer::NotGranted(got) => Ok(got.clone()),
                            other => Err(describe(other)),
                        })?
                    }
                    Some(Latest::Read) => match &self.read {
                        Some((_, Read::NotGranted(got))) => Ok(got.clone()),
                        Some((_, other)) => Err(describe_read(other)),
                        None => return error("no view was read"),
                    },
                    None => return error("no command was sent and no view was read"),
                };
                match refused {
                    Ok(got) if got == *actor => Ok(()),
                    Ok(got) => failed(format!(
                        "the refusal should name {}; it names {}",
                        actor_name(actor.as_deref()),
                        actor_name(got.as_deref())
                    )),
                    Err(answered) => failed(format!(
                        "the request should be refused as not granted to {}; it answered {answered}",
                        actor_name(actor.as_deref())
                    )),
                }
            }
            Step::Capture {
                instance,
                event,
                field,
            } => {
                let sent = self.last_sent()?;
                let mut matching = sent
                    .executed
                    .events
                    .iter()
                    .filter(|published| published.name == *event);
                let (Some(first), None) = (matching.next(), matching.next()) else {
                    let count = sent
                        .executed
                        .events
                        .iter()
                        .filter(|published| published.name == *event)
                        .count();
                    if count == 0 {
                        return failed(format!(
                            "`{}` should publish `{event}` to capture `{instance}` from; it published {}",
                            sent.command,
                            events(&sent.executed.events)
                        ));
                    }
                    return error(format!(
                        "`{}` published `{event}` {count} times: which one `{instance}` names is not decided",
                        sent.command
                    ));
                };
                let Some(value) = first.payload.member(field) else {
                    return failed(format!(
                        "`{event}` carries no `{field}` to capture `{instance}` from: {}",
                        render(&first.payload)
                    ));
                };
                self.instances.insert(instance.clone(), value.clone());
                Ok(())
            }
            Step::ConfigureExternal { command, outcome } => {
                self.forces.insert(command.clone(), outcome.clone());
                Ok(())
            }
            Step::ReadAs { actor } => {
                self.reader = Reader::As(actor.clone());
                Ok(())
            }
            Step::Query { view, params } => {
                let params = self.resolve(params)?;
                let read = target.query(view, &self.reader, params);
                if let Read::Broken(reason) = &read {
                    return error(format!("the target could not read `{view}`: {reason}"));
                }
                self.read = Some((view.clone(), read));
                self.latest = Some(Latest::Read);
                Ok(())
            }
            Step::ExpectView { view, expectation } => {
                let rows = self.rows(view)?;
                self.expect(view, rows, expectation)
            }
            Step::Snapshot {
                view,
                subject,
                complete,
            } => {
                let subject = self.resolve_fields(subject)?;
                let rows = self.rows(view)?;
                let row = subject_row(view, rows, &subject)?.cloned();
                self.kept.insert(
                    view.clone(),
                    Kept {
                        subject,
                        row,
                        complete: complete.clone(),
                    },
                );
                Ok(())
            }
            Step::ExpectUnchanged { view, complete } => {
                let rows = self.rows(view)?;
                let Some(kept) = self.kept.get(view) else {
                    return error(format!("no snapshot of `{view}` was taken"));
                };
                if kept.complete.is_some() != *complete {
                    return error(format!(
                        "the snapshot of `{view}` and this step disagree on whether it is complete"
                    ));
                }
                let now = subject_row(view, rows, &kept.subject)?;
                let same = match (&kept.row, now) {
                    (None, None) => true,
                    (Some(before), Some(now)) => match &kept.complete {
                        Some(fields) => fields
                            .iter()
                            .all(|field| view_equal(before.member(field), now.member(field))),
                        None => view_equal(Some(before), Some(now)),
                    },
                    _ => false,
                };
                if same {
                    return Ok(());
                }
                failed(format!(
                    "the row of `{view}` the subject {} names changed: it was {}, it is {}",
                    render(&Value::Object(kept.subject.clone())),
                    kept.row.as_ref().map_or("absent".to_owned(), render),
                    now.map_or("absent".to_owned(), render)
                ))
            }
        }
    }

    fn expect(&self, view: &str, rows: &[Value], expectation: &Expectation) -> Result<(), Halt> {
        match expectation {
            Expectation::Contains(fields) => {
                let claim = self.resolve_fields(fields)?;
                if rows.iter().any(|row| holds(row, &claim)) {
                    return Ok(());
                }
                failed(format!(
                    "no row of `{view}` holds {}; its rows: {}",
                    render(&Value::Object(claim)),
                    brief(&Value::Array(rows.to_vec()), 1500)
                ))
            }
            Expectation::Excludes(fields) => {
                let claim = self.resolve_fields(fields)?;
                match rows.iter().find(|row| holds(row, &claim)) {
                    None => Ok(()),
                    Some(row) => failed(format!(
                        "`{view}` should hold no row with {}; it holds {}",
                        render(&Value::Object(claim)),
                        render(row)
                    )),
                }
            }
            Expectation::Counts { at_least, at_most } => {
                let count = rows.len() as u64;
                let low = at_least.is_none_or(|least| count >= least);
                let high = at_most.is_none_or(|most| count <= most);
                if low && high {
                    return Ok(());
                }
                failed(format!(
                    "`{view}` should hold {} rows; it holds {count}: {}",
                    bounds(*at_least, *at_most),
                    brief(&Value::Array(rows.to_vec()), 1500)
                ))
            }
            Expectation::Satisfies(predicate) => {
                for row in rows {
                    if let Err(reason) = satisfies(row, predicate) {
                        return failed(format!("a row of `{view}` {reason}: {}", render(row)));
                    }
                }
                Ok(())
            }
            Expectation::ValueInvariants(check) => {
                let Some(witness) = self
                    .published
                    .iter()
                    .rev()
                    .find(|published| published.name == check.event)
                else {
                    return error(format!(
                        "no `{}` was published to name the witness row",
                        check.event
                    ));
                };
                let Some(identity) = witness.payload.member(&check.field) else {
                    return failed(format!(
                        "`{}` carries no `{}`: {}",
                        check.event,
                        check.field,
                        render(&witness.payload)
                    ));
                };
                let named: Vec<&Value> = rows
                    .iter()
                    .filter(|row| view_equal(Some(identity), row.member(&check.identity)))
                    .collect();
                let [row] = named.as_slice() else {
                    return failed(format!(
                        "`{view}` should hold one row with `{}` {}; it holds {}",
                        check.identity,
                        render(identity),
                        named.len()
                    ));
                };
                let mut values = Vec::new();
                reach(row.member(&check.root), &check.path, &mut values).map_err(|reason| {
                    Halt::Failed(format!(
                        "`{view}` at `{}` of row {}: {reason}",
                        check.root,
                        render(row)
                    ))
                })?;
                for value in values {
                    let Value::Text(text) = value else {
                        return failed(format!(
                            "`{view}` publishes {} where `{}` holds text",
                            render(value),
                            check.value_type
                        ));
                    };
                    let scalars = text.chars().count() as f64;
                    if let Some((op, bound)) = check
                        .counts
                        .iter()
                        .find(|(op, bound)| !op.holds(scalars, *bound))
                    {
                        return failed(format!(
                            "`{view}` publishes {} at `{}`, and `{}` declares `value.count {op:?} {bound}`",
                            render(value),
                            check.root,
                            check.value_type
                        ));
                    }
                    if let Some(alphabet) = &check.alphabet
                        && let Some(stray) = text
                            .chars()
                            .find(|character| !alphabet.contains(*character))
                    {
                        return failed(format!(
                            "`{view}` publishes {} at `{}`, and `{stray}` is outside the alphabet of `{}`",
                            render(value),
                            check.root,
                            check.value_type
                        ));
                    }
                }
                Ok(())
            }
        }
    }

    fn resolve_fields(&self, fields: &Fields) -> Result<Vec<(String, Value)>, Halt> {
        fields
            .iter()
            .map(|(name, reference)| {
                let value = match reference {
                    Ref::Literal(value) => value.clone(),
                    Ref::Instance(instance) => match self.instances.get(instance) {
                        Some(value) => value.clone(),
                        None => {
                            return error(format!("the instance `{instance}` was never captured"));
                        }
                    },
                };
                Ok((name.clone(), value))
            })
            .collect()
    }

    fn resolve(&self, fields: &Fields) -> Result<Value, Halt> {
        self.resolve_fields(fields).map(Value::Object)
    }

    fn last_sent(&self) -> Result<&Sent, Halt> {
        match &self.sent {
            Some(sent) => Ok(sent),
            None => error("no command was sent"),
        }
    }

    fn last_sent_mut(&mut self) -> Result<&mut Sent, Halt> {
        match &mut self.sent {
            Some(sent) => Ok(sent),
            None => error("no command was sent"),
        }
    }

    fn sent(&self, command: &str) -> Result<&Sent, Halt> {
        let sent = self.last_sent()?;
        if sent.command != command {
            return error(format!(
                "the step expects an answer of `{command}`; the last command sent was `{}`",
                sent.command
            ));
        }
        Ok(sent)
    }

    fn rows(&self, view: &str) -> Result<&[Value], Halt> {
        match &self.read {
            None => error(format!("`{view}` was not read")),
            Some((read, _)) if read != view => error(format!(
                "the step expects rows of `{view}`; the last view read was `{read}`"
            )),
            Some((_, Read::Rows(rows))) => Ok(rows),
            Some((_, other)) => failed(format!(
                "`{view}` should answer rows; it answered {}",
                describe_read(other)
            )),
        }
    }
}

/// The outcome the step's own `expect_outcome` names, looked up before the next command.
fn expected_outcome<'a>(steps: &'a [Step], at: usize, command: &str) -> Option<&'a str> {
    steps[at + 1..]
        .iter()
        .take_while(|step| !matches!(step, Step::Execute { .. }))
        .find_map(|step| match step {
            Step::ExpectOutcome {
                command: named,
                outcome,
            } if named == command => Some(outcome.as_str()),
            _ => None,
        })
}

/// Whether `row` holds every claimed field, as a view assertion reads it.
fn holds(row: &Value, claim: &[(String, Value)]) -> bool {
    claim
        .iter()
        .all(|(name, value)| view_equal(Some(value), row.member(name)))
}

/// The row of `rows` the subject names, if one does.
fn subject_row<'a>(
    view: &str,
    rows: &'a [Value],
    subject: &[(String, Value)],
) -> Result<Option<&'a Value>, Halt> {
    let mut named = rows.iter().filter(|row| holds(row, subject));
    match (named.next(), named.next()) {
        (row, None) => Ok(row),
        _ => failed(format!(
            "`{view}` holds several rows the subject {} names",
            render(&Value::Object(subject.to_vec()))
        )),
    }
}

/// Collects every value at `path`, starting from `value`.
fn reach<'a>(
    value: Option<&'a Value>,
    path: &[PathOp],
    out: &mut Vec<&'a Value>,
) -> Result<(), String> {
    let present = value.filter(|value| **value != Value::Null);
    match path.split_first() {
        None => match present {
            Some(value) => {
                out.push(value);
                Ok(())
            }
            None => Err("a value the type requires is absent".to_owned()),
        },
        Some((PathOp::Present, rest)) => match present {
            Some(value) => reach(Some(value), rest, out),
            None => Ok(()),
        },
        Some((PathOp::Field(name), rest)) => match present {
            Some(Value::Object(_)) => {
                reach(present.and_then(|value| value.member(name)), rest, out)
            }
            Some(other) => Err(format!(
                "`{name}` is read from {}, not a struct",
                render(other)
            )),
            None => Err(format!("the struct holding `{name}` is absent")),
        },
        Some((PathOp::Element, rest)) => match present {
            Some(Value::Array(items)) => items
                .iter()
                .try_for_each(|item| reach(Some(item), rest, out)),
            Some(other) => Err(format!("{} is not a list", render(other))),
            None => Err("the list is absent".to_owned()),
        },
    }
}

/// `Ok` where `row` satisfies `predicate`; otherwise how it does not.
fn satisfies(row: &Value, predicate: &Predicate) -> Result<(), String> {
    let (field, op, bound, measured) = match predicate {
        Predicate::Count { field, op, bound } => {
            let measured = match row.member(field) {
                Some(Value::Text(text)) => text.chars().count(),
                Some(Value::Array(items)) => items.len(),
                other => {
                    return Err(format!(
                        "has no text or list at `{field}` to count: {}",
                        other.map_or("absent".to_owned(), render)
                    ));
                }
            };
            (field, op, bound, measured)
        }
        Predicate::Utf8Bytes { field, op, bound } => {
            let measured = match row.member(field) {
                Some(Value::Text(text)) => text.len(),
                other => {
                    return Err(format!(
                        "has no text at `{field}` to measure: {}",
                        other.map_or("absent".to_owned(), render)
                    ));
                }
            };
            (field, op, bound, measured)
        }
    };
    if op.holds(measured as f64, *bound) {
        return Ok(());
    }
    Err(format!(
        "breaks `{field}` {op:?} {bound}: it measures {measured}"
    ))
}

/// `Ok` where the event's payload holds exactly the shape's members, each of its declared type.
fn conforms(event: &Event, shape: &[(String, Shape)]) -> Result<(), Halt> {
    let Value::Object(members) = &event.payload else {
        return failed(format!(
            "the payload of `{}` is {}, not an object",
            event.name,
            render(&event.payload)
        ));
    };
    for (name, value) in members {
        let Some((_, declared)) = shape.iter().find(|(field, _)| field == name) else {
            return failed(format!(
                "the payload of `{}` carries `{name}`, which its shape does not declare",
                event.name
            ));
        };
        let fits = match (declared, value) {
            (Shape::Text, Value::Text(_)) => true,
            (Shape::Enum(variants), Value::Text(variant)) => variants.contains(variant),
            _ => false,
        };
        if !fits {
            return failed(format!(
                "the payload of `{}` carries {} at `{name}`, which is not a {declared:?}",
                event.name,
                render(value)
            ));
        }
    }
    if let Some((name, _)) = shape
        .iter()
        .find(|(name, _)| !members.iter().any(|(member, _)| member == name))
    {
        return failed(format!(
            "the payload of `{}` lacks `{name}`, which its shape declares",
            event.name
        ));
    }
    Ok(())
}

fn describe(answer: &Answer) -> String {
    match answer {
        Answer::Outcome {
            outcome,
            error: Some(error),
        } => format!("`{outcome}` with the error `{error}`"),
        Answer::Outcome {
            outcome,
            error: None,
        } => format!("`{outcome}`"),
        Answer::NotGranted(actor) => format!("not granted to {}", actor_name(actor.as_deref())),
        Answer::Refused(reason) => format!("a refusal with no declared outcome ({reason})"),
        Answer::Broken(reason) => format!("nothing ({reason})"),
    }
}

fn describe_read(read: &Read) -> String {
    match read {
        Read::Rows(rows) => format!("{} rows", rows.len()),
        Read::NotGranted(actor) => format!("not granted to {}", actor_name(actor.as_deref())),
        Read::Refused(reason) => format!("a refusal with no rows ({reason})"),
        Read::Broken(reason) => format!("nothing ({reason})"),
    }
}

fn actor_name(actor: Option<&str>) -> String {
    actor.map_or("no actor".to_owned(), |actor| format!("`{actor}`"))
}

fn events(events: &[Event]) -> String {
    if events.is_empty() {
        return "nothing".to_owned();
    }
    events
        .iter()
        .map(|event| format!("`{}` {}", event.name, render(&event.payload)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn bounds(at_least: Option<u64>, at_most: Option<u64>) -> String {
    match (at_least, at_most) {
        (Some(least), Some(most)) if least == most => format!("exactly {least}"),
        (Some(least), Some(most)) => format!("{least} to {most}"),
        (Some(least), None) => format!("at least {least}"),
        (None, Some(most)) => format!("at most {most}"),
        (None, None) => "any number of".to_owned(),
    }
}
