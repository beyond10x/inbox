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
//!   64-bit FNV-1a hash of the caller's address), and holds it until the process ends, a kill
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

use std::fs::{DirBuilder, File, OpenOptions, TryLockError};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
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
}

impl Watch {
    fn new() -> Self {
        Self {
            pipe: output::pipe(),
            ended: Mutex::new(false),
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
        std::thread::Builder::new()
            .name(WATCH_THREAD.to_owned())
            .spawn(move || watch.watch(json))
            .map(drop)
            .map_err(|error| Failure::Local(format!("the watch of the reader: {error}")))
    }

    fn watch(&self, json: bool) {
        let Some(pipe) = &self.pipe else {
            return;
        };
        loop {
            std::thread::sleep(WATCH_PERIOD);
            if output::reader_gone(pipe) {
                let ended = self.hold();
                if *ended {
                    return;
                }
                std::process::exit(report(&no_reader(), json));
            }
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

/// Runs `inbox wake` for `client`, with `follow` `inbox wake --follow`, and answers its exit
/// status. Every line it writes to standard error starts with `inbox wake: `, but the
/// reconnect's own line.
pub fn run(client: &Client, json: bool, follow: bool) -> i32 {
    let watch = Arc::new(Watch::new());
    let result = wake(client, json, follow, &watch);
    *watch.hold() = true;
    match result {
        Ok(status) => status,
        Err(failure) => report(&failure, json),
    }
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
    if follow {
        terminal_only(client)?;
    }
    end_on_signal()?;
    watch.start(json)?;
    let address = client.caller.principal.clone();
    let _lock = lock(client.outbox.path(), &address)?.ok_or_else(|| {
        Failure::AnotherWake(format!("another wake of {address} runs on this host"))
    })?;
    // Rounds in a row of `--follow` in which a mark failed, or that got a row the round before
    // marked delivered: the row may still be in `Sent`, and the next round may get it again at
    // once.
    let mut failed_rounds = 0_u32;
    // The rows the round before marked delivered, only those: no set grows over a long run.
    let mut marked: Vec<String> = Vec::new();
    loop {
        let (answer, reconnected) = held(client)?;
        if follow {
            // The record may have changed since the start: an inbox opened with another adapter
            // meanwhile gets no row of this round, and the end of the process frees the lock.
            terminal_only(client)?;
        }
        let again = output::rows(&answer).iter().any(|row| {
            marked
                .iter()
                .any(|id| id == output::text(row, "message_id"))
        });
        let (first, now_marked) =
            deliver(client, json, follow, &answer, reconnected.as_deref(), watch)?;
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

/// `inbox wake --follow` is the adapter `terminal`: when this host's record of an `inbox open`
/// of the caller names another adapter (the inbox is a Claude Code session's, say), it ends with
/// [`Failure::OtherAdapter`] and does not take the lock that adapter's wake takes. Without a
/// record it goes on as `inbox wake` does.
fn terminal_only(client: &Client) -> Result<(), Failure> {
    let principal = &client.caller.principal;
    match client.outbox.opened(principal) {
        Some(record) if record.adapter != TERMINAL => Err(Failure::OtherAdapter(format!(
            "{principal} was opened on this host with the adapter {}, and `inbox wake --follow` \
             is the adapter {TERMINAL}: run `inbox open --adapter {TERMINAL}` to show its \
             messages in this terminal",
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
fn held(client: &Client) -> Result<(Value, Option<String>), Failure> {
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
                let principal = &client.caller.principal;
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
                        say_once(&mut failures, &failure);
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
                say_once(&mut failures, &failure);
                wait = 0;
            }
            Err(failure) => return Err(failure),
        }
    }
}

/// One failure of a run of them that is tried again: one line on standard error at the first of
/// the run, then the pause for the run's length so far.
fn say_once(failures: &mut u32, failure: &Failure) {
    if *failures == 0 {
        output::warn(&format!(
            "inbox wake: {}; trying again until the service answers",
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

/// SIGINT and SIGTERM end the process at once by their default action, also when it was started
/// with them ignored: a thread waits for either and takes it by its default action. The `flock`
/// is the kernel's to release.
fn end_on_signal() -> Result<(), Failure> {
    let mut signals = Signals::new([SIGINT, SIGTERM])
        .map_err(|error| Failure::Local(format!("the signal handlers: {error}")))?;
    std::thread::Builder::new()
        .name("inbox-wake-signals".to_owned())
        .spawn(move || {
            if let Some(signal) = signals.forever().next() {
                let _ = signal_hook::low_level::emulate_default_handler(signal);
                signal_hook::low_level::exit(128 + signal);
            }
        })
        .map_err(|error| Failure::Local(format!("the signal thread: {error}")))?;
    Ok(())
}

/// The lock file of the wake of `address` beside the outbox file `outbox`.
fn lock_path(outbox: &Path, address: &str) -> PathBuf {
    let mut path = outbox.as_os_str().to_owned();
    path.push(format!(".wake.{:016x}", fnv1a(address.as_bytes())));
    PathBuf::from(path)
}

/// The 64-bit FNV-1a hash of `bytes`: a name of fixed length for any address.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Takes the lock of the wake of `address`: the open file whose `flock` this process holds, or
/// `None` when another process holds it. The directory of the outbox file is made (mode 0700) as
/// the outbox lock makes it; the file is created with mode 0600, opened without blocking and
/// without following a symbolic link in its place (`O_NOFOLLOW`: a link is refused, nothing is
/// created through it), and must be a regular file.
///
/// # Errors
///
/// [`Failure::Local`] when the file cannot be opened or locked, or is no regular file.
fn lock(outbox: &Path, address: &str) -> Result<Option<File>, Failure> {
    let path = lock_path(outbox, address);
    let local = |error: std::io::Error| Failure::Local(format!("{}: {error}", path.display()));
    if let Some(parent) = outbox
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
        .open(&path)
        .map_err(local)?;
    if !file.metadata().map_err(local)?.is_file() {
        return Err(Failure::Local(format!(
            "{}: it is no regular file",
            path.display()
        )));
    }
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => Err(local(error)),
    }
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
