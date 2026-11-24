//! `inbox wake`: the command every wake adapter runs (`docs/design/wake.md`). It holds the
//! `Pending` view of the caller's own inbox open until it has a row in `Sent` that is not due,
//! then writes those rows to standard output, flushes them, sends `MarkDelivered` for each, and
//! exits. A row in `Delivered` (delivered before, not yet read) or past its `expires` neither ends
//! a wake nor is printed by it: the service leaves them out of the answer to `undelivered=1`.
//!
//! - A standard output that is closed or `/dev/null`, or a pipe or a FIFO without a reader, ends
//!   the wake with 4 first of all, before the lock and any request (`output::reaches_a_reader`):
//!   no one would take its rows. The rows are written through a duplicate of descriptor 1
//!   (`output::try_print_text`), so a standard output that cannot be written (the reader gone,
//!   open only for reading) ends it with 4 and no mark.
//! - The reader of a pipe or a FIFO is watched while the wake waits ([`Watch`]): a thread looks
//!   every [`WATCH_PERIOD`], and when the reading end is closed it ends the wake with 4, no mark,
//!   without a post having to arrive; the end of the process frees the lock. The same check runs
//!   after the rows of a round are written and before their first mark. A terminal, a regular
//!   file and a socket are not watched.
//! - One wake per inbox and outbox file on a host: before its first request it takes an
//!   exclusive `flock` of a file beside the outbox file, `<outbox>.wake.<16 hex digits>` (the
//!   64-bit FNV-1a hash of the address of the inbox it acts for, `--for` or the caller's own), and holds it until the process ends, a kill
//!   included: the kernel releases it. A second wake that finds it held exits 5 and sends
//!   nothing. The file is opened without blocking and without following a link, and must be a
//!   regular file; a file left behind by a wake that ended holds no lock and blocks nothing.
//! - Each round is a read without a wait, which names the epoch of the start that answers it, then
//!   a read held open by the service for at most [`WAIT_SECONDS`]; both ask with `undelivered=1`
//!   ([`commands::undelivered`]). After every answer the reconnect of every client command runs
//!   ([`commands::reconnect`]). The service answers a read with `undelivered=1` at once with
//!   `409 {"refused":"inbox not open"}` while the caller's inbox is not open, also at a new start
//!   and on a close while the read is held: the reconnect then opens the inbox again when this
//!   host recorded an `inbox open` of it at another start, and the wake reads again at once; it
//!   also reads again at once when the record is of the start that answered, and after the pause
//!   of a failure when the open got no answer or one tried again. It ends with exit 1 and one line
//!   that names `inbox open` ([`Failure::NotOpen`]) when this host has no record, when the service
//!   refused the open, and when the inbox is still not open right after an open that succeeded.
//! - A held read answered in less than [`QUICK`] without a row is followed by the pause of a
//!   failure, a run of them by the growing pause ([`pause`]), so a service that ends the wait at
//!   once does not make the wake spin.
//! - No answer, an answer cut before its end, and an answer `408` or from `500` on that is no
//!   refusal of the model are tried again after a pause that starts at [`FIRST_PAUSE`] and
//!   doubles up to [`MOST_PAUSE`] ([`pause`]); one line on standard error says so at the first
//!   failure of a run of them. Every other failure ends the wake with its exit status: wrong
//!   usage (2), a certificate that is not the pinned one, a peer without TLS, or an answer that
//!   is neither a success nor a refusal (a `3xx`, a `4xx` without `refused`, a `2xx` that is not
//!   the view) (3), a refusal (1), a local file (4).
//! - SIGINT and SIGTERM end the wake at once, as their default action does, also when the wake
//!   was started with them ignored; nothing is written to standard output.
//! - `inbox wake --follow` is the adapter `terminal` (`docs/design/wake.md`, `## terminal`): the
//!   same rounds, the same lock and the same failures, but after the marks of a round it waits
//!   again instead of exiting, so only a signal or a failure ends it. A mark that fails is named
//!   on standard error, and the next round starts after the pause of a failure ([`pause`]); so
//!   does a round that got a row the round before marked delivered. The rows of a round are
//!   followed by one BEL when standard output is a terminal ([`output::try_print_round`]). When
//!   this host's record of an `inbox open` of the caller names another adapter it ends with 1
//!   ([`Failure::OtherAdapter`]): at the start, before the lock and before any request, and in
//!   every round, before the rows of the round are written.
//! - `inbox bridge` (`src/bridge.rs`) runs the same rounds ([`rounds`]) with the chat channel as
//!   its [`Surface`]: the same reads, reconnect, tries, pauses, signals and lock, so a wake and a
//!   bridge of one inbox and outbox file on a host exclude each other (exit 5). The watch of the
//!   reader belongs to the surface that writes to standard output ([`Surface::watched`]): the
//!   bridge writes no row there and has none.

use std::fs::{DirBuilder, File, OpenOptions, TryLockError};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::Child;
#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::SeqCst};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use inbox::server::json::Value;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

use crate::client::declared_error;
use crate::commands::{self, Client, Reopen, Typed};
use crate::failure::Failure;
use crate::output;

/// The longest wait one read asks of the service, in seconds; the service cuts it to its own
/// `--max-wait`.
const WAIT_SECONDS: u64 = 60;
/// The pause after the first failure of a run of them.
const FIRST_PAUSE: Duration = Duration::from_millis(100);
/// The longest pause between two tries.
const MOST_PAUSE: Duration = Duration::from_secs(30);
/// A held read answered sooner than this without a row in `Sent` is followed by a pause, as a
/// failure is: the wait ended without holding (a service that does not know `undelivered=1`
/// answers its rows in `Delivered` at once), and the wake must not spin on it.
const QUICK: Duration = Duration::from_secs(1);

/// The pause after `failures` failures in a row before this one: [`FIRST_PAUSE`], doubled per
/// failure, at most [`MOST_PAUSE`].
pub fn pause(failures: u32) -> Duration {
    FIRST_PAUSE
        .checked_mul(2_u32.saturating_pow(failures))
        .map_or(MOST_PAUSE, |pause| pause.min(MOST_PAUSE))
}

/// Whether a failure of a read of `Pending` is tried again: no answer or one cut before its end,
/// or an answer `408` or from `500` on that is no refusal of the model (the service busy or
/// stopping, or a proxy in front of it). A `3xx`, a `4xx` without `refused`, and a `2xx` that is
/// not the view end the wake with 3.
fn tried_again(failure: &Failure) -> bool {
    match failure {
        Failure::Transport(_) => true,
        Failure::Unexpected { status, .. } => passing(*status),
        Failure::Refused { status, answer, .. } => {
            passing(*status) && declared_error(answer).is_none()
        }
        _ => false,
    }
}

/// A status that says the failure may pass: `408`, or from `500` on.
fn passing(status: u16) -> bool {
    status == 408 || status >= 500
}

/// Whether a failure is the service's answer to a read with `undelivered=1` that the caller's
/// inbox is not open there (`409 {"refused":"inbox not open"}`).
fn not_open(failure: &Failure) -> bool {
    matches!(failure, Failure::Refused { name, status: 409, answer }
        if name == "inbox not open" && declared_error(answer).is_none())
}

/// The adapter `inbox wake --follow` is.
const TERMINAL: &str = "terminal";

/// How often the watch of the reader looks whether a pipe or a FIFO on standard output still
/// has a reader: on an idle host a wake whose reader is gone ends within about this time.
const WATCH_PERIOD: Duration = Duration::from_millis(500);

/// The name of the thread that watches the reader.
const WATCH_THREAD: &str = "inbox-reader";

/// The watch of the reader of standard output (`docs/design/wake.md`, "A reader that is gone").
/// Only a pipe or a FIFO is watched: of these the kernel tells whether a reader holds the other
/// end ([`output::reader_gone`]). On a terminal, a regular file or a socket no thread runs, and
/// the wake is as it was before the watch.
struct Watch {
    /// A duplicate of descriptor 1 when it is a pipe or a FIFO, else `None`.
    pipe: Option<File>,
    /// Held by a round from the write of its rows to its last mark, and by the end of the wake,
    /// which sets it: the watch ends the process only while it holds it unset, so it cuts no
    /// round between its rows and its marks, and writes no second line after the wake ended in
    /// another way.
    ended: Mutex<bool>,
    /// The looks of the watch begun and done, a debug build's only: the point before the exit
    /// waits on them ([`Watch::before_exit`]).
    #[cfg(debug_assertions)]
    looks: Looks,
}

/// How many looks the watch has begun and how many it has done (a look is done when it found
/// the reader there or the wake ended), and whether its thread runs: set where the thread is
/// started, cleared by the watch before it returns; a debug build's only.
#[cfg(debug_assertions)]
#[derive(Default)]
struct Looks {
    begun: AtomicU64,
    done: AtomicU64,
    running: AtomicBool,
}

impl Watch {
    fn new() -> Self {
        Self {
            pipe: output::pipe(),
            ended: Mutex::new(false),
            #[cfg(debug_assertions)]
            looks: Looks::default(),
        }
    }

    /// The point before the exit of `inbox wake --before-exit <path>`, a debug build's only, for
    /// a case: once the wake has ended (its line written, its status known, the hold set), it
    /// creates the file `path`, waits until the file is gone again, and then, while the watch
    /// runs, until the watch has done a look it began after that. A case closes the reader
    /// before it removes the file, so a watch that runs has looked at a pipe without a reader
    /// before the process exits. A watch that has returned (it found the reader gone and the
    /// hold set) or was never started (the wake ended before it: no reader at the start, an
    /// inbox opened with another adapter) does not look again, and the point returns.
    #[cfg(debug_assertions)]
    fn before_exit(&self, path: Option<&Path>) {
        let Some(path) = path else {
            return;
        };
        if File::create_new(path).is_err() {
            return;
        }
        while path.symlink_metadata().is_ok() {
            std::thread::sleep(Duration::from_millis(10));
        }
        let begun = self.looks.begun.load(SeqCst);
        while self.looks.running.load(SeqCst) && self.looks.done.load(SeqCst) <= begun {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn hold(&self) -> MutexGuard<'_, bool> {
        self.ended.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Starts the thread that looks every [`WATCH_PERIOD`] whether the reader is gone and then
    /// ends the process as [`report`] ends it with [`no_reader`]: exit 4, no mark. Starts nothing
    /// when standard output is not a pipe or a FIFO.
    fn start(self: &Arc<Self>, json: bool) -> Result<(), Failure> {
        if self.pipe.is_none() {
            return Ok(());
        }
        let watch = Arc::clone(self);
        #[cfg(debug_assertions)]
        self.looks.running.store(true, SeqCst);
        let started = std::thread::Builder::new()
            .name(WATCH_THREAD.to_owned())
            .spawn(move || watch.watch(json));
        #[cfg(debug_assertions)]
        if started.is_err() {
            self.looks.running.store(false, SeqCst);
        }
        started
            .map(drop)
            .map_err(|error| Failure::Local(format!("the watch of the reader: {error}")))
    }

    fn watch(&self, json: bool) {
        let Some(pipe) = &self.pipe else {
            #[cfg(debug_assertions)]
            self.looks.running.store(false, SeqCst);
            return;
        };
        loop {
            std::thread::sleep(WATCH_PERIOD);
            #[cfg(debug_assertions)]
            self.looks.begun.fetch_add(1, SeqCst);
            if output::reader_gone(pipe) {
                let ended = self.hold();
                if *ended {
                    #[cfg(debug_assertions)]
                    {
                        self.looks.done.fetch_add(1, SeqCst);
                        self.looks.running.store(false, SeqCst);
                    }
                    return;
                }
                std::process::exit(report(&no_reader(), json));
            }
            #[cfg(debug_assertions)]
            self.looks.done.fetch_add(1, SeqCst);
        }
    }
}

/// The failure of a wake whose pipe or FIFO has no reader left.
fn no_reader() -> Failure {
    Failure::Local(format!(
        "standard output: {}: no message is marked delivered",
        output::NO_READER
    ))
}

/// Where the rounds ([`rounds`]) hand their rows: standard output for `inbox wake` and `inbox
/// wake --follow`, the chat channel of a binding for `inbox bridge` (`src/bridge.rs`).
pub trait Surface {
    /// The name each of its lines on standard error starts with: `inbox wake`, `inbox bridge`.
    fn name(&self) -> &'static str;

    /// Whether it runs on after the marks of a round (`--follow`, the bridge) instead of exiting.
    fn follows(&self) -> bool;

    /// Whether this host's record of an `inbox open` of the caller lets it serve the inbox
    /// ([`only`]): asked at the start, before the lock and any request, and, when it follows, in
    /// every round before the rows of the round are handed over.
    ///
    /// # Errors
    ///
    /// The failure that ends it.
    fn admits(&self, client: &Client) -> Result<(), Failure>;

    /// Runs after the signals are taken and before the lock: the surface that writes to standard
    /// output starts the watch of its reader here ([`Watch`]); the chat channel has none.
    ///
    /// # Errors
    ///
    /// The failure that ends it.
    fn watched(&self) -> Result<(), Failure> {
        Ok(())
    }

    /// Runs once the lock is held, before the first read. A failure that may pass
    /// ([`tried_again`]) is tried again after the pause of a failure, with the line of the wake
    /// that says so; every other failure ends it.
    ///
    /// # Errors
    ///
    /// The failure that ends it.
    fn started(&mut self, _client: &Client) -> Result<(), Failure> {
        Ok(())
    }

    /// Hands the rows of `answer` over and marks each one it handed over delivered. Answers the
    /// first failure of a row, after which the next round of a surface that follows starts after
    /// the pause of a failure, and the message ids whose mark succeeded.
    ///
    /// # Errors
    ///
    /// The failure that ends it.
    fn deliver(
        &mut self,
        client: &Client,
        answer: &Value,
        reconnected: Option<&str>,
    ) -> Result<(Option<Failure>, Vec<String>), Failure>;
}

/// Standard output, the surface of `inbox wake` and, with `follow`, of `inbox wake --follow`,
/// with the watch of its reader: started before the lock ([`Surface::watched`]) and held by each
/// round from the write of its rows to its last mark ([`deliver`]).
struct Output {
    json: bool,
    follow: bool,
    watch: Arc<Watch>,
}

impl Surface for Output {
    fn name(&self) -> &'static str {
        "inbox wake"
    }

    fn follows(&self) -> bool {
        self.follow
    }

    fn admits(&self, client: &Client) -> Result<(), Failure> {
        if self.follow {
            only(
                client,
                TERMINAL,
                "inbox wake --follow",
                "to show its messages in this terminal",
            )
        } else {
            Ok(())
        }
    }

    fn watched(&self) -> Result<(), Failure> {
        self.watch.start(self.json)
    }

    fn deliver(
        &mut self,
        client: &Client,
        answer: &Value,
        reconnected: Option<&str>,
    ) -> Result<(Option<Failure>, Vec<String>), Failure> {
        deliver(
            client,
            self.json,
            self.follow,
            answer,
            reconnected,
            &self.watch,
        )
    }
}

/// Runs `inbox wake` for `client`, with `follow` `inbox wake --follow`, and answers its exit
/// status. Every line it writes to standard error starts with `inbox wake: `, but the
/// reconnect's own line. `before_exit` is `--before-exit` of a debug build (`Watch::before_exit`),
/// `None` in a release build.
pub fn run(client: &Client, json: bool, follow: bool, before_exit: Option<&Path>) -> i32 {
    let watch = Arc::new(Watch::new());
    let result = wake(client, json, follow, &watch);
    *watch.hold() = true;
    let status = match result {
        Ok(status) => status,
        Err(failure) => report(&failure, json),
    };
    #[cfg(debug_assertions)]
    watch.before_exit(before_exit);
    #[cfg(not(debug_assertions))]
    let _ = before_exit;
    status
}

/// Writes `failure` as every command does (the document with `--json`, one line on standard
/// error) and answers its exit status.
pub fn report(failure: &Failure, json: bool) -> i32 {
    if json {
        output::print_document(&failure.document());
    }
    output::warn(&format!("inbox wake: {}", failure.message()));
    failure.status()
}

fn wake(client: &Client, json: bool, follow: bool, watch: &Arc<Watch>) -> Result<i32, Failure> {
    // First of all: a wake whose rows no one would read takes no lock and asks nothing.
    if let Err(error) = output::reaches_a_reader() {
        return Err(Failure::Local(format!(
            "standard output: {error}: nothing is asked of the service"
        )));
    }
    rounds(
        client,
        &mut Output {
            json,
            follow,
            watch: Arc::clone(watch),
        },
    )
}

/// The rounds of a wake for `surface`: whether it admits the inbox, the signals, the watch of the
/// surface, the lock of the wake, its start (tried again while the service does not answer), then
/// a read of `Pending` that holds a row in `Sent` ([`held`]) and the rows handed to the
/// surface, once, or, for a surface that follows, again and again until a failure or a signal
/// ends it. A round of a surface that follows in which a row failed, or that got a row the round
/// before marked delivered, is followed by the pause of a failure.
///
/// # Errors
///
/// The failure that ends it: [`Failure::AnotherWake`] when another process holds the lock.
pub fn rounds(client: &Client, surface: &mut dyn Surface) -> Result<i32, Failure> {
    surface.admits(client)?;
    end_on_signal()?;
    surface.watched()?;
    let address = client.own_inbox().0;
    let _lock = lock(client.outbox.path(), &address)?.ok_or_else(|| {
        Failure::AnotherWake(format!("another wake of {address} runs on this host"))
    })?;
    // The start (the bridge's open) waits for the service as a read of `Pending` does.
    let mut failures = 0_u32;
    loop {
        match surface.started(client) {
            Ok(()) => break,
            Err(failure) if tried_again(&failure) => {
                say_once(&mut failures, &failure, surface.name());
            }
            Err(failure) => return Err(failure),
        }
    }
    let follow = surface.follows();
    // Rounds in a row of `--follow` in which a mark failed, or that got a row the round before
    // marked delivered: the row may still be in `Sent`, and the next round may get it again at
    // once.
    let mut failed_rounds = 0_u32;
    // The rows the round before marked delivered, only those: no set grows over a long run.
    let mut marked: Vec<String> = Vec::new();
    loop {
        let (answer, reconnected) = held(client, surface.name())?;
        if follow {
            // The record may have changed since the start: an inbox opened with another adapter
            // meanwhile gets no row of this round, and the end of the process frees the lock.
            surface.admits(client)?;
        }
        let again = output::rows(&answer).iter().any(|row| {
            marked
                .iter()
                .any(|id| id == output::text(row, "message_id"))
        });
        let (first, now_marked) = surface.deliver(client, &answer, reconnected.as_deref())?;
        if !follow {
            return Ok(first.map_or(0, |failure| failure.status()));
        }
        marked = now_marked;
        if first.is_some() || again {
            std::thread::sleep(pause(failed_rounds));
            failed_rounds = failed_rounds.saturating_add(1);
        } else {
            failed_rounds = 0;
        }
    }
}

/// A mode that is the adapter `adapter` (`inbox wake --follow` is `terminal`, `inbox bridge` is
/// `slack`): when this host's record of an `inbox open` of the caller names another adapter (the
/// inbox is a Claude Code session's, say), it ends with [`Failure::OtherAdapter`], whose line
/// names the mode `mode` and the `inbox open` that gives the inbox to it, followed by `purpose`,
/// and does not take the lock that adapter's wake takes. Without a record it goes on as `inbox
/// wake` does.
///
/// # Errors
///
/// [`Failure::OtherAdapter`].
pub fn only(client: &Client, adapter: &str, mode: &str, purpose: &str) -> Result<(), Failure> {
    let principal = &client.own_inbox().0;
    match client.outbox.opened(principal) {
        Some(record) if record.adapter != adapter => Err(Failure::OtherAdapter(format!(
            "{principal} was opened on this host with the adapter {}, and `{mode}` is the \
             adapter {adapter}: run `inbox open --adapter {adapter}` {purpose}",
            record.adapter
        ))),
        _ => Ok(()),
    }
}

/// Writes the rows of `answer` (with `--json` its document, with the member `reconnected` when
/// a reconnect ran) and flushes them, then sends `MarkDelivered` for each row, one after
/// another; a round of `--follow` rings the bell after its text ([`output::try_print_round`]).
/// Answers the first mark that failed (each is named in one line on standard error) and the
/// message ids whose mark succeeded. It holds `watch` from the write to the last mark: the watch
/// does not end the process in between.
///
/// # Errors
///
/// [`Failure::Local`] when standard output cannot be written, or is a pipe or a FIFO whose reader
/// is gone once the rows are written: then no mark is sent.
fn deliver(
    client: &Client,
    json: bool,
    follow: bool,
    answer: &Value,
    reconnected: Option<&str>,
    watch: &Watch,
) -> Result<(Option<Failure>, Vec<String>), Failure> {
    let mut round = watch.hold();
    let done = commands::pending_done(answer);
    let written = if json {
        let document = match reconnected {
            Some(member) => output::with_member(&done.document, "reconnected", member),
            None => std::borrow::Cow::Borrowed(done.document.as_str()),
        };
        output::try_print_document(&document)
    } else if follow {
        output::try_print_round(&done.text)
    } else {
        output::try_print_text(&done.text)
    };
    if let Err(error) = written {
        *round = true;
        return Err(Failure::Local(format!(
            "standard output: {error}: no message is marked delivered"
        )));
    }
    // The reader may have closed the pipe after the rows went in: they are not marked.
    if let Err(error) = output::reaches_a_reader() {
        *round = true;
        return Err(Failure::Local(format!(
            "standard output: {error}: no message is marked delivered"
        )));
    }
    if !follow {
        // The wake ends after these marks, whatever the reader does meanwhile.
        *round = true;
    }
    let mut first: Option<Failure> = None;
    let mut marked = Vec::new();
    for row in output::rows(answer) {
        let message_id = output::text(row, "message_id");
        match commands::mark_delivered(client, message_id) {
            Ok(_) => marked.push(message_id.to_owned()),
            Err(failure) => {
                output::warn(&format!(
                    "inbox wake: {message_id} was printed and not marked delivered: {}",
                    failure.message()
                ));
                first.get_or_insert(failure);
            }
        }
    }
    Ok((first, marked))
}

/// Reads `Pending` until it has a row in `Sent`, as the module says: answers that answer with its
/// rows in `Sent` only, and the document of the last reconnect that ran. A `409 inbox not open`
/// runs the reconnect and is handled by what its open came to ([`Reopen`]), as the module says.
fn held(client: &Client, name: &str) -> Result<(Value, Option<String>), Failure> {
    let mut wait = 0;
    let mut failures = 0_u32;
    let mut quick = 0_u32;
    let mut reconnected = None;
    // Whether the last answer said "not open" and its reconnect opened the inbox: a second "not
    // open" right after it ends the wake instead of opening again.
    let mut reopened = false;
    loop {
        let asked = Instant::now();
        match commands::undelivered(client, wait) {
            Ok(answer) => {
                failures = 0;
                reopened = false;
                if let Some(reconnect) = commands::reconnect(client, Typed::Other) {
                    output::warn(&reconnect.line);
                    reconnected = Some(reconnect.document);
                }
                if !matches!(answer.member("rows"), Some(Value::Array(_))) {
                    return Err(Failure::Unexpected {
                        status: 200,
                        detail: format!(
                            "the answer is not the view of Pending: {}",
                            output::document(&answer)
                        ),
                    });
                }
                let answer = in_sent(answer);
                if !output::rows(&answer).is_empty() {
                    return Ok((answer, reconnected));
                }
                if wait != 0 {
                    if asked.elapsed() < QUICK {
                        std::thread::sleep(pause(quick));
                        quick = quick.saturating_add(1);
                    } else {
                        quick = 0;
                    }
                }
                wait = if wait == 0 { WAIT_SECONDS } else { 0 };
            }
            Err(failure) if not_open(&failure) => {
                let came = commands::reconnect(client, Typed::Other).and_then(|reconnect| {
                    output::warn(&reconnect.line);
                    reconnected = Some(reconnect.document);
                    reconnect.open
                });
                let principal = &client.own_inbox().0;
                match came {
                    _ if reopened => {
                        return Err(Failure::NotOpen(format!(
                            "{principal} is still not open at the service right after an open \
                             of it succeeded: run `inbox open` to open it again"
                        )));
                    }
                    Some(Reopen::Failed(failure)) if tried_again(&failure) => {
                        // The open did not reach the service, or got an answer that may pass:
                        // as a failure of the read, and the next read runs the reconnect again.
                        say_once(&mut failures, &failure, name);
                        reopened = false;
                        wait = 0;
                    }
                    Some(Reopen::Failed(failure @ Failure::Refused { .. })) => {
                        return Err(Failure::NotOpen(format!(
                            "{principal} is not open at the service, and its open again with the \
                             adapter of this host's `inbox open` was {}: run `inbox open` to see \
                             why",
                            failure.message()
                        )));
                    }
                    Some(Reopen::Failed(failure)) => return Err(failure),
                    Some(Reopen::Opened) => {
                        failures = 0;
                        reopened = true;
                        wait = 0;
                    }
                    Some(Reopen::Closed) | None if client.outbox.opened(principal).is_none() => {
                        return Err(Failure::NotOpen(format!(
                            "{principal} is not open at the service, and this host has no record \
                             of an `inbox open` of it (never opened here, or closed by `inbox \
                             close`): run `inbox open` first"
                        )));
                    }
                    Some(Reopen::Closed) | None => {
                        // This host's record of an open is of the start that answered (an `inbox
                        // open` of the owner after this read began, or an answer without an
                        // epoch): nothing to open again; the read is asked once more at once.
                        failures = 0;
                        reopened = true;
                        wait = 0;
                    }
                }
            }
            Err(failure) if tried_again(&failure) => {
                say_once(&mut failures, &failure, name);
                wait = 0;
            }
            Err(failure) => return Err(failure),
        }
    }
}

/// One failure of a run of them that is tried again: one line on standard error at the first of
/// the run, then the pause for the run's length so far.
fn say_once(failures: &mut u32, failure: &Failure, name: &str) {
    if *failures == 0 {
        output::warn(&format!(
            "{name}: {}; trying again until the service answers",
            failure.message()
        ));
    }
    std::thread::sleep(pause(*failures));
    *failures = failures.saturating_add(1);
}

/// A view answer with its rows in `Sent` only. The service answers `undelivered=1` with no other
/// row; this guards against a service that does not know the parameter and answers its whole
/// view: a row in `Delivered` is neither printed nor marked by a wake, `inbox pending` lists it.
/// Every other member stays as it was.
fn in_sent(answer: Value) -> Value {
    let Value::Object(members) = answer else {
        return answer;
    };
    Value::Object(
        members
            .into_iter()
            .map(|(name, value)| match (name.as_str(), value) {
                ("rows", Value::Array(rows)) => (
                    name,
                    Value::Array(
                        rows.into_iter()
                            .filter(|row| output::text(row, "state") == "Sent")
                            .collect(),
                    ),
                ),
                (_, value) => (name, value),
            })
            .collect(),
    )
}

/// The child process a surface waits for while it runs (the `connectors` of a post or of a read of
/// the channel of `inbox bridge`, one at a time by the bridge's turn): the signal path ends it
/// before the process ends ([`end_on_signal`]).
static CHILD: Mutex<Option<Child>> = Mutex::new(None);

/// The slot of the child process a surface waits for ([`CHILD`]). A surface puts its child there
/// while it holds the slot, before the signal path can look, and takes it out once it has ended.
pub fn child() -> MutexGuard<'static, Option<Child>> {
    CHILD.lock().unwrap_or_else(PoisonError::into_inner)
}

/// SIGINT and SIGTERM end the process at once by their default action, also when it was started
/// with them ignored: a thread waits for either, takes the slot of the child
/// ([`CHILD`]), ends the child in it with its process group ([`end_group`]) and waits for it, and
/// takes the signal by its default action. It holds the slot until the process has ended, so no
/// thread starts a child after the group was ended. So no process of the group of a running
/// `connectors` of the bridge outlives a bridge ended by one of them. The `flock` is the kernel's
/// to release.
fn end_on_signal() -> Result<(), Failure> {
    let mut signals = Signals::new([SIGINT, SIGTERM])
        .map_err(|error| Failure::Local(format!("the signal handlers: {error}")))?;
    std::thread::Builder::new()
        .name("inbox-wake-signals".to_owned())
        .spawn(move || {
            if let Some(signal) = signals.forever().next() {
                let mut held = child();
                if let Some(running) = held.take() {
                    end_group(running);
                }
                let _ = signal_hook::low_level::emulate_default_handler(signal);
                signal_hook::low_level::exit(128 + signal);
            }
        })
        .map_err(|error| Failure::Local(format!("the signal thread: {error}")))?;
    Ok(())
}

/// Ends `running`, the leader of a process group of its own, with that group: SIGKILL to the
/// group, so the processes the child started end with it, then to the child alone in case it left
/// its group, then waits for it. The child is not yet waited for while it is in [`CHILD`], so its
/// id names it and its group and no other process. A process that left the group (a session of
/// its own) is not reached.
pub fn end_group(mut running: Child) {
    let group = rustix::process::Pid::from_child(&running);
    let _ = rustix::process::kill_process_group(group, rustix::process::Signal::KILL);
    let _ = running.kill();
    let _ = running.wait();
}

/// The lock file of the wake of `address` beside the outbox file `outbox`.
fn lock_path(outbox: &Path, address: &str) -> PathBuf {
    let mut path = outbox.as_os_str().to_owned();
    path.push(format!(".wake.{}", digits(address)));
    PathBuf::from(path)
}

/// The 16 hex digits of `address` in the names of the files beside the outbox file that are of one
/// address: the lock of the wake, the record of the chat posts of `inbox bridge`.
pub fn digits(address: &str) -> String {
    format!("{:016x}", fnv1a(address.as_bytes()))
}

/// The 64-bit FNV-1a hash of `bytes`: a name of fixed length for any address.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Takes the lock of the wake of `address`: the open file whose `flock` this process holds, or
/// `None` when another process holds it. The file is opened as [`lock_file`] opens it.
///
/// # Errors
///
/// [`Failure::Local`] when the file cannot be opened or locked, or is no regular file.
fn lock(outbox: &Path, address: &str) -> Result<Option<File>, Failure> {
    let path = lock_path(outbox, address);
    let file = lock_file(&path)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => {
            Err(Failure::Local(format!("{}: {error}", path.display())))
        }
    }
}

/// Opens the lock file at `path`, not yet locked: its directory (the directory of the outbox
/// file) is made with mode 0700 as the outbox lock makes it; the file is created with mode 0600,
/// opened without blocking and without following a symbolic link in its place (`O_NOFOLLOW`: a
/// link is refused, nothing is created through it), and must be a regular file.
///
/// # Errors
///
/// [`Failure::Local`] when the file cannot be opened, or is no regular file.
fn lock_file(path: &Path) -> Result<File, Failure> {
    let local = |error: std::io::Error| Failure::Local(format!("{}: {error}", path.display()));
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)
            .map_err(local)?;
    }
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
        .open(path)
        .map_err(local)?;
    if !file.metadata().map_err(local)?.is_file() {
        return Err(Failure::Local(format!(
            "{}: it is no regular file",
            path.display()
        )));
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use super::{lock_path, pause};

    #[test]
    fn the_pause_starts_short_doubles_and_stops_at_30_s() {
        let pauses: Vec<Duration> = (0..12).map(pause).collect();
        assert_eq!(pauses[0], Duration::from_millis(100));
        for pair in pauses.windows(2) {
            assert!(
                pair[1] == pair[0] * 2 || pair[1] == Duration::from_secs(30),
                "{pauses:?}"
            );
        }
        assert_eq!(
            pauses[9],
            Duration::from_millis(51_200).min(Duration::from_secs(30))
        );
        for failures in [9, 10, 40, u32::MAX] {
            assert_eq!(pause(failures), Duration::from_secs(30), "{failures}");
        }
    }

    #[test]
    fn each_address_has_a_lock_file_of_its_own_beside_the_outbox_file() {
        let outbox = Path::new("state/outbox.jsonl");
        let bob = lock_path(outbox, "agent:test/bob");
        let alice = lock_path(outbox, "agent:test/alice");
        assert_ne!(bob, alice);
        assert_eq!(bob, lock_path(outbox, "agent:test/bob"));
        for path in [&bob, &alice] {
            let name = path.file_name().unwrap().to_str().unwrap();
            let digits = name.strip_prefix("outbox.jsonl.wake.").unwrap();
            assert_eq!(digits.len(), 16, "{name}");
            assert!(
                digits.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "{name}"
            );
            assert_eq!(path.parent(), outbox.parent());
        }
        let long = "agent:host/".to_owned() + &"x".repeat(4096);
        let name_length = lock_path(outbox, &long).file_name().unwrap().len();
        assert_eq!(name_length, "outbox.jsonl.wake.".len() + 16);
    }
}
