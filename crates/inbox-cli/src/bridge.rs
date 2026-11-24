//! `inbox bridge`: the chat bridge (`docs/design/bridge.md`). It serves the inbox of a person
//! through a chat channel: it holds the caller's own `Pending` open in the rounds of
//! `inbox wake --follow` ([`wake::rounds`]: the reads with `undelivered=1`, the reconnect after
//! every answer, the tries, the pauses, the lock of the wake and the signals), and for each row in
//! `Sent` that is not due it makes one chat post through `connectors`, writes it into its record
//! of the chat posts, and then sends `MarkDelivered`. A thread of it reads the recent posts of the
//! channel at an interval and marks read each message whose post the allowed chat user reacted to
//! ([`Poll`]). It holds no token of the chat provider and starts no listener: `connectors` holds
//! the connection.
//!
//! - The binding (a `ChatChannelBinding`) is given by 3 flags: the chat channel, the one chat
//!   user allowed to act on the posts (compared with the user ids of a reaction), and the target
//!   of `connectors invoke`. One binding per bridge process; without one of the 3 it refuses to
//!   start, with one line and no request.
//! - `connectors` is the first executable of that name on `PATH`, looked up before anything is
//!   asked of the service. One child process per post ([`invoke`], whose callers are [`post`] and
//!   the read of a page of the channel, one child at a time by the turn, [`TURN`]):
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
//! - The read mark (story:the-chat-bridge-marks-a-message-read-on-a-reaction, [`Poll`]): once the
//!   inbox is open, the thread `inbox-bridge-poll` polls at once and then every
//!   `--chat-poll-interval` seconds. A poll with no `posted` entry in the record starts no child;
//!   else it reads the channel from the oldest `posted` entry on (`conversations.history`, at most
//!   [`MOST_PAGES`] pages), and sends the `read` of `inbox read` for each `posted` entry whose post
//!   the allowed chat user reacted to; the entry becomes `read`, or `due` on `deadline-passed`, and
//!   is never asked about again. A poll that fails marks nothing more and is tried again after the
//!   pause of a failure; one line names a run of them. It never stops the rounds.
//!
//! The bridge issues and passes no approval proof, by the operator's decision: a released
//! `connectors` refuses its posts until it holds a standing approval of the operator for the post
//! operation into one channel (`docs/design/bridge.md`, "What stays open"). That entry closes the
//! input, so the input document holds `body.channel` and `body.text` and nothing else.
//!
//! [`Outbox::record_chat_post`]: crate::outbox::Outbox::record_chat_post

use std::collections::HashSet;
use std::io::{Read, Write as _};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::{CommandExt as _, ExitStatusExt as _};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};

use inbox::server::json::{self, Value};
use inbox_server::{Clock, SystemClock, format_reading};
use inbox_verify::Instant as Reading;

use crate::client::declared_error;
use crate::clock_file::FileClock;
use crate::commands::{self, Client};
use crate::failure::Failure;
use crate::outbox::{self, ChatPost};
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
/// The read operation of the chat provider's connector: the recent posts of a channel, newest
/// first, each with its reactions.
const READ_OPERATION: &str = "conversations.history";
/// The posts one page of a read asks for.
const PAGE_LIMIT: u32 = 100;
/// The most pages one poll reads: 1000 posts since the oldest open post. A channel with a further
/// page then is a failed poll, which marks nothing.
const MOST_PAGES: usize = 10;

/// The flags of the binding, as given.
#[derive(Debug, Clone, Copy)]
pub struct Given<'a> {
    /// `--chat-channel`.
    pub chat_channel: Option<&'a str>,
    /// `--allowed-chat-user`.
    pub allowed_chat_user: Option<&'a str>,
    /// `--chat-connector`.
    pub chat_connector: Option<&'a str>,
    /// `--chat-poll-interval`, in seconds.
    pub chat_poll_interval: u64,
    /// `--chat-post-retention`, in seconds.
    pub chat_post_retention: u64,
    /// `--clock-file`, a debug build's only; the system clock where absent.
    pub clock_file: Option<&'a Path>,
    /// `--authority`: the id of the delegation the bridge acts for the inbox of `--for` under,
    /// named in the line of a refused key ([`refused_key`]); given with `--for` only.
    pub authority: Option<&'a str>,
}

/// A `ChatChannelBinding`: the chat channel the caller's inbox is posted into, the one chat user
/// allowed to act on the posts, and the target of `connectors invoke`.
#[derive(Debug, Clone)]
struct Binding {
    channel: String,
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
/// ends only by a signal or a failure. With `--for` and `--authority` it ends the process on the
/// first answer `401` ([`refused_key`]).
pub fn run(client: Client, json: bool, given: Given<'_>) -> i32 {
    let client = match given.authority {
        Some(authority) if client.acts_for_another() => {
            let end = refused_key(&client, authority, json);
            client.ending_on_refused_key(end)
        }
        _ => client,
    };
    match bridge(&client, given) {
        Ok(status) => status,
        Err(failure) => report(&failure, json),
    }
}

/// The end of a bridge with `--for` whose request the service answered `401`
/// (story:the-chat-bridge-runs-with-its-own-key-under-a-delegation, Open 4 A): the service gives
/// that answer alike for a key entry and a delegation that are missing or lapsed in its key file,
/// so the one line names both ids and both commands, and that the service reads the key file at
/// its start. It takes the slot of the `connectors` child and holds it to the end, ends a running
/// child with its group as the signal path does ([`wake::end_group`]), writes the failure
/// ([`report`]) and exits 1: no post and no further request. The ids come as typed
/// (`--authority`) and as the secret key file holds them (the key id, the principal), checked by
/// nobody on the client: each is written with its line ends as `\u{a}` ([`output::inline`]), and
/// the line as every line ([`output::warn`]: each other control character, each character that
/// changes the direction of text and each one a reader cannot see as its escape), so it stays one
/// line in the order it was written.
fn refused_key(client: &Client, authority: &str, json: bool) -> commands::RefusedKey {
    let key_id = output::inline(&client.caller.key_id).into_owned();
    let principal = output::inline(&client.caller.principal).into_owned();
    let inbox = output::inline(&client.own_inbox().0).into_owned();
    let authority = output::inline(authority).into_owned();
    Arc::new(move || {
        let mut held = wake::child();
        if let Some(running) = held.take() {
            wake::end_group(running);
        }
        let failure = Failure::KeyRefused(format!(
            "the service refused the key {key_id} acting for {inbox} under the delegation \
             {authority} (401): its key file holds no live key entry {key_id} or no live \
             delegation {authority} of {principal} for {inbox}; add the missing one with `inbox \
             keyfile add-key` or `inbox keyfile add-delegation` and start the service again: it \
             reads the key file at its start only"
        ));
        let status = report(&failure, json);
        std::process::exit(status);
    })
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
    let binding = Binding::given(given, &client.own_inbox().0)?;
    let program = on_path(CONNECTORS).ok_or_else(|| {
        Failure::Local(format!(
            "`{CONNECTORS}` is not on PATH: the bridge posts through it, and nothing is asked of \
             the service without it"
        ))
    })?;
    let clock: Arc<dyn Clock> = match given.clock_file {
        Some(path) => Arc::new(FileClock::of(NAME, path).map_err(|cause| {
            Failure::Local(format!("--clock-file {}: {cause}", path.display()))
        })?),
        None => Arc::new(SystemClock),
    };
    let poll = Poll {
        client: client.again()?,
        binding: binding.clone(),
        program: program.clone(),
        interval: Duration::from_secs(given.chat_poll_interval),
        retention: i64::try_from(given.chat_post_retention).unwrap_or(i64::MAX),
        clock,
        failing_end: false,
    };
    wake::rounds(
        client,
        &mut Chat {
            binding,
            program,
            failing: false,
            poll: Some(poll),
        },
    )
}

/// The turn of the `connectors` children of the bridge: [`invoke`] holds it from before a child
/// starts to its end, so a post and a read of the channel never run at once: each waits for the
/// other. A lock inside the process: the lock of the wake already keeps a second bridge of the
/// inbox and outbox file off the host, and the 2 threads of one bridge share the process.
static TURN: Mutex<()> = Mutex::new(());

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
    /// The poll, until its thread is started once the inbox is open.
    poll: Option<Poll>,
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

    /// Opens the caller's inbox with the adapter `slack`, once the lock is held, then starts the
    /// thread of the poll.
    fn started(&mut self, client: &Client) -> Result<(), Failure> {
        commands::open(client, ADAPTER)?;
        if let Some(poll) = self.poll.take() {
            std::thread::Builder::new()
                .name("inbox-bridge-poll".to_owned())
                .spawn(move || poll.run())
                .map_err(|error| Failure::Local(format!("the thread of the poll: {error}")))?;
        }
        Ok(())
    }

    /// Posts the rows in their order, each recorded before its `MarkDelivered`; a post that
    /// fails ends the round. A row whose message id the record holds, of any channel, is not
    /// posted: it is marked delivered, and read when its entry is `read` ([`Chat::again`]). A
    /// record that cannot be read ends the bridge with 4 before any post or mark of the round.
    fn deliver(
        &mut self,
        client: &Client,
        answer: &Value,
        _reconnected: Option<&str>,
    ) -> Result<(Option<Failure>, Vec<String>), Failure> {
        let mut first: Option<Failure> = None;
        let mut marked = Vec::new();
        // No post without a readable record: a message it holds would be posted a second time.
        // The bridge ends with 4, as after a post whose record cannot be written.
        let held = client
            .outbox
            .chat_posts(&client.own_inbox().0)
            .map_err(|failure| {
                let message_id = output::rows(answer)
                    .first()
                    .map(|row| output::text(row, "message_id"))
                    .unwrap_or_default();
                Failure::Local(format!(
                    "{message_id} is not posted into the chat channel and not marked delivered: \
                     the record of the chat posts cannot be read: {}",
                    failure.message()
                ))
            })?;
        for row in output::rows(answer) {
            let message_id = output::text(row, "message_id");
            if let Some(entry) = held.iter().find(|entry| entry.message_id == message_id) {
                match Self::again(client, entry) {
                    Ok(()) => marked.push(message_id.to_owned()),
                    Err(failure) => {
                        first.get_or_insert(failure);
                    }
                }
                continue;
            }
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
            let given = |name: &str| {
                Some(output::text(row, name).to_owned()).filter(|text| !text.is_empty())
            };
            client
                .outbox
                .record_chat_post(
                    &client.own_inbox().0,
                    &ChatPost {
                        message_id: message_id.to_owned(),
                        channel: Some(self.binding.channel.clone()),
                        reference,
                        state: outbox::POSTED.to_owned(),
                        thread: given("thread"),
                        expires: given("expires"),
                        end: None,
                        gone_at: None,
                    },
                )
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

impl Chat {
    /// A row in `Sent` whose message id the record holds in `entry` (a repost after a restart of
    /// the service, or a message a bridge before this one posted): no post. `MarkDelivered`; then
    /// an end `gone` or `answered` and a `gone_at` are taken off the entry, since its message is
    /// live again; then, when the entry is `read`, the `read` of `inbox read`
    /// (the service answers a note `handled` and removes it). A `read` that fails is named in one
    /// line and sent again by the poll, which sends it while the entry is `read` and `Pending`
    /// lists the message in `Delivered` ([`Poll::once`]). The entry keeps its reference and its
    /// channel.
    ///
    /// # Errors
    ///
    /// The `MarkDelivered` that failed, named in one line.
    fn again(client: &Client, entry: &ChatPost) -> Result<(), Failure> {
        let message_id = &entry.message_id;
        if let Err(failure) = commands::mark_delivered(client, message_id) {
            output::warn(&format!(
                "{NAME}: {message_id} was posted before and is not marked delivered: {}",
                failure.message()
            ));
            return Err(failure);
        }
        let ended_before = matches!(entry.end.as_deref(), Some(outbox::GONE | outbox::ANSWERED));
        if ended_before || entry.gone_at.is_some() {
            let live = ChatPost {
                end: if ended_before {
                    None
                } else {
                    entry.end.clone()
                },
                gone_at: None,
                ..entry.clone()
            };
            if let Err(failure) =
                client
                    .outbox
                    .change_chat_post(&client.own_inbox().0, entry, Some(&live))
            {
                output::warn(&format!(
                    "{NAME}: {message_id} is live again, and the record of the chat posts could \
                     not say so: {}",
                    failure.message()
                ));
            }
        }
        if entry.state == outbox::READ {
            match commands::read(client, message_id) {
                Ok(_) => {}
                Err(Failure::Refused { answer, .. }) if declared_error(&answer).is_some() => {}
                Err(failure) => output::warn(&format!(
                    "{NAME}: {message_id} was read before and is not marked read again yet: {}; \
                     the poll sends it again",
                    failure.message()
                )),
            }
        }
        Ok(())
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

/// One chat post of `row` into the channel of `binding` ([`write`]), under the key of its message.
///
/// # Errors
///
/// Why the post was not made.
fn post(binding: &Binding, program: &Path, row: &Value) -> Result<String, NotMade> {
    let key = idempotency_key(output::text(row, "message_id"));
    write(binding, program, &output::chat_post(row), None, &key)
}

/// The idempotency key of the line at the end of the message `message_id`: another key than its
/// post's, the same at every try.
fn end_key(message_id: &str) -> String {
    format!("inbox-{message_id}-end")
}

/// One write of the post operation into the channel of `binding`: the input document
/// `{"body":{"channel","text"}}`, with `thread_ts` the reference of the post a reply goes into,
/// one invocation of `connectors` under the idempotency key `key`, and the reference of the post
/// its answer names ([`judged`]).
///
/// # Errors
///
/// Why the post was not made.
fn write(
    binding: &Binding,
    program: &Path,
    text: &str,
    thread_ts: Option<&str>,
    key: &str,
) -> Result<String, NotMade> {
    let mut input = String::from("{");
    json::member(&mut input, "body");
    input.push('{');
    json::member(&mut input, "channel");
    json::push_text(&mut input, &binding.channel);
    json::member(&mut input, "text");
    json::push_text(&mut input, text);
    if let Some(thread_ts) = thread_ts {
        json::member(&mut input, "thread_ts");
        json::push_text(&mut input, thread_ts);
    }
    input.push_str("}}");
    let ran = invoke(program, &binding.connector, OPERATION, Some(key), &input)
        .map_err(NotMade::because)?;
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

/// Starts `connectors --output json invoke <target> <operation> --input-stdin`, with
/// `--idempotency-key <key>` for a post, with `input` on its standard input and waits for its
/// exit. First it takes the turn ([`TURN`]) and holds it until it returns, so one child runs at a
/// time: a post waits for a read of the channel that runs, and a read for a post. Its standard
/// output and standard error are captured
/// ([`output::captured`]) and read by threads of their own to [`MOST_ANSWER_BYTES`] each
/// ([`read_into`]), never shown as they are. The exit of the child ends the run: after it the
/// streams are read for at most [`DRAIN`], so a process that inherited the pipes holds nothing.
/// The child is the leader of a process group of its own; while it runs it is in
/// [`wake::CHILD`], where a SIGINT or SIGTERM of the bridge ends its group before the bridge ends.
///
/// # Errors
///
/// The child did not start, was not waited for, or was gone from its slot (a defensive check no
/// path reaches). A child the signal path took returns nothing: that path holds the slot's guard
/// until the process ends, and this thread blocks on it until then.
fn invoke(
    program: &Path,
    target: &str,
    operation: &str,
    key: Option<&str>,
    input: &str,
) -> Result<Ran, String> {
    let turn = TURN.lock().unwrap_or_else(PoisonError::into_inner);
    let mut command = Command::new(program);
    command.args([
        "--output",
        "json",
        "invoke",
        target,
        operation,
        "--input-stdin",
    ]);
    if let Some(key) = key {
        command.args(["--idempotency-key", key]);
    }
    command
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
                // Defensive: no path reaches this. Only this call fills and empties the slot
                // while it holds the turn, and the signal path that takes the child holds the
                // slot's guard until the process ends, so this thread then blocks on
                // `wake::child()` above and never finds the slot empty.
                return Err(format!(
                    "`{CONNECTORS}` was gone from the slot of the child it runs in, and is not \
                     waited for"
                ));
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
    let ran = Ran {
        status,
        stdout: took(&stdout),
        stderr: took(&stderr),
    };
    drop(turn);
    Ok(ran)
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
    match exit_code(ran)? {
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
        other => Err(other_exit(other, ran)),
    }
}

/// The exit status of `ran`.
///
/// # Errors
///
/// The child was ended by a signal, or ended without an exit status.
fn exit_code(ran: &Ran) -> Result<i32, NotMade> {
    ran.status.code().ok_or_else(|| {
        NotMade::because(match ran.status.signal() {
            Some(signal) => format!("`{CONNECTORS}` was ended by the signal {signal}"),
            None => format!("`{CONNECTORS}` ended without an exit status"),
        })
    })
}

/// A run that exited `code`, neither 0 nor 1: 2 is a usage error, any other an exit of its own;
/// with the first line of its standard error.
fn other_exit(code: i32, ran: &Ran) -> NotMade {
    let what = if code == 2 {
        format!("`{CONNECTORS}` exited with 2, a usage error")
    } else {
        format!("`{CONNECTORS}` exited with {code}")
    };
    NotMade::because(with_said(what, &ran.stderr))
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

/// The poll of the read mark: a thread of the bridge with a client of its own that reads the
/// recent posts of the channel and marks read each message whose `posted` chat post the allowed
/// chat user reacted to (`docs/design/bridge.md`, "The read mark").
struct Poll {
    client: Client,
    binding: Binding,
    program: PathBuf,
    interval: Duration,
    /// `--chat-post-retention`, in seconds.
    retention: i64,
    /// The clock of the bridge: when the `expires` of a message and the retention have passed.
    /// The requests are signed with the system clock all the same.
    clock: Arc<dyn Clock>,
    /// Whether the last line at the end of a message failed: a run of failures is named once.
    failing_end: bool,
}

impl Poll {
    /// Polls at once, then after the interval; a poll that failed is followed by the wait it
    /// named (`rate_limited`) and the pause of a failure ([`wake::pause`]) instead, and one line
    /// on standard error names the first failed poll of a run of them. It runs until the process
    /// ends.
    fn run(mut self) {
        let mut failures = 0_u32;
        loop {
            match self.once() {
                Ok(()) => {
                    failures = 0;
                    std::thread::sleep(self.interval);
                }
                Err(NotMade { why, wait }) => {
                    if failures == 0 {
                        output::warn(&format!(
                            "{NAME}: the read of the reactions in the chat channel {} failed: \
                             {why}; nothing is marked read, and it is tried again after a pause \
                             (no further line until a read succeeds)",
                            self.binding.channel
                        ));
                    }
                    if let Some(wait) = wait {
                        std::thread::sleep(wait);
                    }
                    std::thread::sleep(wake::pause(failures));
                    failures = failures.saturating_add(1);
                }
            }
        }
    }

    /// One poll. It reads the entries of the bound channel in the record: those not at their end
    /// (no `end`), those at their end whose retention the clock of the bridge has passed
    /// ([`Poll::past_retention`]), and those at their end that have no start of a retention yet
    /// (no `expires`, no `gone_at`: an entry of a build before this one). With none, nothing.
    /// Else the person's `Pending` ([`Poll::listed`]); then, for each entry `read` not at its end
    /// whose message `Pending` lists in `Delivered` (a `read` of a repost that was lost), the
    /// `read` request of `inbox read` again; then, with an entry `posted` whose message it lists,
    /// the reactions of the channel from the oldest of them on ([`Poll::reacted`]), and for each
    /// whose post the allowed chat user reacted to the `read` request, after which the entry is
    /// `read` or `due` in the record (the line of a due message once it is recorded `due`); then,
    /// for each entry not at its end whose message `Pending` does not list, the row of the message
    /// in `Thread` and what it shows ([`Poll::ended`]); then, for each entry at its end past the
    /// retention, the row in `Thread`, and with none the entry is dropped; then, for each entry at
    /// its end without a start of its retention, the row in `Thread`, and with none the entry
    /// records the reading of the clock as `gone_at`. An entry of another channel is not looked
    /// at; one that names no channel counts as of the bound channel.
    ///
    /// So a poll makes at most 1 read of `Pending`, 1 read of `Thread` per entry not at its end
    /// whose message left `Pending`, per entry at its end past the retention and per entry at its
    /// end without a start of its retention, 10 pages of the channel, 1 `read` per entry listed
    /// in `Pending` (a reacted post, or an entry `read` whose message is `Delivered`) and 1 line
    /// per message that reached its end.
    ///
    /// # Errors
    ///
    /// Why the poll failed: the record that cannot be read, the read of `Pending`, the read of
    /// the channel, a `read` request that got another answer than success, `already-read`,
    /// `unknown-message` or `deadline-passed`, a read of `Thread`, or a change of the record that
    /// failed. A line at the end of a message that failed fails no poll.
    fn once(&mut self) -> Result<(), NotMade> {
        let address = self.client.own_inbox().0;
        let address = &address;
        let channel = self.binding.channel.clone();
        let channel = &channel;
        let now = self.clock.now();
        let entries: Vec<ChatPost> = self
            .client
            .outbox
            .chat_posts(address)
            .map_err(|failure| {
                NotMade::because(format!(
                    "the record of the chat posts: {}",
                    failure.message()
                ))
            })?
            .into_iter()
            .filter(|post| post.channel.as_ref().is_none_or(|of| of == channel))
            .collect();
        let (ended, followed): (Vec<ChatPost>, Vec<ChatPost>) =
            entries.into_iter().partition(|post| post.end.is_some());
        let (rooted, unrooted): (Vec<ChatPost>, Vec<ChatPost>) = ended
            .into_iter()
            .partition(|post| retention_start(post).is_some());
        let ripe: Vec<ChatPost> = rooted
            .into_iter()
            .filter(|post| self.past_retention(post, now))
            .collect();
        if followed.is_empty() && ripe.is_empty() && unrooted.is_empty() {
            return Ok(());
        }
        // `Pending` is read after the record: an entry read above was posted, so its message was
        // in `Pending` before this read, and one that is not listed now has left it.
        let (listed, delivered) = self.listed()?;
        let (left, open): (Vec<ChatPost>, Vec<ChatPost>) = followed
            .into_iter()
            .partition(|post| !listed.contains(&post.message_id));
        for post in lost_reads(&open, &delivered) {
            self.read(post)?;
        }
        let open: Vec<ChatPost> = open
            .into_iter()
            .filter(|post| post.state == outbox::POSTED)
            .collect();
        if !open.is_empty() {
            self.marked(&open)?;
        }
        for post in &left {
            self.ended(post, now)?;
        }
        for post in &ripe {
            if self.row(post)?.is_none() {
                self.change(post, None)?;
            }
        }
        for post in &unrooted {
            if self.row(post)?.is_none() {
                self.change(
                    post,
                    Some(&ChatPost {
                        gone_at: Some(format_reading(now)),
                        ..post.clone()
                    }),
                )?;
            }
        }
        Ok(())
    }

    /// Whether the clock of the bridge at `now` has passed the start of the retention of `post`
    /// ([`retention_start`]) plus the retention; an entry without a start has not.
    fn past_retention(&self, post: &ChatPost, now: Reading) -> bool {
        retention_start(post).is_some_and(|start| now >= start.plus_seconds(self.retention))
    }

    /// The row of the message of `post` in the `Thread` view of its thread (the message id, in a
    /// line of a build before the thread was recorded), as the read rule gives it to the person:
    /// `None` when the service holds none.
    ///
    /// # Errors
    ///
    /// No answer, or one that is not the view.
    fn row(&self, post: &ChatPost) -> Result<Option<Value>, NotMade> {
        let thread = post.thread.as_deref().unwrap_or(&post.message_id);
        let answer = commands::thread_now(&self.client, thread).map_err(|failure| {
            NotMade::because(format!(
                "the read of the thread of {} failed: {}",
                post.message_id,
                failure.message()
            ))
        })?;
        let Some(Value::Array(rows)) = answer.member("rows") else {
            return Err(NotMade::because(format!(
                "the answer to the read of the thread of {} is not the view: {}",
                post.message_id,
                output::document(&answer)
            )));
        };
        Ok(rows
            .iter()
            .find(|row| output::text(row, "message_id") == post.message_id)
            .cloned())
    }

    /// Replaces the line of `post` in the record with `to`, or drops it.
    ///
    /// # Errors
    ///
    /// The record could not be changed.
    fn change(&self, post: &ChatPost, to: Option<&ChatPost>) -> Result<(), NotMade> {
        self.client
            .outbox
            .change_chat_post(&self.client.own_inbox().0, post, to)
            .map(drop)
            .map_err(|failure| {
                NotMade::because(format!(
                    "the record of the chat posts could not take the end of {}: {}",
                    post.message_id,
                    failure.message()
                ))
            })
    }

    /// An entry not at its end whose message left `Pending`: moved by what its row in `Thread`
    /// shows (`docs/design/bridge.md`, "The nouns of the bridge"): `Read`, the read mark `read`;
    /// answered or refused by the person, the end `answered`; answered on silence, the line of
    /// silence and the end `decided`; no row, the line of an expired message and the end
    /// `expired` once its `expires` has passed, the end `gone` before, and the entry dropped once
    /// the retention has passed too. An entry that names no `expires` (of a build before this
    /// one) gets no line and no end: the first poll that finds no row records the reading of the
    /// clock as `gone_at`, from which its retention counts, and the poll after the retention that
    /// finds no row drops it.
    ///
    /// # Errors
    ///
    /// The read of `Thread`, or a change of the record, failed.
    fn ended(&mut self, post: &ChatPost, now: Reading) -> Result<(), NotMade> {
        let with_end = |end: &str| ChatPost {
            end: Some(end.to_owned()),
            ..post.clone()
        };
        let Some(row) = self.row(post)? else {
            return match expires(post) {
                None if post.gone_at.is_none() => self.change(
                    post,
                    Some(&ChatPost {
                        gone_at: Some(format_reading(now)),
                        ..post.clone()
                    }),
                ),
                None if self.past_retention(post, now) => self.change(post, None),
                None => Ok(()),
                Some(_) if self.past_retention(post, now) => self.change(post, None),
                Some(expires) if now >= expires => {
                    self.line(post, output::CHAT_EXPIRED, &with_end(outbox::EXPIRED))
                }
                Some(_) => self.change(post, Some(&with_end(outbox::GONE))),
            };
        };
        match output::text(&row, "state") {
            "Read" if post.state != outbox::READ => self.change(
                post,
                Some(&ChatPost {
                    state: outbox::READ.to_owned(),
                    ..post.clone()
                }),
            ),
            "Answered" if output::text(&row, "answered_by") == "silence" => {
                let letter = output::text(&row, "decided_option");
                let text = row
                    .member("decision")
                    .and_then(|decision| decision.member("options"))
                    .and_then(|options| match options {
                        Value::Array(options) => options
                            .iter()
                            .find(|option| output::text(option, "letter") == letter),
                        _ => None,
                    })
                    .map(|option| output::text(option, "text"));
                let line = output::chat_silence(letter, text);
                self.line(post, &line, &with_end(outbox::DECIDED))
            }
            "Answered" | "Refused" => self.change(post, Some(&with_end(outbox::ANSWERED))),
            _ => Ok(()),
        }
    }

    /// Writes `text` into the thread of the chat post of `post` (a reply under the key
    /// [`end_key`], the same at every try), then changes the entry to `to`. A reply that fails
    /// is named in one line per run of failures and tried again at a later poll; it fails no
    /// poll.
    ///
    /// # Errors
    ///
    /// The change of the record failed after the reply: the next poll writes it again with the
    /// same key.
    fn line(&mut self, post: &ChatPost, text: &str, to: &ChatPost) -> Result<(), NotMade> {
        let key = end_key(&post.message_id);
        match write(
            &self.binding,
            &self.program,
            text,
            Some(&post.reference),
            &key,
        ) {
            Ok(_) => {
                self.failing_end = false;
                self.change(post, Some(to))
            }
            Err(NotMade { why, .. }) => {
                if !self.failing_end {
                    output::warn(&format!(
                        "{NAME}: the line at the end of {} in the thread of its post in the chat \
                         channel {} failed: {why}; it is tried again at a later poll (no further \
                         line until one succeeds)",
                        post.message_id, self.binding.channel
                    ));
                }
                self.failing_end = true;
                Ok(())
            }
        }
    }

    /// The read marks of one poll: the reactions of the channel from the oldest of the `posted`
    /// entries `open` on, and for each reacted post the `read` request and the entry's new state.
    ///
    /// # Errors
    ///
    /// As [`Poll::once`] says.
    fn marked(&self, open: &[ChatPost]) -> Result<(), NotMade> {
        let address = &self.client.own_inbox().0;
        let channel = &self.binding.channel;
        let reacted = self.reacted(oldest(open))?;
        for post in open.iter().filter(|post| reacted.contains(&post.reference)) {
            let state = self.read(post)?;
            self.client
                .outbox
                .mark_chat_post(address, post, state)
                .map_err(|failure| {
                    NotMade::because(format!(
                        "the service answered the read mark of {}, and the record of the chat \
                         posts could not say so (the mark is sent again): {}",
                        post.message_id,
                        failure.message()
                    ))
                })?;
            if state == outbox::DUE {
                output::warn(&format!(
                    "{NAME}: {} was not marked read on the reaction to its post in the chat \
                     channel {channel}: refused with Expired, its deadline has passed; the expiry \
                     task decides it, and the post is not asked about again",
                    post.message_id
                ));
            }
        }
        Ok(())
    }

    /// The message ids of the rows of the person's `Pending`, read whole and at once
    /// ([`commands::pending_now`]), and those of its rows in `Delivered`.
    ///
    /// # Errors
    ///
    /// No answer, or one that is not the view ([`listed_ids`]): the poll settles nothing then.
    fn listed(&self) -> Result<(HashSet<String>, HashSet<String>), NotMade> {
        let answer = commands::pending_now(&self.client).map_err(|failure| {
            NotMade::because(format!(
                "the read of `Pending` failed: {}",
                failure.message()
            ))
        })?;
        let listed = listed_ids(&answer).ok_or_else(|| {
            NotMade::because(format!(
                "the answer to the read of `Pending` is not the view: {}",
                output::document(&answer)
            ))
        })?;
        let delivered = output::rows(&answer)
            .iter()
            .filter(|row| output::text(row, "state") == "Delivered")
            .map(|row| output::text(row, "message_id").to_owned())
            .collect();
        Ok((listed, delivered))
    }

    /// The `read` request of the message of `post`, and the state its answer gives the entry:
    /// `read` for a success (`handled`, `read`, `already-read`) and for `unknown-message`; `due`
    /// for `deadline-passed`, which [`Poll::once`] names in one line once it is recorded.
    ///
    /// # Errors
    ///
    /// Every other answer, and no answer.
    fn read(&self, post: &ChatPost) -> Result<&'static str, NotMade> {
        let refused = match commands::read(&self.client, &post.message_id) {
            Ok(_) => return Ok(outbox::READ),
            Err(Failure::Refused { answer, .. }) if declared_error(&answer).is_some() => answer,
            Err(failure) => {
                return Err(NotMade::because(format!(
                    "the read mark of {} was not taken: {}",
                    post.message_id,
                    failure.message()
                )));
            }
        };
        match output::text(&refused, "outcome") {
            "unknown-message" => Ok(outbox::READ),
            "deadline-passed" => Ok(outbox::DUE),
            other => Err(NotMade::because(format!(
                "the read mark of {} was refused: {} ({other})",
                post.message_id,
                declared_error(&refused).unwrap_or_default()
            ))),
        }
    }

    /// The references of the posts of the channel from `oldest` on that the allowed chat user
    /// reacted to: one read per page ([`read_input`], [`judged_read`]), at most [`MOST_PAGES`].
    ///
    /// # Errors
    ///
    /// A page that failed, or a further page after the last one read.
    fn reacted(&self, oldest: Option<&str>) -> Result<HashSet<String>, NotMade> {
        let mut reacted = HashSet::new();
        let mut cursor: Option<String> = None;
        for _ in 0..MOST_PAGES {
            let input = read_input(&self.binding.channel, oldest, cursor.as_deref());
            let ran = invoke(
                &self.program,
                &self.binding.connector,
                READ_OPERATION,
                None,
                &input,
            )
            .map_err(NotMade::because)?;
            let page = judged_read(&ran, &self.binding.allowed_chat_user)?;
            reacted.extend(page.reacted);
            match page.next {
                Some(next) => cursor = Some(next),
                None => return Ok(reacted),
            }
        }
        Err(NotMade::because(format!(
            "the channel has more than {MOST_PAGES} pages of {PAGE_LIMIT} posts from the oldest \
             open post on"
        )))
    }
}

/// The `expires` the entry `post` names, when it names one that reads as an instant.
fn expires(post: &ChatPost) -> Option<Reading> {
    post.expires
        .as_deref()
        .and_then(|text| Reading::parse(text).ok())
}

/// Where the retention of the entry `post` starts: its `expires`, else the reading at which the
/// bridge first found its message gone (`gone_at`, an entry of a build before `expires` was
/// recorded); `None` when it names neither as an instant.
fn retention_start(post: &ChatPost) -> Option<Reading> {
    expires(post).or_else(|| {
        post.gone_at
            .as_deref()
            .and_then(|text| Reading::parse(text).ok())
    })
}

/// The entries of `open` whose `read` of a repost was lost: the entry is `read` and its own
/// message is `Delivered` in `Pending` (`delivered`). An entry `read` whose message is in `Sent`
/// (a repost the round has not marked yet) is not one, also when another message is
/// `Delivered`.
fn lost_reads<'a>(open: &'a [ChatPost], delivered: &HashSet<String>) -> Vec<&'a ChatPost> {
    open.iter()
        .filter(|post| post.state == outbox::READ && delivered.contains(&post.message_id))
        .collect()
}

/// The least reference of `open` in the order of the provider's `ts` ([`ts_order`]): the
/// `oldest` of a read. A reference of another form is never sent.
fn oldest(open: &[ChatPost]) -> Option<&str> {
    open.iter()
        .filter_map(|post| ts_order(&post.reference).map(|order| (order, post.reference.as_str())))
        .min()
        .map(|(_, reference)| reference)
}

/// The order of a `ts` of the provider, `<seconds>.<fraction>` in digits: the seconds, then the
/// fraction to 9 digits. `None` for another form.
fn ts_order(reference: &str) -> Option<(u64, u64)> {
    let (seconds, fraction) = reference.split_once('.')?;
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    if !digits(seconds) || !digits(fraction) || fraction.len() > 9 {
        return None;
    }
    Some((
        seconds.parse().ok()?,
        format!("{fraction:0<9}").parse().ok()?,
    ))
}

/// The input document of one page of a read of `channel`: the provider's parameters as members,
/// `oldest` as a string (a JSON number would lose its trailing zeros) with `inclusive` true (the
/// provider lists the posts after `oldest`, and the post at `oldest`, the oldest open post, only
/// with it), and `cursor` after the first page.
fn read_input(channel: &str, oldest: Option<&str>, cursor: Option<&str>) -> String {
    let mut input = String::from("{");
    json::member(&mut input, "channel");
    json::push_text(&mut input, channel);
    if let Some(oldest) = oldest {
        json::member(&mut input, "oldest");
        json::push_text(&mut input, oldest);
        json::member(&mut input, "inclusive");
        input.push_str("true");
    }
    json::member(&mut input, "limit");
    input.push_str(&PAGE_LIMIT.to_string());
    if let Some(cursor) = cursor {
        json::member(&mut input, "cursor");
        json::push_text(&mut input, cursor);
    }
    input.push('}');
    input
}

/// One page of a read of the channel: the references of its posts that the allowed chat user
/// reacted to, and the cursor of the next page, if there is one.
#[derive(Debug, Default, PartialEq, Eq)]
struct Page {
    reacted: Vec<String>,
    next: Option<String>,
}

/// What one run of `connectors` that read a page came to, read by its exit status as a post's
/// ([`judged`]): a page only when it exited 0 and its standard output is one ([`page`]); on exit 1
/// the refusal on standard error ([`read_refused`]).
///
/// # Errors
///
/// Why the read gave no page.
fn judged_read(ran: &Ran, user: &str) -> Result<Page, NotMade> {
    match exit_code(ran)? {
        0 => {
            let answer = document(&ran.stdout).map_err(|what| {
                NotMade::because(format!(
                    "`{CONNECTORS}` exited with 0 and its standard output is {what}"
                ))
            })?;
            page(&answer, user).map_err(|lacks| {
                NotMade::because(format!(
                    "`{CONNECTORS}` exited with 0 and its answer is not a page of the channel: \
                     {lacks}"
                ))
            })
        }
        1 => Err(match document(&ran.stderr) {
            Ok(failure) => read_refused(&failure),
            Err(what) => NotMade::because(format!(
                "`{CONNECTORS}` exited with 1 and its standard error is {what}"
            )),
        }),
        other => Err(other_exit(other, ran)),
    }
}

/// The page an answer of `connectors --output json invoke <target> conversations.history`
/// holds: `result.result.status` 200, `result.result.body.ok` true, `result.result.body.messages`
/// a list of posts, each an object with a `ts` text that is not empty and its `reactions` absent
/// or a list; the next page's cursor in `response_metadata.next_cursor` (none when it, or its
/// object, is absent, or it is empty). A reaction counts when its `users` is a list that holds
/// `user` as a text; any other reaction counts as none of `user`.
///
/// # Errors
///
/// Which member is missing or of another type: the whole page is refused.
fn page(answer: &Value, user: &str) -> Result<Page, String> {
    let provider = answer
        .member("result")
        .and_then(|result| result.member("result"));
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
    let Some(Value::Array(messages)) = body.and_then(|body| body.member("messages")) else {
        return Err("the provider's answer holds no list of posts (`messages`)".to_owned());
    };
    let mut reacted = Vec::new();
    for message in messages {
        let ts = match message.member("ts") {
            Some(Value::Text(ts)) if !ts.is_empty() => ts,
            _ => return Err("a post of the answer names no reference (`ts`)".to_owned()),
        };
        match message.member("reactions") {
            None => {}
            Some(Value::Array(reactions)) => {
                if reactions.iter().any(|reaction| reacted_by(reaction, user)) {
                    reacted.push(ts.clone());
                }
            }
            Some(_) => {
                return Err(format!(
                    "the reactions of the post {ts} are not a list (`reactions`)"
                ));
            }
        }
    }
    let next = match body.and_then(|body| body.member("response_metadata")) {
        None => None,
        Some(metadata @ Value::Object(_)) => match metadata.member("next_cursor") {
            None => None,
            Some(Value::Text(cursor)) if cursor.is_empty() => None,
            Some(Value::Text(cursor)) => Some(cursor.clone()),
            Some(_) => {
                return Err(
                    "the cursor of the next page (`response_metadata.next_cursor`) is not a text"
                        .to_owned(),
                );
            }
        },
        Some(_) => return Err("`response_metadata` is not an object".to_owned()),
    };
    Ok(Page { reacted, next })
}

/// Whether `reaction` is one of `user`: its `users` is a list that holds `user` as a text.
/// The message ids of the rows of a `Pending` answer: `None` unless `rows` is a list whose every
/// row has a text `message_id`, so an answer of another form settles no post.
fn listed_ids(answer: &Value) -> Option<HashSet<String>> {
    let Some(Value::Array(rows)) = answer.member("rows") else {
        return None;
    };
    rows.iter()
        .map(|row| match row.member("message_id") {
            Some(Value::Text(id)) => Some(id.clone()),
            _ => None,
        })
        .collect()
}

fn reacted_by(reaction: &Value, user: &str) -> bool {
    matches!(reaction.member("users"),
        Some(Value::Array(users)) if users.iter().any(|id| matches!(id, Value::Text(id) if id == user)))
}

/// The read not made that the refusal of `connectors` (exit 1, on standard error) names: `ok`
/// false and an object `error`; the code in `error.data.code` (`service_failure` when the
/// provider refused) and the class in `error.data.service_code`. The line names both and no
/// reason of the provider; for the class (or code) `rate_limited` the wait is its
/// `error.data.retry_after_seconds` ([`retry_after`]).
fn read_refused(failure: &Value) -> NotMade {
    let error = failure.member("error");
    if !matches!(failure.member("ok"), Some(Value::Bool(false)))
        || !matches!(error, Some(Value::Object(_)))
    {
        return NotMade::because(format!(
            "`{CONNECTORS}` exited with 1 and its standard error is not a failure document (`ok` \
             false, `error`)"
        ));
    }
    let data = error.and_then(|error| error.member("data"));
    let code = text_of(data.and_then(|data| data.member("code")));
    let class = text_of(data.and_then(|data| data.member("service_code")));
    let mut why = format!("`{CONNECTORS}` exited with 1: the read was refused");
    match &code {
        Some(code) => why.push_str(&format!(" with the code `{code}`")),
        None => why.push_str(" with no code"),
    }
    if let Some(class) = &class {
        why.push_str(&format!(" of the class `{class}`"));
    }
    let mut wait = None;
    if class.as_deref() == Some("rate_limited") || code.as_deref() == Some("rate_limited") {
        wait = data.and_then(retry_after);
        match wait {
            Some(wait) => why.push_str(&format!(
                "; the next read is not before {} s",
                wait.as_secs()
            )),
            None => {
                why.push_str("; it names no wait of 1 to 3600 s, the pause of a failure applies");
            }
        }
    }
    NotMade { why, wait }
}

#[cfg(test)]
mod tests {
    use inbox::server::json;

    use std::collections::HashSet;
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;
    use std::time::Duration;

    use std::sync::Mutex;

    use super::{
        Binding, Given, MOST_ANSWER_BYTES, NotMade, Page, Ran, Taken, applied, judged, judged_read,
        listed_ids, lost_reads, oldest, page, read_input, read_into,
    };
    use crate::outbox::ChatPost;

    const USER: &str = "U0000000001";

    /// The poll's `read` again goes to an entry `read` whose own message is `Delivered`, and to
    /// no other: not to an entry `read` whose message is in `Sent` while another message is
    /// `Delivered` (a repost the round has not marked yet: its `MarkDelivered` comes first), not
    /// to an entry `posted` whose message is `Delivered`.
    #[test]
    fn a_read_is_sent_again_only_for_an_entry_read_whose_own_message_is_delivered() {
        let entry = |message_id: &str, state: &str| ChatPost {
            message_id: message_id.to_owned(),
            channel: Some("C0000000001".to_owned()),
            reference: format!("1700000000.00000{}", message_id.len()),
            state: state.to_owned(),
            thread: None,
            expires: None,
            end: None,
            gone_at: None,
        };
        let open = [
            entry("lost", crate::outbox::READ),
            entry("waiting", crate::outbox::READ),
            entry("unread", crate::outbox::POSTED),
        ];
        let delivered: HashSet<String> = ["lost", "unread"].map(str::to_owned).into();
        let again: Vec<&str> = lost_reads(&open, &delivered)
            .into_iter()
            .map(|post| post.message_id.as_str())
            .collect();
        assert_eq!(again, ["lost"]);
        let only_other: HashSet<String> = ["unread"].map(str::to_owned).into();
        assert!(lost_reads(&open, &only_other).is_empty());
        assert!(lost_reads(&open, &HashSet::new()).is_empty());
    }

    /// A page of a read with the body `body` (the members inside `result.result.body`).
    fn read_answer(body: &str) -> String {
        format!(r#"{{"result":{{"result":{{"status":200,"body":{{{body}}}}}}}}}"#)
    }

    fn page_of(body: &str) -> Result<Page, String> {
        page(&json::parse(&read_answer(body)).unwrap(), USER)
    }

    /// A page lists the posts the allowed chat user reacted to by their `ts`, and the cursor of
    /// the next page; a reaction of another user, or one that names no list of user ids as
    /// texts, is none of the allowed chat user's.
    #[test]
    fn a_page_lists_the_posts_the_allowed_chat_user_reacted_to_and_its_cursor() {
        let read = page_of(
            r#""ok":true,"messages":[
                {"ts":"3.000003","reactions":[{"name":"eyes","users":["U0000000002","U0000000001"],"count":2}]},
                {"ts":"2.000002","reactions":[{"name":"eyes","users":["U0000000002"]},{"name":"x","users":"U0000000001"},{"name":"y","users":[1]},{"name":"z"},"U0000000001",{"name":"w","users":["U00000000011"]}]},
                {"ts":"1.000001"}
            ],"has_more":true,"response_metadata":{"next_cursor":"bmV4dA=="}"#,
        )
        .unwrap();
        assert_eq!(
            read,
            Page {
                reacted: vec!["3.000003".to_owned()],
                next: Some("bmV4dA==".to_owned()),
            }
        );
        for last in [
            r#""ok":true,"messages":[]"#,
            r#""ok":true,"messages":[],"has_more":true"#,
            r#""ok":true,"messages":[],"response_metadata":{}"#,
            r#""ok":true,"messages":[],"response_metadata":{"next_cursor":""}"#,
        ] {
            assert_eq!(page_of(last).unwrap().next, None, "{last}");
        }
    }

    /// A member missing or of another type refuses the whole page, also when a post before it
    /// carries a reaction of the allowed chat user.
    #[test]
    fn a_member_missing_or_mistyped_refuses_the_whole_page() {
        let reacted = r#"{"ts":"1.000001","reactions":[{"users":["U0000000001"]}]}"#;
        for body in [
            r#""ok":false,"messages":[]"#.to_owned(),
            r#""ok":"true","messages":[]"#.to_owned(),
            r#""ok":true"#.to_owned(),
            r#""ok":true,"messages":{}"#.to_owned(),
            format!(r#""ok":true,"messages":[{reacted},{{"ts":5}}]"#),
            format!(r#""ok":true,"messages":[{reacted},{{"ts":""}}]"#),
            format!(r#""ok":true,"messages":[{reacted},"1.000002"]"#),
            format!(r#""ok":true,"messages":[{reacted},{{"ts":"2.0","reactions":{{}}}}]"#),
            format!(r#""ok":true,"messages":[{reacted}],"response_metadata":[]"#),
            format!(r#""ok":true,"messages":[{reacted}],"response_metadata":{{"next_cursor":7}}"#),
        ] {
            assert!(page_of(&body).is_err(), "{body}");
        }
        for answer in [
            r#"{}"#,
            r#"{"result":{"result":{"status":"200","body":{"ok":true,"messages":[]}}}}"#,
            r#"{"result":{"result":{"status":429,"body":{"ok":true,"messages":[]}}}}"#,
        ] {
            assert!(
                page(&json::parse(answer).unwrap(), USER).is_err(),
                "{answer}"
            );
        }
    }

    /// A refused read names its code and its class and no reason of the provider; `rate_limited`
    /// waits its bounded `retry_after_seconds`; a usage error, a signal and a standard output over
    /// the bound are failed reads.
    #[test]
    fn a_refused_read_names_its_class_and_a_rate_limit_its_wait() {
        let refused = |class: &str, extra: &str| {
            format!(
                r#"{{"ok":false,"error":{{"code":"failure","message":"the provider's \u001b reason","data":{{"code":"service_failure","stage":"dispatch","service_code":"{class}"{extra}}}}}}}"#
            )
        };
        let read = |raw: i32, stdout: &str, stderr: &str| {
            judged_read(&ran(raw, stdout, stderr), USER).unwrap_err()
        };
        let forbidden = read(1 << 8, "", &refused("forbidden", ""));
        assert_eq!(
            forbidden.why,
            "`connectors` exited with 1: the read was refused with the code `service_failure` of \
             the class `forbidden`"
        );
        assert_eq!(forbidden.wait, None);
        let limited = read(
            1 << 8,
            "",
            &refused("rate_limited", r#","retry_after_seconds":7"#),
        );
        assert_eq!(limited.wait, Some(Duration::from_secs(7)));
        assert!(limited.why.contains("`rate_limited`"), "{limited:?}");
        let unbounded = read(
            1 << 8,
            "",
            &refused("rate_limited", r#","retry_after_seconds":3601"#),
        );
        assert_eq!(unbounded.wait, None);
        assert!(
            read(1 << 8, "", r#"{"ok":true,"error":{}}"#)
                .why
                .contains("not a failure document")
        );
        assert!(
            read(1 << 8, "", "connectors: not configured\n")
                .why
                .contains("exited with 1 and its standard error is not one JSON document")
        );
        assert_eq!(
            read(2 << 8, "", "error: unexpected argument\n").why,
            "`connectors` exited with 2, a usage error: error: unexpected argument"
        );
        assert_eq!(
            read(9, "", "").why,
            "`connectors` was ended by the signal 9"
        );
        let whole = read_answer(r#""ok":true,"messages":[]"#);
        assert_eq!(judged_read(&ran(0, &whole, ""), USER), Ok(Page::default()));
        let long = format!("{whole}{}", " ".repeat(MOST_ANSWER_BYTES));
        let over = Ran {
            status: ExitStatus::from_raw(0),
            stdout: taken(long.as_bytes()),
            stderr: Taken::default(),
        };
        assert!(
            judged_read(&over, USER)
                .unwrap_err()
                .why
                .contains(&format!("longer than {MOST_ANSWER_BYTES} bytes"))
        );
    }

    /// The ids of a `Pending` answer are those of its rows; an answer without a list of rows, or
    /// with a row without a text `message_id`, gives none, so no post is settled by it.
    #[test]
    fn only_a_view_with_an_id_in_every_row_lists_the_messages_of_pending() {
        let ids = |text: &str| listed_ids(&json::parse(text).unwrap());
        assert_eq!(
            ids(r#"{"rows":[{"message_id":"A","state":"Sent"},{"message_id":"B"}]}"#),
            Some(HashSet::from(["A".to_owned(), "B".to_owned()]))
        );
        assert_eq!(ids(r#"{"rows":[]}"#), Some(HashSet::new()));
        assert_eq!(
            ids(r#"{"rows":[{"message_id":"A"},{"message_id":7}]}"#),
            None
        );
        assert_eq!(ids(r#"{"rows":[{"message_id":"A"},{}]}"#), None);
        assert_eq!(ids(r#"{"rows":{}}"#), None);
        assert_eq!(ids(r#"{"refused":"inbox not open"}"#), None);
    }

    /// A read asks from the oldest open post on, that post included, in the order of the
    /// provider's `ts`, as a string; a reference of another form is never sent; the cursor follows
    /// the first page.
    #[test]
    fn a_read_asks_from_the_oldest_open_post_on() {
        let post = |reference: &str| ChatPost {
            message_id: "M".to_owned(),
            channel: Some("C0000000001".to_owned()),
            reference: reference.to_owned(),
            state: "posted".to_owned(),
            thread: None,
            expires: None,
            end: None,
            gone_at: None,
        };
        let open = [
            post("1700000010.000001"),
            post("not-a-ts"),
            post("1700000009.9"),
            post("1700000009.100000"),
            post("999999999.000000"),
        ];
        assert_eq!(oldest(&open), Some("999999999.000000"));
        assert_eq!(oldest(&open[..4]), Some("1700000009.100000"));
        assert_eq!(oldest(&[post("x.1"), post("1.")]), None);
        assert_eq!(
            read_input("C0000000001", Some("1700000000.000100"), None),
            r#"{"channel":"C0000000001","oldest":"1700000000.000100","inclusive":true,"limit":100}"#
        );
        assert_eq!(
            read_input("C0000000001", None, Some("bmV4dA==")),
            r#"{"channel":"C0000000001","limit":100,"cursor":"bmV4dA=="}"#
        );
    }

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
                chat_poll_interval: 10,
                chat_post_retention: 86_400,
                clock_file: None,
                authority: None,
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
