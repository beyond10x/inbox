//! The acceptance of story:cli-client: after the recipient runs `inbox open` against a running
//! `inbox serve`, `inbox post` by the sender followed by `inbox pending` by the recipient prints
//! the posted message.
//!
//! The steps are the first use of `README.md`, run in its order, and the second case holds the
//! README to them: every command line of its first use is one of these steps.

mod support;

use std::collections::HashMap;

use support::{Dir, Service, ids, rows, run, text};

/// The command lines of the README's first use, in its order.
const FIRST_USE: &[&str] = &[
    "OPERATOR_PUBLIC_KEY=$(inbox key new --out operator.key)",
    "ALICE_PUBLIC_KEY=$(inbox key new --out alice.key --key-id alice-1 --principal agent:demo/alice)",
    "BOB_PUBLIC_KEY=$(inbox key new --out bob.key --key-id bob-1 --principal agent:demo/bob)",
    "inbox keyfile add-key --file keys.json --operator-secret-key-file operator.key --key-id alice-1 --principal agent:demo/alice --public-key \"$ALICE_PUBLIC_KEY\"",
    "inbox keyfile add-key --file keys.json --operator-secret-key-file operator.key --key-id bob-1 --principal agent:demo/bob --public-key \"$BOB_PUBLIC_KEY\"",
    "inbox serve --listen 127.0.0.1:0 --key-file keys.json --operator-public-key \"$OPERATOR_PUBLIC_KEY\"",
    "inbox open --secret-key-file bob.key",
    "MESSAGE_ID=$(inbox post --secret-key-file alice.key --to agent:demo/bob --body \"Hello, bob\")",
    "inbox pending --secret-key-file bob.key",
    "inbox read --secret-key-file bob.key \"$MESSAGE_ID\"",
];

/// The words of one command line: split at spaces, a double-quoted run kept whole, and a word
/// that is `$NAME` replaced by that variable.
fn words(line: &str, variables: &HashMap<String, String>) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    let mut started = false;
    for character in line.chars() {
        match character {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            ' ' if !quoted => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            character => {
                word.push(character);
                started = true;
            }
        }
    }
    if started {
        words.push(word);
    }
    words
        .into_iter()
        .map(|word| match word.strip_prefix('$') {
            Some(name) => variables
                .get(name)
                .unwrap_or_else(|| panic!("`${name}` is not set"))
                .clone(),
            None => word,
        })
        .collect()
}

#[test]
fn a_message_the_sender_posts_is_in_the_recipients_pending_after_the_recipient_opened_the_inbox() {
    let dir = Dir::new("first-use");
    let mut variables: HashMap<String, String> = HashMap::new();
    let mut service: Option<Service> = None;
    for line in FIRST_USE {
        let (capture, command) = match line.split_once("=$(") {
            Some((name, rest)) => (
                Some(name),
                rest.strip_suffix(')').expect("a capture ends in `)`"),
            ),
            None => (None, *line),
        };
        let words = words(command, &variables);
        assert_eq!(words[0], "inbox", "{line}");
        let args: Vec<&str> = words[1..].iter().map(String::as_str).collect();
        if args[0] == "serve" {
            service = Some(Service::start(&dir.path, &args[1..], &[]));
            continue;
        }
        let url = service.as_ref().map(|service| service.url.clone());
        let env: Vec<(&str, &str)> = url.iter().map(|url| ("INBOX_URL", url.as_str())).collect();
        let answer = run(&dir.path, &args, &env).ok();
        if let Some(name) = capture {
            variables.insert(name.to_owned(), answer.stdout.trim().to_owned());
        }
        if args[0] == "pending" {
            let message_id = &variables["MESSAGE_ID"];
            assert!(answer.stdout.contains(message_id.as_str()), "{answer:#?}");
            assert!(answer.stdout.contains("Hello, bob"), "{answer:#?}");

            let mut json_args = args.clone();
            json_args.push("--json");
            let pending = run(&dir.path, &json_args, &env).ok().json();
            assert_eq!(ids(&pending), vec![message_id.clone()]);
            assert_eq!(text(&rows(&pending)[0], "body"), "Hello, bob");
            assert_eq!(text(&rows(&pending)[0], "from"), "agent:demo/alice");
        }
    }
    let url = service.as_ref().expect("the service ran").url.clone();
    let pending = run(
        &dir.path,
        &["pending", "--secret-key-file", "bob.key", "--json"],
        &[("INBOX_URL", url.as_str())],
    )
    .ok()
    .json();
    assert!(ids(&pending).is_empty(), "a note is handled once read");
}

#[test]
fn every_command_line_of_the_readme_first_use_is_a_step_of_this_test_in_its_order() {
    let readme = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md"))
        .expect("README.md reads");
    let section = readme
        .split("\n### First use\n")
        .nth(1)
        .expect("README.md has a `### First use` section");
    let section = section.split("\n## ").next().unwrap_or(section);
    let mut lines = Vec::new();
    let mut fenced = false;
    for line in section.lines() {
        if line.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        let line = line.trim();
        if !fenced
            || line.is_empty()
            || line.starts_with('#')
            || line.starts_with("export INBOX_URL=")
        {
            continue;
        }
        lines.push(line.to_owned());
    }
    assert_eq!(lines, FIRST_USE);
}
