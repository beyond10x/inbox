//! Adversary pass 1 on story:the-chat-bridge-turns-a-thread-reply-into-an-answer. Each case runs
//! against a real `inbox serve` on loopback and the stand-in `connectors` of `bridge_once`, with
//! the thread of a post scripted in `replies-of.<ts of the post>`; no case reaches a chat provider
//! or holds a wall-clock time: each waits on the stand-in's record or a fact, with the bound of
//! `bridge_once` that only ends a hang.

use crate::bridge_once::{
    self, Bridge, CHANNEL, Connectors, Forwarder, client, post, posted, state, wait_for,
};
use crate::support;

use inbox::server::json;
use support::{Signer, World, text};

/// The allowed chat user of the binding of `bridge_once`.
const USER: &str = "U0000000001";
/// Another chat user of the channel.
const OTHER_USER: &str = "U0000000002";

fn world(name: &str) -> World {
    bridge_once::clocked_signed(name, Signer::Person)
}

/// Scripts the thread of the post `reference`: the post itself, then `said` (reference, chat
/// user, text), and `has_more` as the provider's page says it.
fn thread(connectors: &Connectors, reference: &str, said: &[(String, &str, &str)], more: bool) {
    let mut lines = vec![format!(
        r#"{{"type":"message","user":"U0000000009","text":"a post","ts":"{reference}"}}"#
    )];
    for (ts, user, said) in said {
        let mut quoted = String::new();
        json::push_text(&mut quoted, said);
        lines.push(format!(
            r#"{{"type":"message","user":"{user}","text":{quoted},"thread_ts":"{reference}","ts":"{ts}"}}"#
        ));
    }
    connectors.file(
        &format!("replies-of.{reference}"),
        &format!(
            "{}\n{}\n{}{more}{}\n",
            r#"{"result":{"result":{"status":200,"body":{"ok":true,"messages":["#,
            lines.join("\n"),
            r#"],"has_more":"#,
            r"}}}}",
        ),
    );
}

/// The `ts` of the post of each read of the replies so far, in their order.
fn read_posts(connectors: &Connectors) -> Vec<String> {
    (1..=connectors.thread_reads())
        .map(|k| {
            text(
                &connectors.input(connectors.invocation("thread-read", k)),
                "ts",
            )
        })
        .collect()
}

/// Item 7 of the brief: the read of the replies of one post fails at every poll (its post was
/// removed from the channel: the provider answers `ok: false`, `thread_not_found`). The reply of
/// the allowed chat user in the thread of another post is still sent as its `Answer`.
#[test]
fn a_thread_whose_read_fails_stops_no_answer_in_another_thread() {
    let world = world("adv-answer-one-stops-all");
    let url = world.service.url.clone();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let _bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    let removed = post(&world, &url, "request", "Which build is green?", &[]);
    let other = post(&world, &url, "request", "Which build is red?", &[]);
    let removed_ts = posted(&world, &url, &removed);
    let other_ts = posted(&world, &url, &other);
    connectors.file(
        &format!("replies-of.{removed_ts}"),
        "{\"result\":{\"result\":{\"status\":200,\"body\":{\"ok\":false,\"error\":\"thread_not_found\"}}}}\n",
    );
    thread(
        &connectors,
        &other_ts,
        &[("1700000002.000001".to_owned(), USER, "Build 6 is red.")],
        false,
    );
    let reads = connectors.thread_reads();
    wait_for("3 more reads of the thread of the removed post", || {
        read_posts(&connectors)
            .iter()
            .skip(reads)
            .filter(|ts| **ts == removed_ts)
            .count()
            >= 3
    });
    assert!(
        read_posts(&connectors)
            .iter()
            .skip(reads)
            .any(|ts| *ts == other_ts),
        "the thread of the other post was never read in 3 polls: {:?}",
        read_posts(&connectors)
    );
    wait_for("the other request is answered", || {
        state(&world, &url, &other).is_empty()
    });
}

/// Item 2 of the brief: one page of 100, no cursor, and `oldest` moves only past a reply of the
/// allowed chat user. 99 replies of another chat user fill the first page with the post; the
/// person's reply is on the page after it (`has_more`). As the input never changes, the provider
/// answers the same first page at every poll: the reply is never read.
#[test]
fn a_full_page_of_other_users_does_not_hide_the_persons_reply_for_ever() {
    let world = world("adv-answer-full-page");
    let url = world.service.url.clone();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let _bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    let request_ts = posted(&world, &url, &request);
    let others: Vec<(String, &str, &str)> = (1..=99)
        .map(|k| (format!("1700000002.{k:06}"), OTHER_USER, "me too"))
        .collect();
    thread(&connectors, &request_ts, &others, true);
    let reads = connectors.thread_reads();
    wait_for("3 more reads of the thread", || {
        connectors.thread_reads() >= reads + 3
    });
    let inputs: Vec<_> = (reads + 1..=connectors.thread_reads())
        .map(|k| connectors.input(connectors.invocation("thread-read", k)))
        .collect();
    assert!(
        inputs
            .iter()
            .all(|input| input.member("cursor").is_some() || input.member("oldest").is_some()),
        "every read asks for the same first page, so the reply after it is never read: {inputs:?}"
    );
}

/// Item 1 of the brief: a reply of the allowed chat user to a request that holds only white space
/// (or no text: a file or an image shared in the thread, whose `text` the provider lists empty)
/// sends no `Answer` with an empty body in the person's place.
#[test]
fn a_reply_without_text_is_not_sent_as_an_empty_answer() {
    let world = world("adv-answer-empty");
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let _bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    let request = post(&world, &url, "request", "Send the log of build 7.", &[]);
    let request_ts = posted(&world, &url, &request);
    thread(
        &connectors,
        &request_ts,
        &[("1700000002.000001".to_owned(), USER, "  ")],
        false,
    );
    let reads = connectors.thread_reads();
    wait_for("3 more reads of the thread", || {
        connectors.thread_reads() >= reads + 3
    });
    let sent = forwarder.commands(0, "answer", &request);
    assert!(
        sent.is_empty(),
        "an Answer with no text was sent for {request}: {}",
        sent.iter()
            .map(|at| String::from_utf8_lossy(&forwarder.sent()[*at]).into_owned())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

// Adversary pass 2: the cases below.

use crate::bridge_once::{Instead, recorded};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

/// The start of the head of an `Answer` request.
const ANSWER_HEAD: &str = "POST /messaging/commands/answer ";

/// The reference the stand-in answers the first post of a case with.
const FIRST_POST: &str = "1700000000.000001";

/// Puts a script in front of the stand-in of `connectors` (the stand-in itself runs behind it, so
/// its record of every invocation stays as it is) and answers the stand-in's directory. For the
/// `c`-th read of the replies of the post `<ts>` (counted from 1 in `wrap-read.<ts>.<c>`): with
/// the file `limit.<ts>` there, the read is refused `rate_limited` with `retry_after_seconds` the
/// number the file holds, 1 when it is empty (exit 1, the file removed, the invocation number in
/// `limited`, then the system clock in nanoseconds in `limited-at`); with the file
/// `fail.<ts>.<c>` there, the read answers `ok: false`, `thread_not_found`. The first invocation
/// that starts after `limited-at` is written writes the system clock at its start in
/// `after-limit`.
pub(crate) fn scripted(connectors: &Connectors) -> PathBuf {
    let path = connectors.path();
    let bin = PathBuf::from(
        path.split(':')
            .next()
            .expect("the stand-in's directory first"),
    );
    let dir = bin
        .parent()
        .expect("the stand-in's directory")
        .to_path_buf();
    let shown = dir.to_str().expect("a UTF-8 path");
    assert!(!shown.contains('\''), "{shown}");
    fs::rename(bin.join("connectors"), bin.join("connectors.real")).expect("the stand-in moves");
    let script = format!(
        r#"#!/bin/sh
d='{shown}'
b=$(date +%s%N)
if [ -e "$d/limited-at" ] && [ ! -e "$d/after-limit" ]; then
  echo "$b" > "$d/after-limit.new"
  mv "$d/after-limit.new" "$d/after-limit"
fi
cat > "$d/wrap.in"
t=$(sed -n 's/.*"ts":"\([0-9.]*\)".*/\1/p' "$d/wrap.in")
if [ "$5" = conversations.replies ] && [ -n "$t" ]; then
  c=1
  while [ -e "$d/wrap-read.$t.$c" ]; do c=$((c + 1)); done
  : > "$d/wrap-read.$t.$c"
  if [ -e "$d/limit.$t" ]; then
    s=$(cat "$d/limit.$t")
    [ -n "$s" ] || s=1
    rm -f "$d/limit.$t"
    "$d/bin/connectors.real" "$@" < "$d/wrap.in" > /dev/null 2>&1
    n=1
    while [ -e "$d/args.$n" ]; do n=$((n + 1)); done
    echo "$((n - 1))" > "$d/limited.new"
    mv "$d/limited.new" "$d/limited"
    date +%s%N > "$d/limited-at.new"
    mv "$d/limited-at.new" "$d/limited-at"
    printf '{{"ok":false,"error":{{"code":"failure","message":"rate limited","data":{{"code":"service_failure","service_code":"rate_limited","retry_after_seconds":%s}}}}}}\n' "$s" >&2
    exit 1
  fi
  if [ -e "$d/fail.$t.$c" ]; then
    "$d/bin/connectors.real" "$@" < "$d/wrap.in" > /dev/null 2>&1
    printf '%s\n' '{{"result":{{"result":{{"status":200,"body":{{"ok":false,"error":"thread_not_found"}}}}}}}}'
    exit 0
  fi
fi
exec "$d/bin/connectors.real" "$@" < "$d/wrap.in"
"#
    );
    let program = bin.join("connectors");
    fs::write(&program, script).expect("the script is written");
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
        .expect("the script is executable");
    dir
}

/// What every bridge of the case wrote to standard error so far.
fn bridge_stderr(world: &World) -> String {
    let mut found: Vec<PathBuf> = fs::read_dir(&world.dir.path)
        .expect("the directory is read")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            name.starts_with("bridge-") && name.ends_with(".err")
        })
        .collect();
    found.sort();
    found
        .iter()
        .map(|path| fs::read_to_string(path).unwrap_or_default())
        .collect()
}

/// Item 4 of pass 2: an `Answer` the service took whose answer was lost (the forwarder carries it
/// and throws the service's answer away). Nothing is sent again (one `Answer`, the entry ends
/// `answered`), so no line on standard error may tell the operator that the answer was not taken
/// and is sent again.
#[test]
fn an_answer_the_service_took_is_not_named_as_not_taken_and_sent_again() {
    let world = world("adv2-answer-lost-line");
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    let request_ts = posted(&world, &url, &request);
    forwarder.next(ANSWER_HEAD, Instead::AnswerLost);
    thread(
        &connectors,
        &request_ts,
        &[("1700000002.000001".to_owned(), USER, "Build 7 is green.")],
        false,
    );
    wait_for("the answer of the Answer is lost", || !forwarder.dropping());
    wait_for("the entry ends answered", || {
        recorded(&world, &request, "end") == "answered"
    });
    let sent = forwarder.commands(0, "answer", &request).len();
    let said = bridge.end();
    assert_eq!(sent, 1, "one Answer, nothing sent again: {said}");
    let false_lines: Vec<&str> = said
        .lines()
        .filter(|line| {
            line.contains(&request) && (line.contains("not taken") || line.contains("sent again"))
        })
        .collect();
    assert!(
        false_lines.is_empty(),
        "the service took the one Answer of {request} and nothing was sent again, yet standard \
         error says: {false_lines:?}"
    );
}

/// Item 3 of pass 2: the read of the replies of one post is refused `rate_limited` with a wait
/// of 1 s. The read of the thread of the other post is not fired inside that wait: the next
/// invocation of `connectors` after the refused read is not a read of the replies in the same
/// poll (the next poll starts with the read of the channel).
#[test]
fn a_rate_limited_read_of_one_thread_fires_no_read_of_another_thread_inside_its_wait() {
    let world = world("adv2-answer-rate-limited");
    let url = world.service.url.clone();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let dir = scripted(&connectors);
    let _bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    let first = post(&world, &url, "request", "Which build is green?", &[]);
    let second = post(&world, &url, "request", "Which build is red?", &[]);
    let first_ts = posted(&world, &url, &first);
    let second_ts = posted(&world, &url, &second);
    let reads = connectors.thread_reads();
    wait_for("a read of the thread of the second post", || {
        read_posts(&connectors)
            .iter()
            .skip(reads)
            .any(|ts| *ts == second_ts)
    });
    fs::write(dir.join(format!("limit.{first_ts}")), "").expect("the limit is armed");
    wait_for(
        "the read of the first thread is refused rate_limited",
        || dir.join("limited").exists(),
    );
    let limited: usize = fs::read_to_string(dir.join("limited"))
        .expect("the invocation number")
        .trim()
        .parse()
        .expect("a number");
    assert_eq!(
        text(&connectors.input(limited), "ts"),
        first_ts,
        "the refused read is of the first post"
    );
    wait_for("the invocation after the refused read", || {
        dir.join(format!("args.{}", limited + 1)).exists()
    });
    let next = connectors.args(limited + 1);
    let operation = next.get(4).cloned().unwrap_or_default();
    let of = if operation == "conversations.replies" {
        text(&connectors.input(limited + 1), "ts")
    } else {
        String::new()
    };
    assert!(
        operation != "conversations.replies",
        "the read of the thread of {of} was fired right after the read of the replies of \
         {first_ts} was refused rate_limited (the next read not before 1 s), in the same poll: \
         invocation {} is {operation:?}",
        limited + 1
    );
}

/// Item 3 of pass 2: the read of the replies of one post fails in the reads 1, 2 and 4 of it and
/// succeeds in read 3. Standard error names the post once per run of failures: one line for the
/// reads 1 and 2, and one again for read 4, after the success.
#[test]
fn a_failed_thread_read_is_named_once_per_run_and_again_after_a_success() {
    let world = world("adv2-answer-failing-runs");
    let url = world.service.url.clone();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let dir = scripted(&connectors);
    for c in [1, 2, 4] {
        fs::write(dir.join(format!("fail.{FIRST_POST}.{c}")), "").expect("a failure is armed");
    }
    let _bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    let request_ts = posted(&world, &url, &request);
    assert_eq!(request_ts, FIRST_POST);
    let named = |said: &str| {
        said.lines()
            .filter(|line| {
                line.contains(&format!("the read of the replies to the post of {request}"))
            })
            .count()
    };
    let reads_of = || {
        read_posts(&connectors)
            .iter()
            .filter(|ts| **ts == request_ts)
            .count()
    };
    // Read 3 starts after the poll of read 2 wrote what it writes.
    wait_for("3 reads of the thread", || reads_of() >= 3);
    let said = bridge_stderr(&world);
    assert_eq!(
        named(&said),
        1,
        "one line for the run of the reads 1 and 2: {said}"
    );
    wait_for("4 reads of the thread and 2 lines", || {
        reads_of() >= 4 && named(&bridge_stderr(&world)) >= 2
    });
    let said = bridge_stderr(&world);
    assert_eq!(named(&said), 2, "one line per run of failures: {said}");
}

/// The members of the line of the record of the chat posts of `message_id`, as JSON.
fn record_line(world: &World, message_id: &str) -> Option<json::Value> {
    fs::read_dir(&world.dir.path)
        .expect("the directory is read")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            name.starts_with("bob.outbox.chat.") && !name.ends_with(".new")
        })
        .flat_map(|path| {
            fs::read_to_string(path)
                .unwrap_or_default()
                .lines()
                .filter_map(|line| json::parse(line).ok())
                .collect::<Vec<_>>()
        })
        .find(|line| text(line, "message_id") == message_id)
}

/// Item 2 of pass 2: the bridge's own line about a reply without text, listed by the provider
/// as a reply of the allowed chat user (a connection that posts as that user). Once a read has
/// passed it, `read_to` is at it and `own_lines` holds it no more; every later read starts after
/// it, so it is never read again and never sent as the person's answer.
#[test]
fn an_own_line_a_read_has_passed_is_never_read_again_as_the_persons_reply() {
    let world = world("adv2-answer-own-line-passed");
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let _bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    let request = post(&world, &url, "request", "Send the log of build 7.", &[]);
    let request_ts = posted(&world, &url, &request);
    let first = ("1700000000.900000".to_owned(), USER, "  ");
    thread(
        &connectors,
        &request_ts,
        std::slice::from_ref(&first),
        false,
    );
    wait_for("the line about the reply without text", || {
        connectors.replies() >= 1
    });
    let line = text(
        connectors
            .input(connectors.invocation("reply", 1))
            .member("body")
            .expect("a body"),
        "text",
    );
    assert_eq!(line, "the reply was not taken: it holds no text");
    // The reference the stand-in answered the post of the line with.
    let own = "1700000001.000001";
    thread(
        &connectors,
        &request_ts,
        &[first, (own.to_owned(), USER, line.as_str())],
        false,
    );
    wait_for("a read passes the own line", || {
        recorded(&world, &request, "read_to") == own
    });
    let reads = connectors.thread_reads();
    wait_for("2 more reads of the thread", || {
        connectors.thread_reads() >= reads + 2
    });
    let sent = forwarder.commands(0, "answer", &request);
    assert!(
        sent.is_empty(),
        "the bridge's own line was sent as the person's answer to {request}: {}",
        sent.iter()
            .map(|at| String::from_utf8_lossy(&forwarder.sent()[*at]).into_owned())
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!(
        connectors.replies(),
        1,
        "no line about the bridge's own line"
    );
    let entry = record_line(&world, &request).expect("the line of the request");
    assert!(
        entry.member("own_lines").is_none(),
        "a reference a read has passed leaves own_lines: {entry:?}"
    );
}
