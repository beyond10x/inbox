//! `inbox bridge`: the chat bridge (`docs/design/bridge.md`). It serves the inbox of a person
//! through a chat channel: it holds the caller's own `Pending` open in the rounds of
//! `inbox wake --follow` ([`wake::rounds`]: the reads with `undelivered=1`, the reconnect after
//! every answer, the tries, the pauses, the lock of the wake and the signals), and for each row in
//! `Sent` that is not due it makes one chat post through `connectors`, writes it into its record
//! of the chat posts, and then sends `MarkDelivered`. It holds no token of the chat provider and
//! starts no listener: `connectors` holds the connection.
//!
//! - The binding (a `ChatChannelBinding`) is given by 3 flags: the chat channel, the one chat
//!   user allowed to act on the posts (recorded and checked for its form; nothing compares it with
//!   a user yet), and the target of `connectors invoke`. One binding per bridge process; without
//!   one of the 3 it refuses to start, with one line and no request.
//! - `connectors` is the first executable of that name on `PATH`, looked up before anything is
//!   asked of the service. One child process per post ([`invoke`], whose one caller is [`post`]):
//!   `connectors --output json invoke <target> chat.postMessage --input-stdin
//!   --idempotency-key inbox-<message id>`, the input document `{"body":{"channel","text"}}` on
//!   its standard input, so no text of a message is on a command line or in the environment. The
//!   key is the message id after `inbox-`, so every try of one message repeats it. A post succeeds
//!   only when the child exits 0 and its answer on standard output is classified `applied` with
//!   the provider's status 200, `ok` true and the reference `ts`; anything else, a member missing
//!   or of another type included, is a failed post. On exit 1 the bridge reads the failure
//!   document of `connectors` on standard error for the classification and the code; each stream
//!   is read to a bound of 256 KiB.
//! - A post that succeeded is recorded beside the outbox file ([`Outbox::record_chat_post`])
//!   before its `MarkDelivered` is sent; a record that cannot be written ends the bridge with 4
//!   and no mark. A post that failed sends no mark: the round ends there, so the posts keep the
//!   order of `Pending`, and the next round starts after the pause of a failure ([`wake::pause`]);
//!   one line on standard error names the first failure of a run of them.
//! - The adapter is `slack`: the bridge opens the caller's inbox with it once it holds the lock,
//!   and ends with 1 when this host's record of an `inbox open` of the caller names another
//!   adapter, at the start before the lock and any request and in every round before its posts.
//!   `inbox wake --follow` ends with 1 on an inbox the bridge opened.
//!
//! The bridge issues and passes no approval proof, by the operator's decision: a released
//! `connectors` refuses its posts until it holds a standing approval of the operator for the post
//! operation into one channel (`docs/design/bridge.md`, "What stays open"). That entry closes the
//! input, so the input document holds `body.channel` and `body.text` and nothing else.
//!
//! [`Outbox::record_chat_post`]: crate::outbox::Outbox::record_chat_post

use std::io::{Read, Write as _};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::{CommandExt as _, ExitStatusExt as _};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};

use inbox::server::json::{self, Value};

use crate::commands::{self, Client};
use crate::failure::Failure;
use crate::output;
use crate::wake::{self, Surface};

/// The adapter of the inbox the bridge serves (Open 4 of the story: `slack`, the member of
/// `inbox.messaging.Adapter` the epic names).
pub const ADAPTER: &str = "slack";
/// The name each line of the bridge on standard error starts with.
const NAME: &str = "inbox bridge";
/// The program the bridge posts through.
const CONNECTORS: &str = "connectors";
/// The post operation of the chat provider's connector.
const OPERATION: &str = "chat.postMessage";

/// The flags of the binding, as given.
#[derive(Debug, Clone, Copy)]
pub struct Given<'a> {
    /// `--chat-channel`.
    pub chat_channel: Option<&'a str>,
    /// `--allowed-chat-user`.
    pub allowed_chat_user: Option<&'a str>,
    /// `--chat-connector`.
    pub chat_connector: Option<&'a str>,
}

/// A `ChatChannelBinding`: the chat channel the caller's inbox is posted into, the one chat user
/// allowed to act on the posts, and the target of `connectors invoke`.
#[derive(Debug)]
struct Binding {
    channel: String,
    #[expect(
        dead_code,
        reason = "stored and checked for its form; the bridge that reads reactions compares it"
    )]
    allowed_chat_user: String,
    connector: String,
}

impl Binding {
    /// The binding the 3 flags give.
    ///
    /// # Errors
    ///
    /// [`Failure::Usage`] when a flag is missing ("no binding is configured"), when a value is
    /// empty or holds white space or a control character, and when the target starts with `-`
    /// (`connectors` would read it as a flag).
    fn given(given: Given<'_>, inbox: &str) -> Result<Self, Failure> {
        let (Some(channel), Some(allowed_chat_user), Some(connector)) = (
            given.chat_channel,
            given.allowed_chat_user,
            given.chat_connector,
        ) else {
            return Err(Failure::Usage(format!(
                "no binding is configured for {inbox}: pass --chat-channel <id>, \
                 --allowed-chat-user <id> and --chat-connector <connector[/instance]>"
            )));
        };
        for (flag, value) in [
            ("--chat-channel", channel),
            ("--allowed-chat-user", allowed_chat_user),
            ("--chat-connector", connector),
        ] {
            if value.is_empty()
                || value
                    .chars()
                    .any(|character| character.is_whitespace() || character.is_control())
            {
                return Err(Failure::Usage(format!(
                    "{flag} `{value}` is not one id: it is empty, or holds white space or a \
                     control character"
                )));
            }
        }
        if connector.starts_with('-') {
            return Err(Failure::Usage(format!(
                "--chat-connector `{connector}` starts with `-`, which `connectors` reads as a flag"
            )));
        }
        Ok(Self {
            channel: channel.to_owned(),
            allowed_chat_user: allowed_chat_user.to_owned(),
            connector: connector.to_owned(),
        })
    }
}

/// Runs `inbox bridge` for `client` with the binding `given`, and answers its exit status. It
/// ends only by a signal or a failure.
pub fn run(client: &Client, json: bool, given: Given<'_>) -> i32 {
    match bridge(client, given) {
        Ok(status) => status,
        Err(failure) => report(&failure, json),
    }
}

/// Writes `failure` as every command does (the document with `--json`, one line on standard
/// error that starts with `inbox bridge: `) and answers its exit status.
pub fn report(failure: &Failure, json: bool) -> i32 {
    if json {
        output::print_document(&failure.document());
    }
    output::warn(&format!("{NAME}: {}", failure.message()));
    failure.status()
}

fn bridge(client: &Client, given: Given<'_>) -> Result<i32, Failure> {
    let binding = Binding::given(given, &client.caller.principal)?;
    let program = on_path(CONNECTORS).ok_or_else(|| {
        Failure::Local(format!(
            "`{CONNECTORS}` is not on PATH: the bridge posts through it, and nothing is asked of \
             the service without it"
        ))
    })?;
    wake::rounds(
        client,
        &mut Chat {
            binding,
            program,
            failing: false,
        },
    )
}

/// The first executable regular file named `name` in a directory of `PATH`.
fn on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(name))
        .find(|candidate| {
            std::fs::metadata(candidate)
                .is_ok_and(|found| found.is_file() && found.permissions().mode() & 0o111 != 0)
        })
}

/// The chat channel as the surface of the rounds of the wake.
struct Chat {
    binding: Binding,
    program: PathBuf,
    /// Whether the last post failed: a run of failures is named once on standard error.
    failing: bool,
}

impl Surface for Chat {
    fn name(&self) -> &'static str {
        NAME
    }

    fn follows(&self) -> bool {
        true
    }

    fn admits(&self, client: &Client) -> Result<(), Failure> {
        wake::only(
            client,
            ADAPTER,
            NAME,
            "to post its messages into the chat channel",
        )
    }

    /// Opens the caller's inbox with the adapter `slack`, once the lock is held.
    fn started(&mut self, client: &Client) -> Result<(), Failure> {
        commands::open(client, ADAPTER).map(drop)
    }

    /// Posts the rows in their order, each recorded before its `MarkDelivered`; a post that
    /// fails ends the round.
    fn deliver(
        &mut self,
        client: &Client,
        answer: &Value,
        _reconnected: Option<&str>,
    ) -> Result<(Option<Failure>, Vec<String>), Failure> {
        let mut first: Option<Failure> = None;
        let mut marked = Vec::new();
        for row in output::rows(answer) {
            let message_id = output::text(row, "message_id");
            let reference = match post(&self.binding, &self.program, row) {
                Ok(reference) => {
                    self.failing = false;
                    reference
                }
                Err(NotMade { why, wait }) => {
                    if !self.failing {
                        output::warn(&format!(
                            "{NAME}: the post of {message_id} into the chat channel {} failed: \
                             {why}; it stays in Sent and is tried again after a pause (no further \
                             line until a post succeeds)",
                            self.binding.channel
                        ));
                    }
                    self.failing = true;
                    if let Some(wait) = wait {
                        // `rate_limited`: no try of the message before the wait it named; the
                        // pause of a failure follows it.
                        std::thread::sleep(wait);
                    }
                    first.get_or_insert(Failure::Transport(why));
                    return Ok((first, marked));
                }
            };
            client
                .outbox
                .record_chat_post(&client.caller.principal, message_id, &reference)
                .map_err(|cause| {
                    Failure::Local(format!(
                        "{message_id} was posted into the chat channel and is not marked \
                         delivered: the record of the chat posts: {}",
                        cause.message()
                    ))
                })?;
            match commands::mark_delivered(client, message_id) {
                Ok(_) => marked.push(message_id.to_owned()),
                Err(failure) => {
                    output::warn(&format!(
                        "{NAME}: {message_id} was posted and not marked delivered: {}",
                        failure.message()
                    ));
                    first.get_or_insert(failure);
                }
            }
        }
        Ok((first, marked))
    }
}

/// The idempotency key of every post of the message `message_id`: the id after `inbox-`, so a
/// second try of the same message repeats it.
fn idempotency_key(message_id: &str) -> String {
    format!("inbox-{message_id}")
}

/// A post that was not made: why, for the line on standard error, and for `rate_limited` the
/// wait `connectors` asked for before the next try of the message.
#[derive(Debug, PartialEq, Eq)]
struct NotMade {
    why: String,
    wait: Option<Duration>,
}

impl NotMade {
    fn because(why: String) -> Self {
        Self { why, wait: None }
    }
}

/// The longest wait before the next try that a `retry_after_seconds` of `connectors` gives: one
/// hour, the span of its cap of writes. A value outside 1 to 3600, or one that is not a whole
/// number, gives no wait of its own: the bridge's own pause of a failure applies.
const MOST_RETRY_AFTER: u64 = 3600;

/// The most bytes the bridge reads of each of the standard output and the standard error of one
/// run of `connectors` (256 KiB): an answer of a post is a few KiB. The bytes after the bound are
/// read and dropped, so the child is not held, and a stream over the bound is a failed post.
const MOST_ANSWER_BYTES: usize = 256 * 1024;

/// What the bridge took of one stream of a run of `connectors`: at most [`MOST_ANSWER_BYTES`]
/// bytes, and whether there were more.
#[derive(Debug, Default)]
struct Taken {
    bytes: Vec<u8>,
    over: bool,
}

/// One run of `connectors`: its exit status and what the bridge took of its 2 streams.
#[derive(Debug)]
struct Ran {
    status: ExitStatus,
    stdout: Taken,
    stderr: Taken,
}

/// One chat post of `row` into the channel of `binding`: the input document, one invocation of
/// `connectors`, and the reference of the post its answer names ([`judged`]). This is the one
/// caller of [`invoke`].
///
/// # Errors
///
/// Why the post was not made.
fn post(binding: &Binding, program: &Path, row: &Value) -> Result<String, NotMade> {
    let mut input = String::from("{");
    json::member(&mut input, "body");
    input.push('{');
    json::member(&mut input, "channel");
    json::push_text(&mut input, &binding.channel);
    json::member(&mut input, "text");
    json::push_text(&mut input, &output::chat_post(row));
    input.push_str("}}");
    let key = idempotency_key(output::text(row, "message_id"));
    let ran = invoke(program, &binding.connector, &key, &input).map_err(NotMade::because)?;
    judged(&ran)
}

/// How often the bridge looks whether its `connectors` child has ended while it waits for it,
/// unless a stream of the child ends first. The child is held in [`wake::CHILD`] so the signal
/// path can end it; a blocking wait would hold it from there.
const LOOK: Duration = Duration::from_millis(10);

/// How long the bridge goes on reading the streams of a `connectors` child that has exited: the
/// child's own bytes are in the pipes when it exits, so this ends at once unless another process
/// (a helper the child started) holds a pipe open. Then the post is judged by what was read.
const DRAIN: Duration = Duration::from_secs(1);

/// Starts `connectors --output json invoke <target> chat.postMessage --input-stdin
/// --idempotency-key <key>` with `input` on its standard input and waits for its exit. Its
/// standard output and standard error are captured ([`output::captured`]) and read by threads of
/// their own to [`MOST_ANSWER_BYTES`] each ([`read_into`]), never shown as they are. The exit of
/// the child ends the post: after it the streams are read for at most [`DRAIN`], so a process
/// that inherited the pipes holds nothing. The child is the leader of a process group of its own;
/// while it runs it is in [`wake::CHILD`], where a SIGINT or SIGTERM of the bridge ends its group
/// before the bridge ends.
///
/// # Errors
///
/// It did not start, was not waited for, or was ended by the signal path.
fn invoke(program: &Path, target: &str, key: &str, input: &str) -> Result<Ran, String> {
    let mut command = Command::new(program);
    command
        .args([
            "--output",
            "json",
            "invoke",
            target,
            OPERATION,
            "--input-stdin",
            "--idempotency-key",
            key,
        ])
        .stdin(Stdio::piped())
        // The leader of a process group of its own: the signal path ends the group, helpers
        // the child started included, and the interrupt key of a terminal reaches the bridge
        // alone.
        .process_group(0);
    output::captured(&mut command);
    let mut held = wake::child();
    let mut child = command
        .spawn()
        .map_err(|error| format!("`{CONNECTORS}` did not start: {error}"))?;
    let stdin = child.stdin.take();
    let streams = [
        child
            .stdout
            .take()
            .map(|pipe| Box::new(pipe) as Box<dyn Read + Send>),
        child
            .stderr
            .take()
            .map(|pipe| Box::new(pipe) as Box<dyn Read + Send>),
    ];
    *held = Some(child);
    drop(held);
    if let Some(mut pipe) = stdin {
        let input = input.to_owned();
        // A child that exits without reading its input is judged by its exit and its answer.
        std::thread::spawn(move || {
            let _ = pipe.write_all(input.as_bytes());
        });
    }
    let (ended, ends) = mpsc::channel();
    let [stdout, stderr] = streams.map(|stream| {
        let taken = Arc::new(Mutex::new(Taken::default()));
        if let Some(pipe) = stream {
            let into = Arc::clone(&taken);
            let ended = ended.clone();
            std::thread::spawn(move || {
                read_into(pipe, &into);
                let _ = ended.send(());
            });
        } else {
            let _ = ended.send(());
        }
        taken
    });
    drop(ended);
    let mut streams_ended = 0;
    let status = loop {
        {
            let mut held = wake::child();
            let Some(child) = held.as_mut() else {
                return Err(format!("`{CONNECTORS}` was ended as the bridge ends"));
            };
            match child.try_wait() {
                Ok(Some(status)) => {
                    held.take();
                    break status;
                }
                Ok(None) => {}
                Err(error) => {
                    if let Some(mut child) = held.take() {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                    return Err(format!("`{CONNECTORS}` was not waited for: {error}"));
                }
            }
        }
        match ends.recv_timeout(LOOK) {
            Ok(()) => streams_ended += 1,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => std::thread::sleep(LOOK),
        }
    };
    let drained = Instant::now() + DRAIN;
    while streams_ended < 2 {
        let left = drained.saturating_duration_since(Instant::now());
        if left.is_zero() || ends.recv_timeout(left).is_err() {
            break;
        }
        streams_ended += 1;
    }
    let took = |taken: &Mutex<Taken>| {
        std::mem::take(&mut *taken.lock().unwrap_or_else(PoisonError::into_inner))
    };
    Ok(Ran {
        status,
        stdout: took(&stdout),
        stderr: took(&stderr),
    })
}

/// Reads `pipe` to its end into `into`: at most [`MOST_ANSWER_BYTES`] are kept, the rest is read
/// and dropped, so the child is not held by a full pipe. A read that fails ends it.
fn read_into(mut pipe: impl Read, into: &Mutex<Taken>) {
    let mut buffer = [0_u8; 8192];
    loop {
        let read = match pipe.read(&mut buffer) {
            Ok(0) => return,
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return,
        };
        let mut taken = into.lock().unwrap_or_else(PoisonError::into_inner);
        let room = MOST_ANSWER_BYTES - taken.bytes.len();
        if read > room {
            taken.bytes.extend_from_slice(&buffer[..room]);
            taken.over = true;
        } else {
            taken.bytes.extend_from_slice(&buffer[..read]);
        }
    }
}

/// The first line of `taken` that is not empty, cut to 200 characters: text of another party,
/// which reaches standard error only through [`output::warn`], which shows each control character
/// as its escape.
fn said(taken: &Taken) -> Option<String> {
    String::from_utf8_lossy(&taken.bytes)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.chars().take(200).collect())
}

/// `what` and, after a colon, the first line of `taken` if there is one.
fn with_said(what: String, taken: &Taken) -> String {
    match said(taken) {
        Some(line) => format!("{what}: {line}"),
        None => what,
    }
}

/// The one JSON document `taken` holds.
///
/// # Errors
///
/// What it is instead: longer than the bound, empty, or not one JSON document (with its first
/// line).
fn document(taken: &Taken) -> Result<Value, String> {
    if taken.over {
        return Err(format!("longer than {MOST_ANSWER_BYTES} bytes"));
    }
    std::str::from_utf8(&taken.bytes)
        .ok()
        .and_then(|text| json::parse(text.trim()).ok())
        .ok_or_else(|| with_said("not one JSON document".to_owned(), taken))
}

/// What one run of `connectors` came to, read by its exit status. A post is made only when it
/// exited 0 and its standard output holds an answer classified `applied` with the provider's
/// status 200, `ok` and a `ts` ([`applied`]). Every other run is a failed post, named by what it
/// was: an exit 0 with another standard output; an exit 1 with its failure document on standard
/// error ([`failed`]) or with a standard error that is not one; an exit 2, a usage error; another
/// exit; a child ended by a signal.
fn judged(ran: &Ran) -> Result<String, NotMade> {
    let Some(code) = ran.status.code() else {
        return Err(NotMade::because(match ran.status.signal() {
            Some(signal) => format!("`{CONNECTORS}` was ended by the signal {signal}"),
            None => format!("`{CONNECTORS}` ended without an exit status"),
        }));
    };
    match code {
        0 => {
            let answer = document(&ran.stdout).map_err(|what| {
                NotMade::because(format!(
                    "`{CONNECTORS}` exited with 0 and its standard output is {what}"
                ))
            })?;
            applied(&answer).map(str::to_owned).map_err(|lacks| {
                NotMade::because(format!(
                    "`{CONNECTORS}` exited with 0 and its answer is not a post made: {lacks}"
                ))
            })
        }
        1 => Err(match document(&ran.stderr) {
            Ok(failure) => failed(&failure),
            Err(what) => NotMade::because(format!(
                "`{CONNECTORS}` exited with 1 and its standard error is {what}"
            )),
        }),
        2 => Err(NotMade::because(with_said(
            format!("`{CONNECTORS}` exited with 2, a usage error"),
            &ran.stderr,
        ))),
        other => Err(NotMade::because(with_said(
            format!("`{CONNECTORS}` exited with {other}"),
            &ran.stderr,
        ))),
    }
}

/// The text member `value` holds, if it is one and not empty, cut to 200 characters.
fn text_of(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::Text(text)) if !text.is_empty() => Some(text.chars().take(200).collect()),
        _ => None,
    }
}

/// The post not made that the failure document of `connectors` (exit 1, on standard error) names:
/// `ok` false and `error.code` the fixed `failure`; the code in `error.data.code`, the
/// classification in `error.data.mutation.classification`, and its message in
/// `error.data.reason` or `error.message`. The line names the classification and the code, and
/// what each of the codes of the operator's grant means; for `rate_limited` the wait is its
/// `error.data.retry_after_seconds` ([`retry_after`]). Members the bridge does not know are left
/// alone.
fn failed(failure: &Value) -> NotMade {
    let error = failure.member("error");
    if !matches!(failure.member("ok"), Some(Value::Bool(false)))
        || !matches!(error.and_then(|error| error.member("code")),
            Some(Value::Text(code)) if code == "failure")
    {
        return NotMade::because(format!(
            "`{CONNECTORS}` exited with 1 and its standard error is not a failure document (`ok` \
             false, `error.code` `failure`)"
        ));
    }
    let data = error.and_then(|error| error.member("data"));
    let classification = text_of(
        data.and_then(|data| data.member("mutation"))
            .and_then(|mutation| mutation.member("classification")),
    );
    let code = text_of(data.and_then(|data| data.member("code")));
    let mut why = format!("`{CONNECTORS}` exited with 1: ");
    match &classification {
        Some(classification) => why.push_str(&format!("the post is classified `{classification}`")),
        None => why.push_str("its failure names no classification of the post"),
    }
    match &code {
        Some(code) => why.push_str(&format!(" with the code `{code}`")),
        None => why.push_str(" and no code"),
    }
    if let Some(reason) = text_of(data.and_then(|data| data.member("reason")))
        .or_else(|| text_of(error.and_then(|error| error.member("message"))))
    {
        why.push_str(&format!(": {reason}"));
    }
    let mut wait = None;
    match code.as_deref() {
        Some("approval_required") => why.push_str(
            "; the operator's grant of `connectors` is missing, ended or does not cover this post",
        ),
        Some("idempotency_conflict") => {
            why.push_str("; its idempotency key was used before with another input document");
        }
        Some("unavailable") => why.push_str(
            "; `connectors` says its approval clock cannot be read or its grant file is unsafe",
        ),
        Some("rate_limited") => {
            wait = data.and_then(retry_after);
            match wait {
                Some(wait) => why.push_str(&format!(
                    "; the next try is not before {} s",
                    wait.as_secs()
                )),
                None => why
                    .push_str("; it names no wait of 1 to 3600 s, the pause of a failure applies"),
            }
        }
        _ => {}
    }
    if classification.as_deref() == Some("unknown") {
        why.push_str(
            "; whether the post was made is not known, and the next try with the same key gets \
             the outcome of the first",
        );
    }
    NotMade { why, wait }
}

/// The wait `retry_after_seconds` of `data` gives: a whole number of seconds written in digits
/// only, from 1 to [`MOST_RETRY_AFTER`]. `None` for every other value and when there is none.
fn retry_after(data: &Value) -> Option<Duration> {
    match data.member("retry_after_seconds") {
        Some(Value::Number(spelling)) if spelling.bytes().all(|byte| byte.is_ascii_digit()) => {
            spelling
                .parse::<u64>()
                .ok()
                .filter(|seconds| (1..=MOST_RETRY_AFTER).contains(seconds))
                .map(Duration::from_secs)
        }
        _ => None,
    }
}

/// The reference of the post an answer of `connectors --output json` names when the post was
/// made: `result.mutation.classification` is `applied`, `result.result.status` 200,
/// `result.result.body.ok` true, and `result.result.body.ts` a text that is not empty. Members
/// the bridge does not know are left alone.
///
/// # Errors
///
/// Which of these the answer lacks: a member missing or of another type is a failed post.
fn applied(answer: &Value) -> Result<&str, String> {
    let result = answer.member("result");
    if !matches!(result
        .and_then(|result| result.member("mutation"))
        .and_then(|mutation| mutation.member("classification")),
        Some(Value::Text(classification)) if classification == "applied")
    {
        return Err("the post is not classified `applied`".to_owned());
    }
    let provider = result.and_then(|result| result.member("result"));
    if !matches!(provider.and_then(|provider| provider.member("status")),
        Some(Value::Number(status)) if status == "200")
    {
        return Err("the answer names no status 200 of the provider".to_owned());
    }
    let body = provider.and_then(|provider| provider.member("body"));
    if !matches!(
        body.and_then(|body| body.member("ok")),
        Some(Value::Bool(true))
    ) {
        return Err("the provider's answer is not `ok`".to_owned());
    }
    match body.and_then(|body| body.member("ts")) {
        Some(Value::Text(reference)) if !reference.is_empty() => Ok(reference),
        _ => Err("the provider's answer names no reference of the post (`ts`)".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use inbox::server::json;

    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;
    use std::time::Duration;

    use std::sync::Mutex;

    use super::{
        Binding, Given, MOST_ANSWER_BYTES, NotMade, Ran, Taken, applied, judged, read_into,
    };

    /// What [`read_into`] takes of `bytes`.
    fn taken(bytes: &[u8]) -> Taken {
        let into = Mutex::new(Taken::default());
        read_into(bytes, &into);
        into.into_inner().unwrap()
    }

    fn binding(channel: &str, user: &str, connector: &str) -> Result<Binding, String> {
        Binding::given(
            Given {
                chat_channel: Some(channel),
                allowed_chat_user: Some(user),
                chat_connector: Some(connector),
            },
            "human:person",
        )
        .map_err(|failure| failure.message())
    }

    #[test]
    fn a_connector_that_starts_with_a_dash_is_refused() {
        assert!(binding("C0000000001", "U0000000001", "slack").is_ok());
        assert!(binding("C0000000001", "U0000000001", "slack/instance-1").is_ok());
        let refused = binding("C0000000001", "U0000000001", "--input-file=x").unwrap_err();
        assert!(refused.contains("starts with `-`"), "{refused}");
    }

    /// Only an answer classified `applied` with the status 200, `ok` true and a `ts` is a post
    /// made; every other shape, a member of another type included, is a failed post.
    #[test]
    fn only_an_applied_answer_with_a_reference_is_a_post_made() {
        let made = r#"{"result":{"mutation":{"classification":"applied"},"result":{"status":200,"body":{"ok":true,"ts":"1.2"}}}}"#;
        assert_eq!(applied(&json::parse(made).unwrap()), Ok("1.2"));
        for failed in [
            r#"{}"#,
            r#"{"result":null}"#,
            r#"{"result":{"mutation":{"classification":"unknown"},"result":{"status":200,"body":{"ok":true,"ts":"1.2"}}}}"#,
            r#"{"result":{"mutation":{"classification":"not_attempted"},"result":{"status":200,"body":{"ok":true,"ts":"1.2"}}}}"#,
            r#"{"result":{"mutation":{"classification":["applied"]},"result":{"status":200,"body":{"ok":true,"ts":"1.2"}}}}"#,
            r#"{"result":{"mutation":{"classification":"applied"},"result":{"status":"200","body":{"ok":true,"ts":"1.2"}}}}"#,
            r#"{"result":{"mutation":{"classification":"applied"},"result":{"status":429,"body":{"ok":true,"ts":"1.2"}}}}"#,
            r#"{"result":{"mutation":{"classification":"applied"},"result":{"status":200,"body":{"ok":false,"ts":"1.2"}}}}"#,
            r#"{"result":{"mutation":{"classification":"applied"},"result":{"status":200,"body":{"ok":"true","ts":"1.2"}}}}"#,
            r#"{"result":{"mutation":{"classification":"applied"},"result":{"status":200,"body":{"ok":true}}}}"#,
            r#"{"result":{"mutation":{"classification":"applied"},"result":{"status":200,"body":{"ok":true,"ts":""}}}}"#,
            r#"{"result":{"mutation":{"classification":"applied"},"result":{"status":200,"body":{"ok":true,"ts":12}}}}"#,
        ] {
            assert!(applied(&json::parse(failed).unwrap()).is_err(), "{failed}");
        }
    }

    /// A run of `connectors` as `judged` gets it: the raw wait status `raw`, `stdout` and
    /// `stderr`.
    fn ran(raw: i32, stdout: &str, stderr: &str) -> Ran {
        let whole = |text: &str| Taken {
            bytes: text.as_bytes().to_vec(),
            over: false,
        };
        Ran {
            status: ExitStatus::from_raw(raw),
            stdout: whole(stdout),
            stderr: whole(stderr),
        }
    }

    /// A post not made by a run that exited `code`.
    fn not_made(code: i32, stdout: &str, stderr: &str) -> NotMade {
        judged(&ran(code << 8, stdout, stderr)).unwrap_err()
    }

    const MADE: &str = r#"{"result":{"mutation":{"classification":"applied","standing":{"grant_id":"g"}},"result":{"status":200,"body":{"ok":true,"ts":"1.2"}}}}"#;

    /// The failure document of `connectors` with the classification and the code given and
    /// `extra` members of `error.data`.
    fn failure(classification: &str, code: &str, extra: &str) -> String {
        format!(
            r#"{{"ok":false,"error":{{"code":"failure","message":"m","data":{{"code":"{code}","mutation":{{"classification":"{classification}","standing":{{"grant_id":"g"}}}}{extra}}}}}}}"#
        )
    }

    /// A post is read from standard output on exit 0 and from standard error on exit 1, each
    /// in its own shape; the code is `error.data.code`, the classification
    /// `error.data.mutation.classification`; `retry_after_seconds` gives the wait of
    /// `rate_limited` only as whole seconds from 1 to 3600; members the bridge does not know are
    /// left alone.
    #[test]
    fn a_post_not_made_names_its_code_and_a_rate_limit_its_bounded_wait() {
        assert_eq!(judged(&ran(0, MADE, "")), Ok("1.2".to_owned()));
        assert!(
            not_made(1, MADE, "")
                .why
                .contains("exited with 1 and its standard error is not one JSON document")
        );
        let approval = not_made(1, "", &failure("not_attempted", "approval_required", ""));
        assert!(
            approval
                .why
                .contains("`not_attempted` with the code `approval_required`: m"),
            "{approval:?}"
        );
        assert!(approval.why.contains("grant"), "{approval:?}");
        assert_eq!(approval.wait, None);
        let conflict = not_made(1, "", &failure("not_attempted", "idempotency_conflict", ""));
        assert!(
            conflict.why.contains("another input document"),
            "{conflict:?}"
        );
        let unknown = not_made(1, "", &failure("unknown", "provider_timeout", ""));
        assert!(unknown.why.contains("is not known"), "{unknown:?}");
        let unavailable = not_made(
            1,
            "",
            r#"{"ok":false,"error":{"code":"failure","data":{"code":"unavailable","mutation":{"classification":"not_attempted"}}}}"#,
        );
        assert!(
            unavailable
                .why
                .contains("`not_attempted` with the code `unavailable`"),
            "{unavailable:?}"
        );
        assert_eq!(unavailable.wait, None);
        let unnamed = not_made(
            1,
            "",
            &failure("not_attempted", "a_code_of_a_later_release", ""),
        );
        assert!(
            unnamed.why.contains("the code `a_code_of_a_later_release`"),
            "{unnamed:?}"
        );
        for (seconds, wait) in [
            ("7", Some(Duration::from_secs(7))),
            ("1", Some(Duration::from_secs(1))),
            ("3600", Some(Duration::from_secs(3600))),
            ("0", None),
            ("3601", None),
            ("-1", None),
            ("1.5", None),
            ("1e3", None),
            ("\"7\"", None),
        ] {
            let limited = not_made(
                1,
                "",
                &failure(
                    "not_attempted",
                    "rate_limited",
                    &format!(r#","retry_after_seconds":{seconds}"#),
                ),
            );
            assert_eq!(limited.wait, wait, "{seconds}: {limited:?}");
            assert!(limited.why.contains("`rate_limited`"), "{limited:?}");
        }
        assert!(
            not_made(1, "", r#"{"ok":true,"error":{"code":"failure"}}"#)
                .why
                .contains("not a failure document")
        );
        assert!(
            not_made(0, "", &failure("not_attempted", "approval_required", ""))
                .why
                .contains("exited with 0 and its standard output is not one JSON document")
        );
        assert_eq!(
            not_made(2, MADE, "bad flag\n").why,
            "`connectors` exited with 2, a usage error: bad flag"
        );
        assert_eq!(
            judged(&ran(9, MADE, "")).unwrap_err().why,
            "`connectors` was ended by the signal 9"
        );
    }

    /// Each stream is read to the bound and the rest dropped; a stream over the bound is a failed
    /// post however it starts.
    #[test]
    fn a_stream_over_the_bound_is_read_to_its_end_and_is_a_failed_post() {
        let long = format!("{MADE}{}", " ".repeat(MOST_ANSWER_BYTES));
        let read = taken(long.as_bytes());
        assert!(read.over);
        assert_eq!(read.bytes.len(), MOST_ANSWER_BYTES);
        let whole = taken(MADE.as_bytes());
        assert!(!whole.over);
        let run = Ran {
            status: ExitStatus::from_raw(0),
            stdout: read,
            stderr: Taken::default(),
        };
        assert_eq!(
            judged(&run).unwrap_err().why,
            format!(
                "`connectors` exited with 0 and its standard output is longer than \
                 {MOST_ANSWER_BYTES} bytes"
            )
        );
    }
}
