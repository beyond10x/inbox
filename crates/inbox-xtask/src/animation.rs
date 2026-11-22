//! The data of the landing page's animation and pitch graphics, written from the compiled
//! specification to `website/src/generated/animation.json`.
//!
//! It compiles `ess/` with `ess specify compile --format json` (or reads the compiled IR from a
//! file), builds the story of one message through the inbox from that IR, checks it, and only then
//! writes the file. A failed check changes no file. With `--check` it writes nothing and exits 1
//! when the file differs from what it would write, printing one line: `edited:` or `missing:`.
//!
//! Every name the data carries (commands, views, errors, states, actors, the entities) is read
//! from the IR. This file holds only layout, timing, the vocabulary of the IR format and the rules
//! that select from the IR:
//!
//! - the message entity is the one entity whose lifecycle declares terminal states; the inbox
//!   entity is the entity it references;
//! - the creating actor is the actor that may send the command that creates a message;
//! - the first journey walks the longest path of the creating actor's transitions from the initial
//!   state to the first terminal state, each further journey the shortest path to the next
//!   terminal state; each journey ends with the creating actor's command that deletes a message in
//!   that state;
//! - each command on the message entity that another actor sends gets a journey of its own, with
//!   its default (`otherwise`) outcome;
//! - the command that creates the inbox opens the story, the command of the transition that leaves
//!   the inbox's initial state closes it;
//! - a command is the recipient's when it acts on the inbox entity, or when one of its refusals
//!   decided by the caller is `<reference field> != caller.<attribute>`; otherwise it is the
//!   sender's;
//! - the views of a step are those its actor may read, over the step's entity, whose filter admits
//!   the state after the step (before it, for a step that deletes);
//! - the refusals of a step are the errors its command declares; every error of the IR is shown
//!   once, at a step whose command declares it, scarcest first.
//!
//! The story fails loudly when a command, a view or an error of the IR has no place in it, or when
//! a frame shows more than `WORD_LIMIT` words.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Map, Value, json};

use crate::Answer;

/// The file, relative to the repository root.
pub const OUT: &str = "website/src/generated/animation.json";
const WORD_LIMIT: usize = 12;
const FRAME_MS: u64 = 2600;
const LAYOUT: [(&str, &str); 4] = [
    ("host", "host"),
    ("planned", "planned"),
    ("process", "process"),
    ("service", "inbox"),
];
const LAYOUT_TEXTS: [&str; 5] = ["host", "host", "process", "inbox", "planned"];
const PLANNED_NOTE: &str = "The second host and the link to it are planned: today the service listens on loopback only, so every agent that uses it runs on the service's own host.";
/// The note the file carries about who writes it.
const GENERATED: &str = "Written by `task animation` from the compiled ess/. Do not edit.";

static STATE_IS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^state == (\w+)$").expect("the state pattern compiles"));

type Story<T> = Result<T, String>;

/// Write the file from `ir` (the compiled IR; `None` compiles `ess/`), or with `check` compare it.
pub fn animation(root: &Path, ir: Option<&Path>, check: bool) -> Result<Answer, String> {
    let story = match ir {
        Some(file) => fs::read_to_string(file)
            .map_err(|e| format!("read {}: {e}", file.display()))
            .and_then(|text| {
                serde_json::from_str::<Value>(&text)
                    .map_err(|e| format!("read {}: {e}", file.display()))
            }),
        None => compile_ir(root),
    }
    .and_then(|ir| build(&ir))
    .map(|data| render(&data));
    let text = match story {
        Ok(text) => text,
        Err(error) => {
            return Ok(Answer {
                stderr: vec![format!("animation: {error}")],
                code: 1,
                ..Answer::default()
            });
        }
    };
    let path = root.join(OUT);
    if check {
        let line = match fs::read(&path) {
            Err(_) if !path.exists() => Some(format!("missing: {OUT}")),
            Ok(held) if held == text.as_bytes() => None,
            _ => Some(format!("edited: {OUT}")),
        };
        return Ok(Answer {
            code: u8::from(line.is_some()),
            stdout: line.into_iter().collect(),
            ..Answer::default()
        });
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    fs::write(&path, text).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(Answer::default())
}

/// The data as the file holds it: keys sorted, 2 spaces of indent, non-ASCII text as it is.
pub fn render(data: &Value) -> String {
    let mut text = serde_json::to_string_pretty(data).expect("a JSON value renders");
    text.push('\n');
    text
}

fn compile_ir(root: &Path) -> Story<Value> {
    let run = Command::new("ess")
        .args(["specify", "compile", "--path", "ess", "--format", "json"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("run ess: {e}"))?;
    if !run.status.success() {
        return Err(format!(
            "ess specify compile failed:\n{}",
            String::from_utf8_lossy(&run.stderr).trim()
        ));
    }
    serde_json::from_slice(&run.stdout).map_err(|e| format!("read the compiled IR: {e}"))
}

fn short(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

/// The text with its backquotes dropped and its whitespace runs made one space; "" for no text.
fn plain(text: Option<&Value>) -> String {
    text.and_then(Value::as_str)
        .unwrap_or("")
        .replace('`', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// What a value counts as in a condition: absent, null, false, 0, "" and an empty list or object
/// count as false.
fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64() != Some(0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
    }
}

/// `value[key]` of an object; `None` when absent or null, or when `value` is no object.
fn get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.get(key).filter(|v| !v.is_null())
}

fn need<'a>(value: &'a Value, key: &str, of: &str) -> Story<&'a Value> {
    value.get(key).ok_or_else(|| format!("{of} has no `{key}`"))
}

fn text<'a>(value: &'a Value, key: &str, of: &str) -> Story<&'a str> {
    need(value, key, of)?
        .as_str()
        .ok_or_else(|| format!("`{key}` of {of} is no text"))
}

fn object<'a>(value: &'a Value, key: &str, of: &str) -> Story<&'a Map<String, Value>> {
    need(value, key, of)?
        .as_object()
        .ok_or_else(|| format!("`{key}` of {of} is no object"))
}

fn list(value: Option<&Value>) -> &[Value] {
    value.and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

/// A list as the story's messages print it: `['a', 'b']`.
fn shown(items: &[String]) -> String {
    let quoted: Vec<String> = items.iter().map(|i| format!("'{i}'")).collect();
    format!("[{}]", quoted.join(", "))
}

fn one<T: Clone>(items: &[T], what: &str, show: impl Fn(&T) -> String) -> Story<T> {
    match items {
        [item] => Ok(item.clone()),
        _ => {
            let mut names: Vec<String> = items.iter().map(show).collect();
            names.sort();
            Err(format!(
                "expected exactly one {what}, found {}: {}",
                items.len(),
                shown(&names)
            ))
        }
    }
}

fn one_name(items: &[String], what: &str) -> Story<String> {
    one(items, what, Clone::clone)
}

fn successes(command: &Value) -> Vec<&Value> {
    list(command.get("outcomes"))
        .iter()
        .filter(|o| o.get("error").is_none())
        .collect()
}

fn subject_entity(name: &str, command: &Value) -> Story<String> {
    let mut entities = BTreeSet::new();
    for outcome in successes(command) {
        if truthy(outcome.get("subject")) {
            entities.insert(text(&outcome["subject"], "entity", "a subject")?.to_string());
        }
    }
    let entities: Vec<String> = entities.into_iter().collect();
    one_name(&entities, &format!("subject entity of {name}"))
}

fn refusals(command: &Value) -> Story<Vec<String>> {
    let mut seen: Vec<String> = Vec::new();
    for outcome in list(command.get("outcomes")) {
        let name = outcome.get("error");
        if truthy(name) {
            let name = name
                .and_then(Value::as_str)
                .ok_or("an outcome's `error` is no text")?;
            if !seen.iter().any(|s| s == short(name)) {
                seen.push(short(name).to_string());
            }
        }
    }
    Ok(seen)
}

fn admits(predicate: Option<&Value>, state: Option<&str>) -> bool {
    match predicate {
        None => true,
        Some(Value::String(text)) => match STATE_IS.captures(text.trim()) {
            Some(found) => Some(&found[1]) == state,
            None => true,
        },
        Some(Value::Object(object)) => {
            if let Some(all) = object.get("all") {
                list(Some(all)).iter().all(|p| admits(get_value(p), state))
            } else if let Some(any) = object.get("any") {
                list(Some(any)).iter().any(|p| admits(get_value(p), state))
            } else {
                true
            }
        }
        Some(_) => true,
    }
}

fn get_value(value: &Value) -> Option<&Value> {
    (!value.is_null()).then_some(value)
}

fn is_in(state: Option<&str>, states: &[Value]) -> bool {
    state.is_some_and(|s| states.iter().any(|v| v.as_str() == Some(s)))
}

struct Spec<'a> {
    commands: &'a Map<String, Value>,
    views: &'a Map<String, Value>,
    errors: &'a Map<String, Value>,
    actors: &'a Map<String, Value>,
    message: String,
    lifecycle: &'a Value,
    inbox: String,
    via: String,
    inbox_lifecycle: &'a Value,
    actor_of: BTreeMap<String, String>,
    create_message: String,
    create_inbox: String,
    creator: String,
}

type Edge = (String, String, String, String);

impl<'a> Spec<'a> {
    fn new(ir: &'a Value) -> Story<Self> {
        let commands = object(ir, "commands", "the IR")?;
        let views = object(ir, "views", "the IR")?;
        let errors = object(ir, "errors", "the IR")?;
        let actors = object(ir, "actors", "the IR")?;
        let entities = object(ir, "entities", "the IR")?;
        let terminal: Vec<String> = entities
            .iter()
            .filter(|(_, e)| truthy(get(e, "lifecycle").and_then(|l| l.get("terminal"))))
            .map(|(n, _)| n.clone())
            .collect();
        let message = one_name(&terminal, "entity with terminal states")?;
        let lifecycle = need(&entities[&message], "lifecycle", &message)?;
        let mut references = Vec::new();
        for relation in list(get(&entities[&message], "relations")) {
            if relation.get("kind").and_then(Value::as_str) != Some("references") {
                continue;
            }
            let target = text(relation, "target", "a relation")?;
            if truthy(entities.get(target).and_then(|e| e.get("lifecycle"))) {
                references.push(relation);
            }
        }
        let relation = one(
            &references,
            &format!("referenced entity with a lifecycle of {message}"),
            |r| r.to_string(),
        )?;
        let inbox = text(relation, "target", "a relation")?.to_string();
        let via = text(relation, "via", "a relation")?.to_string();
        let inbox_lifecycle = need(&entities[&inbox], "lifecycle", &inbox)?;
        let mut actor_of = BTreeMap::new();
        for name in commands.keys() {
            let may: Vec<String> = actors
                .iter()
                .filter(|(_, actor)| {
                    list(actor.get("may"))
                        .iter()
                        .any(|m| m.as_str() == Some(name.as_str()))
                })
                .map(|(a, _)| a.clone())
                .collect();
            actor_of.insert(
                name.clone(),
                one_name(&may, &format!("actor that may send {name}"))?,
            );
        }
        let mut spec = Spec {
            commands,
            views,
            errors,
            actors,
            message,
            lifecycle,
            inbox,
            via,
            inbox_lifecycle,
            actor_of,
            create_message: String::new(),
            create_inbox: String::new(),
            creator: String::new(),
        };
        spec.create_message = spec.creating(&spec.message)?;
        spec.create_inbox = spec.creating(&spec.inbox)?;
        spec.creator = spec.actor_of[&spec.create_message].clone();
        Ok(spec)
    }

    fn command(&self, name: &str) -> &'a Value {
        &self.commands[name]
    }

    fn creating(&self, entity: &str) -> Story<String> {
        let found: Vec<String> = self
            .commands
            .iter()
            .filter(|(_, c)| {
                successes(c).iter().any(|o| {
                    get(o, "subject")
                        .and_then(|s| s.get("effect"))
                        .and_then(Value::as_str)
                        == Some("creates")
                        && o["subject"].get("entity").and_then(Value::as_str) == Some(entity)
                })
            })
            .map(|(n, _)| n.clone())
            .collect();
        one_name(&found, &format!("command that creates {entity}"))
    }

    fn transition_command(&self, entity: &str, transition: &str) -> Story<(String, String)> {
        let mut found = Vec::new();
        for (name, command) in self.commands {
            for outcome in successes(command) {
                let subject = get(outcome, "subject");
                let on = subject
                    .and_then(|s| s.get("entity"))
                    .and_then(Value::as_str);
                let named = subject
                    .and_then(|s| get(s, "transition"))
                    .and_then(|t| t.get("name"))
                    .and_then(Value::as_str);
                if on == Some(entity) && named == Some(transition) {
                    found.push((
                        name.clone(),
                        text(outcome, "name", "an outcome")?.to_string(),
                    ));
                }
            }
        }
        one(
            &found,
            &format!("command of the transition {transition} of {entity}"),
            |(c, o)| format!("('{c}', '{o}')"),
        )
    }

    fn creator_graph(&self) -> Story<Vec<Edge>> {
        let mut edges = Vec::new();
        for transition in list(self.lifecycle.get("transitions")) {
            let name = text(transition, "name", "a transition")?;
            let (command, outcome) = self.transition_command(&self.message, name)?;
            if self.actor_of[&command] == self.creator {
                let to = text(transition, "to", "a transition")?;
                for source in list(transition.get("from")) {
                    let source = source.as_str().ok_or("a transition's `from` is no text")?;
                    edges.push((
                        source.to_string(),
                        to.to_string(),
                        command.clone(),
                        outcome.clone(),
                    ));
                }
            }
        }
        Ok(edges)
    }

    /// Every path of the creating actor from the initial state to `target`, each a list of
    /// (state, command, outcome), in the order a depth-first walk over the edges finds them.
    fn paths(&self, target: &str) -> Story<Vec<Vec<(String, String, String)>>> {
        let edges = self.creator_graph()?;
        let initial = text(self.lifecycle, "initial", "the message lifecycle")?;
        let mut found = Vec::new();
        let mut path = Vec::new();
        let mut visited = vec![initial.to_string()];
        walk(&edges, initial, target, &mut path, &mut visited, &mut found);
        if found.is_empty() {
            return Err(format!(
                "no path of {} from {initial} to {target}",
                short(&self.creator)
            ));
        }
        Ok(found)
    }

    fn deleting(&self, state: &str) -> Story<(String, String)> {
        let mut found = Vec::new();
        for (name, command) in self.commands {
            if self.actor_of[name] != self.creator {
                continue;
            }
            for outcome in successes(command) {
                let subject = get(outcome, "subject");
                let condition = need(outcome, "condition", "an outcome")?;
                let states: Vec<Value> = match condition.get("state") {
                    Some(Value::String(s)) => vec![Value::String(s.clone())],
                    other => list(other).to_vec(),
                };
                let deletes = subject
                    .and_then(|s| s.get("effect"))
                    .and_then(Value::as_str)
                    == Some("deletes")
                    && subject
                        .and_then(|s| s.get("entity"))
                        .and_then(Value::as_str)
                        == Some(self.message.as_str());
                if deletes
                    && condition.get("kind").and_then(Value::as_str) == Some("subject_state")
                    && is_in(Some(state), &states)
                {
                    found.push((
                        name.clone(),
                        text(outcome, "name", "an outcome")?.to_string(),
                    ));
                }
            }
        }
        one(
            &found,
            &format!(
                "command of {} that deletes a message in {state}",
                short(&self.creator)
            ),
            |(c, o)| format!("('{c}', '{o}')"),
        )
    }

    fn role(&self, command: &str) -> Story<&'static str> {
        if self.actor_of[command] != self.creator {
            return Ok("service");
        }
        if subject_entity(command, self.command(command))? == self.inbox {
            return Ok("recipient");
        }
        let mut owned = BTreeSet::new();
        for attribute in list(self.actors[&self.creator].get("attributes")) {
            let name = text(attribute, "name", "an attribute")?;
            owned.insert(format!("{} != caller.{name}", self.via));
        }
        for outcome in list(self.command(command).get("outcomes")) {
            let predicate = need(outcome, "condition", "an outcome")?.get("predicate");
            if truthy(outcome.get("decided_by_caller"))
                && predicate
                    .and_then(Value::as_str)
                    .is_some_and(|p| owned.contains(p.trim()))
            {
                return Ok("recipient");
            }
        }
        Ok("sender")
    }

    fn views_for(&self, command: &str, entity: &str, state: Option<&str>) -> Vec<String> {
        let actor = &self.actors[&self.actor_of[command]];
        let readable: BTreeSet<&str> = list(actor.get("may_read"))
            .iter()
            .filter_map(Value::as_str)
            .collect();
        self.views
            .iter()
            .filter(|(n, v)| {
                readable.contains(n.as_str())
                    && v.get("source").and_then(Value::as_str) == Some(entity)
                    && admits(get(v, "filter"), state)
            })
            .map(|(n, _)| short(n).to_string())
            .collect()
    }
}

fn walk(
    edges: &[Edge],
    state: &str,
    target: &str,
    path: &mut Vec<(String, String, String)>,
    visited: &mut Vec<String>,
    found: &mut Vec<Vec<(String, String, String)>>,
) {
    if state == target {
        found.push(path.clone());
        return;
    }
    for (source, to, command, outcome) in edges {
        if source == state && !visited.contains(to) {
            path.push((to.clone(), command.clone(), outcome.clone()));
            visited.push(to.clone());
            walk(edges, to, target, path, visited, found);
            visited.pop();
            path.pop();
        }
    }
}

struct Step {
    actor: &'static str,
    command: String,
    entity: String,
    journey: u64,
    removed: bool,
    state: Option<String>,
    token: Option<&'static str>,
    views: Vec<String>,
}

/// The outcome of `command` named `outcome`.
fn outcome_named<'a>(command: &'a Value, outcome: &str) -> Story<&'a Value> {
    list(command.get("outcomes"))
        .iter()
        .find(|o| o.get("name").and_then(Value::as_str) == Some(outcome))
        .ok_or_else(|| format!("no outcome {outcome}"))
}

fn creating_outcome(command: &Value) -> Story<String> {
    let names: Vec<String> = successes(command)
        .iter()
        .filter(|o| {
            get(o, "subject")
                .and_then(|s| s.get("effect"))
                .and_then(Value::as_str)
                == Some("creates")
        })
        .filter_map(|o| o.get("name").and_then(Value::as_str).map(str::to_string))
        .collect();
    one_name(&names, "creating outcome")
}

fn step(
    spec: &Spec<'_>,
    steps: &mut Vec<Step>,
    command: &str,
    outcome: &str,
    journey: u64,
    state_before: Option<String>,
) -> Story<Option<String>> {
    let entity = subject_entity(command, spec.command(command))?;
    let result = outcome_named(spec.command(command), outcome)?;
    let subject = get(result, "subject");
    let effect = subject
        .and_then(|s| s.get("effect"))
        .and_then(Value::as_str);
    let role = spec.role(command)?;
    let terminal = list(spec.lifecycle.get("terminal"));
    let state = match effect {
        Some("creates") => {
            let lifecycle = if entity == spec.message {
                spec.lifecycle
            } else {
                spec.inbox_lifecycle
            };
            lifecycle
                .get("initial")
                .and_then(Value::as_str)
                .map(str::to_string)
        }
        Some("moves") => subject
            .and_then(|s| s.get("transition"))
            .and_then(|t| t.get("to"))
            .and_then(Value::as_str)
            .map(str::to_string),
        _ => state_before,
    };
    let removed = effect == Some("deletes");
    let token = if entity == spec.inbox {
        None
    } else if removed {
        Some(if role == "sender" { "sender" } else { "inbox" })
    } else if effect == Some("creates") || is_in(state.as_deref(), terminal) {
        Some("inbox")
    } else {
        Some(role)
    };
    let views = spec.views_for(command, &entity, state.as_deref());
    steps.push(Step {
        actor: role,
        command: command.to_string(),
        entity,
        journey,
        removed,
        state: state.clone(),
        token,
        views,
    });
    Ok(state)
}

/// The first of the longest (`longest`) or of the shortest paths.
fn pick<T>(paths: &[Vec<T>], longest: bool) -> &[T] {
    let mut best = &paths[0];
    for path in &paths[1..] {
        if (longest && path.len() > best.len()) || (!longest && path.len() < best.len()) {
            best = path;
        }
    }
    best
}

fn build(ir: &Value) -> Story<Value> {
    let spec = Spec::new(ir)?;
    let mut steps = Vec::new();
    let open = creating_outcome(spec.command(&spec.create_inbox))?;
    step(&spec, &mut steps, &spec.create_inbox, &open, 0, None)?;
    let create = creating_outcome(spec.command(&spec.create_message))?;
    let terminal = list(spec.lifecycle.get("terminal"));
    let mut journey = 0;
    for (index, end) in terminal.iter().enumerate() {
        let end = end.as_str().ok_or("a terminal state is no text")?;
        journey += 1;
        let paths = spec.paths(end)?;
        let path = pick(&paths, index == 0);
        let mut state = step(
            &spec,
            &mut steps,
            &spec.create_message,
            &create,
            journey,
            None,
        )?;
        for (_to, command, outcome) in path {
            state = step(&spec, &mut steps, command, outcome, journey, state)?;
        }
        let (command, outcome) = spec.deleting(end)?;
        step(&spec, &mut steps, &command, &outcome, journey, state)?;
    }
    for name in spec.commands.keys() {
        if spec.actor_of[name] != spec.creator
            && subject_entity(name, spec.command(name))? == spec.message
        {
            journey += 1;
            let state = step(
                &spec,
                &mut steps,
                &spec.create_message,
                &create,
                journey,
                None,
            )?;
            let defaults: Vec<String> = successes(spec.command(name))
                .iter()
                .filter(|o| {
                    o.get("condition")
                        .and_then(|c| c.get("kind"))
                        .and_then(Value::as_str)
                        == Some("otherwise")
                })
                .filter_map(|o| o.get("name").and_then(Value::as_str).map(str::to_string))
                .collect();
            let outcome = one_name(&defaults, &format!("default outcome of {name}"))?;
            step(&spec, &mut steps, name, &outcome, journey, state)?;
        }
    }
    let initial = text(spec.inbox_lifecycle, "initial", "the inbox lifecycle")?;
    let leaving: Vec<String> = list(spec.inbox_lifecycle.get("transitions"))
        .iter()
        .filter(|t| {
            list(t.get("from"))
                .iter()
                .any(|f| f.as_str() == Some(initial))
        })
        .filter_map(|t| t.get("name").and_then(Value::as_str).map(str::to_string))
        .collect();
    let leaving = one_name(&leaving, &format!("transition leaving {initial}"))?;
    let (command, outcome) = spec.transition_command(&spec.inbox, &leaving)?;
    step(
        &spec,
        &mut steps,
        &command,
        &outcome,
        0,
        Some(initial.to_string()),
    )?;

    let placed: BTreeSet<&str> = steps.iter().map(|s| s.command.as_str()).collect();
    let missing: Vec<&str> = spec
        .commands
        .keys()
        .filter(|c| !placed.contains(c.as_str()))
        .map(|c| short(c))
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "commands with no place in the story: {}",
            missing.join(", ")
        ));
    }
    let shown_views: BTreeSet<&str> = steps
        .iter()
        .flat_map(|s| s.views.iter().map(String::as_str))
        .collect();
    let all_views: BTreeSet<&str> = spec.views.keys().map(|v| short(v)).collect();
    let missing: Vec<&str> = all_views.difference(&shown_views).copied().collect();
    if !missing.is_empty() {
        return Err(format!(
            "views with no place in the story: {}",
            missing.join(", ")
        ));
    }

    let offered: Vec<Vec<String>> = steps
        .iter()
        .map(|s| refusals(spec.command(&s.command)))
        .collect::<Story<_>>()?;
    let mut assigned: Vec<Vec<String>> = vec![Vec::new(); steps.len()];
    let mut errors: Vec<String> = spec.errors.keys().map(|e| short(e).to_string()).collect();
    errors.sort();
    let scarcity: BTreeMap<&str, usize> = errors
        .iter()
        .map(|e| (e.as_str(), offered.iter().filter(|o| o.contains(e)).count()))
        .collect();
    let missing: Vec<&str> = errors
        .iter()
        .filter(|e| scarcity[e.as_str()] == 0)
        .map(String::as_str)
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "errors no command of the story declares: {}",
            missing.join(", ")
        ));
    }
    let mut by_scarcity = errors.clone();
    by_scarcity.sort_by(|a, b| (scarcity[a.as_str()], a).cmp(&(scarcity[b.as_str()], b)));
    for error in &by_scarcity {
        let target = (0..steps.len())
            .filter(|&i| offered[i].contains(error))
            .min_by_key(|&i| (assigned[i].len(), i))
            .expect("an error with a scarcity above 0 has a step");
        assigned[target].push(error.clone());
    }
    let mut times_shown: BTreeMap<String, usize> = errors.iter().map(|e| (e.clone(), 1)).collect();
    for (i, chosen) in assigned.iter_mut().enumerate() {
        if chosen.is_empty() && !offered[i].is_empty() {
            let mut pick: Option<(usize, usize, &String)> = None;
            for (position, error) in offered[i].iter().enumerate() {
                let count = *times_shown.get(error).ok_or_else(|| {
                    format!("the error {error} a command declares is not in the IR")
                })?;
                if pick.is_none_or(|(c, p, _)| (count, position) < (c, p)) {
                    pick = Some((count, position, error));
                }
            }
            let (_, _, error) = pick.expect("offered is not empty");
            *times_shown.get_mut(error).expect("counted above") += 1;
            chosen.push(error.clone());
        }
        let order = &offered[i];
        chosen.sort_by_key(|e| order.iter().position(|o| o == e));
    }

    let service: BTreeSet<String> = steps
        .iter()
        .filter(|s| s.actor == "service")
        .map(|s| short(&spec.actor_of[&s.command]).to_string())
        .collect();
    let service: Vec<String> = service.into_iter().collect();
    let service = one_name(&service, "actor of the service's own commands")?;
    let creator = short(&spec.creator).to_string();
    let names: BTreeMap<&str, String> = [
        ("sender", creator.clone()),
        ("recipient", creator),
        ("service", service),
    ]
    .into_iter()
    .collect();

    let mut frames = Vec::new();
    for (i, s) in steps.iter().enumerate() {
        let command = spec.command(&s.command);
        let lifecycle = if s.entity == spec.message {
            spec.lifecycle
        } else {
            spec.inbox_lifecycle
        };
        let state_text = if s.removed { None } else { s.state.clone() };
        let refusals: Vec<Option<&String>> = if assigned[i].is_empty() {
            vec![None]
        } else {
            assigned[i].iter().map(Some).collect()
        };
        for refusal in refusals {
            let mut visible: Vec<String> = LAYOUT_TEXTS.iter().map(|t| t.to_string()).collect();
            visible.push(short(&s.command).to_string());
            if let Some(state) = state_text.as_ref().filter(|t| !t.is_empty()) {
                visible.push(state.clone());
            }
            visible.extend(s.views.iter().cloned());
            if let Some(refusal) = refusal.filter(|r| !r.is_empty()) {
                visible.push(refusal.clone());
            }
            let words: usize = visible.iter().map(|t| t.split_whitespace().count()).sum();
            if words > WORD_LIMIT {
                return Err(format!(
                    "frame {} ({}) shows {words} words, more than {WORD_LIMIT}: {}",
                    frames.len(),
                    short(&s.command),
                    shown(&visible)
                ));
            }
            let summary = plain(get(command, "naming").and_then(|n| n.get("summary")));
            let state_summary = plain(
                s.state
                    .as_ref()
                    .and_then(|state| get(lifecycle, "naming").and_then(|n| get(n, state)))
                    .and_then(|n| n.get("summary")),
            );
            let mut parts = vec![format!(
                "{}, the {}, sends {}. {summary}",
                names[s.actor],
                s.actor,
                short(&s.command)
            )];
            if s.removed {
                parts.push(format!(
                    "The {} is removed.",
                    short(&s.entity).to_lowercase()
                ));
            } else {
                let state = s.state.as_deref().unwrap_or("None");
                let mut part = format!("State: {state}.");
                if !state_summary.is_empty() {
                    part.push(' ');
                    part.push_str(&state_summary);
                }
                parts.push(part);
            }
            if !s.views.is_empty() {
                let lead = if s.removed {
                    "Found in "
                } else {
                    "Visible in "
                };
                parts.push(format!("{lead}{}.", s.views.join(", ")));
            }
            if let Some(refusal) = refusal.filter(|r| !r.is_empty()) {
                let error = spec
                    .errors
                    .iter()
                    .find(|(n, _)| short(n) == refusal.as_str())
                    .map(|(_, e)| e)
                    .expect("an assigned refusal is an error of the IR");
                parts.push(format!(
                    "{} can be refused with {refusal}: {}",
                    short(&s.command),
                    plain(error.get("summary"))
                ));
            }
            let description: Vec<String> = parts.into_iter().filter(|p| !p.is_empty()).collect();
            let end = if s.entity == spec.message {
                terminal
                    .iter()
                    .position(|t| s.state.as_deref().is_some_and(|s| t.as_str() == Some(s)))
                    .map(Value::from)
            } else {
                None
            };
            frames.push(json!({
                "actor": s.actor,
                "command": short(&s.command),
                "description": description.join(" "),
                "end": end,
                "entity": short(&s.entity),
                "journey": s.journey,
                "refusal": refusal,
                "removed": s.removed,
                "state": state_text,
                "step": i,
                "token": s.token,
                "views": s.views,
                "visible": visible,
                "words": words,
            }));
        }
    }

    let mut order: Vec<Vec<String>> = Vec::new();
    for kind in ["command", "views", "refusal"] {
        let mut seen: Vec<String> = Vec::new();
        for frame in &frames {
            let values = match &frame[kind] {
                Value::Array(items) => items.clone(),
                other => vec![other.clone()],
            };
            for value in values {
                if let Some(value) = value.as_str().filter(|v| !v.is_empty())
                    && !seen.iter().any(|s| s == value)
                {
                    seen.push(value.to_string());
                }
            }
        }
        order.push(seen);
    }
    let first: Vec<&Step> = steps.iter().filter(|s| s.journey == 1).collect();
    let still: Vec<Value> = first
        .iter()
        .map(|s| {
            let toward = if s.token == Some("inbox") {
                "inbox"
            } else {
                "actor"
            };
            json!({
                "actor": s.actor,
                "command": short(&s.command),
                "toward": toward,
            })
        })
        .collect();
    let path: Vec<Option<&String>> = first
        .iter()
        .filter(|s| !s.removed && !is_in(s.state.as_deref(), terminal))
        .map(|s| s.state.as_ref())
        .collect();
    let layout: Map<String, Value> = LAYOUT
        .iter()
        .map(|(k, v)| (k.to_string(), Value::from(*v)))
        .collect();
    Ok(json!({
        "actors": names,
        "commands": order[0],
        "counts": {
            "commands": spec.commands.len(),
            "errors": spec.errors.len(),
            "views": spec.views.len(),
        },
        "errors": order[2],
        "frame_ms": FRAME_MS,
        "frames": frames,
        "generated": GENERATED,
        "inbox": short(&spec.inbox),
        "message": short(&spec.message),
        "still": still,
        "layout": layout,
        "lifecycle": {
            "initial": spec.lifecycle.get("initial").cloned().unwrap_or(Value::Null),
            "path": path,
            "terminal": terminal,
        },
        "planned_note": PLANNED_NOTE,
        "views": order[1],
        "word_limit": WORD_LIMIT,
    }))
}
