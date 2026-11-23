//! The reference document of this command line (`b10x-cli-reference/1`), written from the clap
//! definition: the documentation site renders `generated/cli/inbox.json`, and a test holds the
//! file to the definition.
//!
//! What clap does not know is stated here: the description, the environment variables read
//! outside a flag, and the meaning of each exit status.

use clap::{Arg, ArgAction, Command};
use inbox::server::json::{self, Value};

use crate::failure::{LOCAL, REFUSED, TRANSPORT, USAGE};

const DESCRIPTION: &str = "`inbox serve` runs the HTTP service of the inbox on a loopback \
address: plain HTTP by default, TLS with `--tls-file` and the certificate `inbox tls new` makes. \
Every other command is a client of that service and signs what it sends with the caller's own \
key. A client command whose answer comes from a new start of the service then opens the caller's \
own inbox again with the adapter of its last `inbox open` (none after `inbox close`) and posts \
the kept envelopes again, as `inbox repost` does; one line on standard error says so, and with \
`--json` the member `reconnected` of the document.";

/// The environment variables read outside a flag: name, description, the command that reads it
/// (every client command when `None`).
const ENVIRONMENT: [(&str, &str, Option<&str>); 3] = [
    (
        "INBOX_BACKEND",
        "The backend `inbox serve` keeps the inboxes in. `memory`, the default, is the only one.",
        Some("serve"),
    ),
    (
        "XDG_STATE_HOME",
        "Without `--outbox` and `INBOX_OUTBOX`, the outbox file is \
         `$XDG_STATE_HOME/inbox/outbox.jsonl`.",
        None,
    ),
    (
        "HOME",
        "Without `XDG_STATE_HOME` either, the outbox file is \
         `$HOME/.local/state/inbox/outbox.jsonl`.",
        None,
    ),
];

const EXIT_STATUSES: [(i32, &str); 5] = [
    (0, "Done."),
    (REFUSED, "The service refused; the output names the error."),
    (
        USAGE,
        "Wrong usage, including an argument read as text that is not UTF-8, an `INBOX_URL` that is not a loopback address, an `https://` URL without a pin and a pin beside an `http://` URL.",
    ),
    (
        TRANSPORT,
        "The service could not be reached, its TLS certificate did not match the pin, or its answer was neither a success nor a refusal.",
    ),
    (LOCAL, "A local file could not be read, written or created."),
];

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

fn object(members: Vec<(&str, Value)>) -> Value {
    Value::Object(
        members
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

fn argument(arg: &Arg) -> Value {
    let id = arg.get_id().as_str();
    let switch =
        !arg.is_positional() && !matches!(arg.get_action(), ArgAction::Set | ArgAction::Append);
    let kind = if arg.is_positional() {
        "positional"
    } else if switch {
        "switch"
    } else {
        "option"
    };
    let mut members = vec![("name", text(id)), ("kind", text(kind))];
    if let Some(long) = arg.get_long() {
        members.push(("long", text(long)));
    }
    if let Some(short) = arg.get_short() {
        members.push(("short", text(&short.to_string())));
    }
    if !switch {
        let name = arg
            .get_value_names()
            .and_then(|names| names.first())
            .map_or_else(|| id.to_uppercase(), ToString::to_string);
        members.push(("valueName", text(&name)));
    }
    if arg.is_required_set() {
        members.push(("required", Value::Bool(true)));
    }
    // A switch carries no default: clap gives a built `SetTrue` flag the default `false`, and
    // the format refuses `default` on a switch.
    if !switch && let Some(default) = arg.get_default_values().first() {
        members.push(("default", text(&default.to_string_lossy())));
    }
    if let Some(variable) = arg.get_env() {
        members.push(("env", text(&variable.to_string_lossy())));
    }
    if matches!(arg.get_action(), ArgAction::Append) {
        members.push(("repeatable", Value::Bool(true)));
    }
    if let Some(help) = arg.get_help() {
        members.push(("help", text(&help.to_string())));
    }
    object(members)
}

fn arguments(command: &Command, global: bool) -> Vec<Value> {
    command
        .get_arguments()
        .filter(|arg| !arg.is_hide_set() && arg.is_global_set() == global)
        .map(argument)
        .collect()
}

/// The groups of a command that constrain its arguments: one of them is required, or not more
/// than one may be given. The group clap derives for every flattened struct constrains nothing.
fn groups(command: &Command) -> Vec<Value> {
    command
        .get_groups()
        .cloned()
        .filter_map(|mut group| {
            // clap 4 asks for `&mut self` to read this flag.
            let multiple = group.is_multiple();
            (group.is_required_set() || !multiple).then(|| {
                object(vec![
                    ("name", text(group.get_id().as_str())),
                    (
                        "arguments",
                        Value::Array(group.get_args().map(|id| text(id.as_str())).collect()),
                    ),
                    ("required", Value::Bool(group.is_required_set())),
                    ("multiple", Value::Bool(multiple)),
                ])
            })
        })
        .collect()
}

fn commands(command: &Command, path: &[String], out: &mut Vec<Value>) {
    for sub in command.get_subcommands().filter(|sub| !sub.is_hide_set()) {
        let mut path = path.to_vec();
        path.push(sub.get_name().to_owned());
        if sub.has_subcommands() {
            commands(sub, &path, out);
            continue;
        }
        let mut members = vec![(
            "path",
            Value::Array(path.iter().map(|token| text(token)).collect()),
        )];
        if let Some(about) = sub.get_about() {
            members.push(("about", text(&about.to_string())));
        }
        members.push(("arguments", Value::Array(arguments(sub, false))));
        let groups = groups(sub);
        if !groups.is_empty() {
            members.push(("groups", Value::Array(groups)));
        }
        out.push(object(members));
    }
}

/// The reference document of `root`.
pub fn reference(root: &Command) -> Value {
    let mut listed = Vec::new();
    commands(root, &[], &mut listed);
    let environment = ENVIRONMENT
        .iter()
        .map(|(name, description, command)| {
            let mut members = vec![("name", text(name)), ("description", text(description))];
            if let Some(command) = command {
                members.push((
                    "commands",
                    Value::Array(vec![Value::Array(vec![text(command)])]),
                ));
            }
            object(members)
        })
        .collect();
    let statuses = EXIT_STATUSES
        .iter()
        .map(|(code, meaning)| {
            object(vec![
                ("code", Value::Number(code.to_string())),
                ("meaning", text(meaning)),
            ])
        })
        .collect();
    let mut members = vec![
        ("format", text("b10x-cli-reference/1")),
        ("binary", text(root.get_name())),
    ];
    if let Some(about) = root.get_about() {
        members.push(("about", text(&about.to_string())));
    }
    members.extend([
        ("description", text(DESCRIPTION)),
        ("globalOptions", Value::Array(arguments(root, true))),
        ("commands", Value::Array(listed)),
        ("environment", Value::Array(environment)),
        ("exitStatuses", Value::Array(statuses)),
    ]);
    object(members)
}

/// `value` as an indented JSON document that ends with a newline: one member or item per line,
/// so that a change of one flag is one changed line.
pub fn pretty(value: &Value) -> String {
    let mut out = String::new();
    write(value, 0, &mut out);
    out.push('\n');
    out
}

fn write(value: &Value, depth: usize, out: &mut String) {
    let indent = |out: &mut String, depth: usize| out.push_str(&"  ".repeat(depth));
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(flag) => json::push_bool(out, *flag),
        Value::Number(spelling) => out.push_str(spelling),
        Value::Text(text) => json::push_text(out, text),
        Value::Array(items) if items.is_empty() => out.push_str("[]"),
        Value::Array(items) => {
            out.push_str("[\n");
            for (at, item) in items.iter().enumerate() {
                indent(out, depth + 1);
                write(item, depth + 1, out);
                out.push_str(if at + 1 < items.len() { ",\n" } else { "\n" });
            }
            indent(out, depth);
            out.push(']');
        }
        Value::Object(members) if members.is_empty() => out.push_str("{}"),
        Value::Object(members) => {
            out.push_str("{\n");
            for (at, (name, member)) in members.iter().enumerate() {
                indent(out, depth + 1);
                json::push_text(out, name);
                out.push_str(": ");
                write(member, depth + 1, out);
                out.push_str(if at + 1 < members.len() { ",\n" } else { "\n" });
            }
            indent(out, depth);
            out.push('}');
        }
    }
}
