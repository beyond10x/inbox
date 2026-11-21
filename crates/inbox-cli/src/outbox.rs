//! The outbox file: every envelope the client posted, kept until it is collected, refused or
//! expired, so `inbox repost` can post it again after a restart of the service lost it.
//!
//! One line per envelope, the exact text that was posted (the generated encoding of
//! `PostMessage`, signature included); nothing secret. Every change runs under an exclusive
//! lock of a file beside it (`<outbox>.lock`), reads the whole file, and replaces it with a
//! file written beside it and renamed over it, so two `inbox` processes that write at once
//! lose no entry and a reader never sees half a file.

use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use inbox::messaging::PostMessage;
use inbox::server::json;
use inbox::server::wire;
use inbox_verify::Instant;

use crate::failure::Failure;

/// The outbox file name under the state directory.
const FILE: &str = "outbox.jsonl";

/// One kept envelope.
#[derive(Debug, Clone)]
pub struct Kept {
    /// The envelope.
    pub envelope: PostMessage,
    /// The text that was posted.
    pub text: String,
}

/// A line of the outbox file that does not read as an envelope. It is never dropped: `outbox`
/// lists it and `repost` names it, for the owner of the file to remove by hand.
#[derive(Debug, Clone)]
pub struct Unreadable {
    /// Its number in the file, from 1.
    pub line: usize,
    /// Why the generated decoder refused it.
    pub reason: String,
}

/// What the outbox file holds for one sender.
#[derive(Debug, Default)]
pub struct Contents {
    /// The sender's envelopes.
    pub kept: Vec<Kept>,
    /// Every line that does not read as an envelope, whoever wrote it.
    pub unreadable: Vec<Unreadable>,
}

/// The outbox file.
#[derive(Debug)]
pub struct Outbox {
    path: PathBuf,
}

impl Outbox {
    /// The outbox at `given` (the flag `--outbox` or `INBOX_OUTBOX`), else
    /// `$XDG_STATE_HOME/inbox/outbox.jsonl`, else `$HOME/.local/state/inbox/outbox.jsonl`.
    ///
    /// # Errors
    ///
    /// [`Failure::Local`] when none of them is set.
    pub fn locate(given: Option<&Path>) -> Result<Self, Failure> {
        let state = || {
            let from = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
            from("XDG_STATE_HOME")
                .map(PathBuf::from)
                .or_else(|| from("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        };
        let path = match given {
            Some(path) => path.to_owned(),
            None => state()
                .map(|state| state.join("inbox").join(FILE))
                .ok_or_else(|| {
                    Failure::Local(
                        "the outbox: pass --outbox <path>, or set INBOX_OUTBOX, XDG_STATE_HOME \
                         or HOME"
                            .to_owned(),
                    )
                })?,
        };
        Ok(Self { path })
    }

    /// The path of the file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Keeps `text`, the envelope as it is about to be posted, and drops what expired at `now`.
    ///
    /// # Errors
    ///
    /// [`Failure::Local`] when the file cannot be read or written.
    pub fn keep(&self, text: &str, now: Instant) -> Result<(), Failure> {
        self.change(now, |lines| lines.push(text.to_owned()))
    }

    /// Drops the envelope with this message id, and what expired at `now`.
    ///
    /// # Errors
    ///
    /// [`Failure::Local`] when the file cannot be read or written.
    pub fn remove(&self, message_id: &str, now: Instant) -> Result<(), Failure> {
        self.change(now, |lines| *lines = without(lines, message_id))
    }

    /// The envelopes kept for `from` that have not expired at `now`, in the order they were
    /// posted, and every line of the file that does not read as an envelope; drops what expired.
    ///
    /// # Errors
    ///
    /// [`Failure::Local`] when the file cannot be read or written.
    pub fn kept(&self, from: &str, now: Instant) -> Result<Contents, Failure> {
        let mut contents = Contents::default();
        self.change(now, |lines| {
            contents = Contents {
                kept: of_sender(lines, from),
                unreadable: unreadable(lines),
            };
        })?;
        Ok(contents)
    }

    /// Runs `change` over the lines of the file under the lock, after dropping every envelope
    /// whose `expires` is at or before `now`, and writes the file again when they changed. A line
    /// that does not read as an envelope is kept as it is.
    fn change(&self, now: Instant, change: impl FnOnce(&mut Vec<String>)) -> Result<(), Failure> {
        let shown = self.path.display();
        let local = |error: std::io::Error| Failure::Local(format!("{shown}: {error}"));
        if let Some(parent) = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(parent)
                .map_err(local)?;
        }
        let lock = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(beside(&self.path, ".lock"))
            .map_err(local)?;
        lock.lock().map_err(local)?;
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
            Err(error) => return Err(local(error)),
        };
        let before: Vec<String> = text.lines().map(str::to_owned).collect();
        let mut lines = unexpired(&before, now);
        change(&mut lines);
        if lines != before {
            let staged = beside(&self.path, ".new");
            let mut file: File = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&staged)
                .map_err(local)?;
            let mut out = String::new();
            for line in &lines {
                out.push_str(line);
                out.push('\n');
            }
            file.write_all(out.as_bytes()).map_err(local)?;
            file.sync_all().map_err(local)?;
            fs::rename(&staged, &self.path).map_err(local)?;
        }
        lock.unlock().map_err(local)
    }
}

fn beside(path: &Path, suffix: &str) -> PathBuf {
    let mut beside = path.to_owned();
    beside.as_mut_os_string().push(suffix);
    beside
}

/// The envelope a line holds, read by the generated decoder, or why it does not read.
fn read(line: &str) -> Result<PostMessage, String> {
    let value = json::parse(line).map_err(|error| error.to_string())?;
    wire::decode_command_inbox_messaging_post_message(&value, "").map_err(|error| error.to_string())
}

/// The envelope a line holds, read by the generated decoder.
fn decode(line: &str) -> Option<PostMessage> {
    read(line).ok()
}

/// Every line that does not read as an envelope: its number (from 1) and why.
fn unreadable(lines: &[String]) -> Vec<Unreadable> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(at, line)| {
            read(line).err().map(|reason| Unreadable {
                line: at + 1,
                reason,
            })
        })
        .collect()
}

/// The lines whose envelope has not expired at `now`: its `expires` is after it (the model's
/// reading: `expires <= now` is expired). A line that does not read as an envelope, or whose
/// `expires` does not read, is kept as it is.
fn unexpired(lines: &[String], now: Instant) -> Vec<String> {
    lines
        .iter()
        .filter(|line| {
            decode(line).is_none_or(|envelope| match Instant::try_from(&envelope.expires) {
                Ok(expires) => expires > now,
                Err(_) => true,
            })
        })
        .cloned()
        .collect()
}

/// The lines but the envelope with this message id.
fn without(lines: &[String], message_id: &str) -> Vec<String> {
    lines
        .iter()
        .filter(|line| decode(line).is_none_or(|envelope| envelope.message_id.0 != message_id))
        .cloned()
        .collect()
}

/// The envelopes `from` sent, in their order.
fn of_sender(lines: &[String], from: &str) -> Vec<Kept> {
    lines
        .iter()
        .filter_map(|line| {
            decode(line)
                .filter(|envelope| envelope.from.0 == from)
                .map(|envelope| Kept {
                    envelope,
                    text: line.clone(),
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use inbox::messaging::{
        Address, Body, DedupKey, Kind, MessageId, PostMessage, Principal, Signature, ThreadId,
    };
    use inbox::primitives::Timestamp;
    use inbox::server::wire;
    use inbox_verify::Instant;

    use super::{of_sender, unexpired, unreadable, without};

    #[test]
    fn a_line_that_does_not_decode_is_named_with_its_number_and_the_decoders_reason() {
        let lines = vec![
            envelope(1, "agent:test/alice", "2026-11-22T12:00:00Z"),
            "not json".to_owned(),
            envelope(2, "agent:test/alice", "2026-11-22T12:00:00Z").replace(&id(2), "short"),
        ];
        let found = unreadable(&lines);
        assert_eq!(
            found.iter().map(|entry| entry.line).collect::<Vec<_>>(),
            [2, 3]
        );
        assert!(
            found[1].reason.contains("message_id"),
            "{}",
            found[1].reason
        );
    }

    /// A ULID-shaped id, distinct per `n`.
    fn id(n: u32) -> String {
        format!("01JD{n:022}")
    }

    fn envelope(n: u32, from: &str, expires: &str) -> String {
        let post = PostMessage {
            message_id: MessageId(id(n)),
            from: Principal(from.to_owned()),
            on_behalf_of: Principal(from.to_owned()),
            authority: None,
            to: Address("agent:test/bob".to_owned()),
            kind: Kind::Note,
            thread: ThreadId(id(n)),
            reply_shape: None,
            decision: None,
            default_on_silence: None,
            created: Timestamp("2026-11-21T12:00:00Z".to_owned()),
            expires: Timestamp(expires.to_owned()),
            dedup_key: DedupKey(id(n)),
            body: Body("hello".to_owned()),
            signature: Signature {
                key_id: "k".to_owned(),
                value: String::new(),
            },
        };
        let mut out = String::new();
        wire::encode_command_inbox_messaging_post_message(&post, &mut out);
        out
    }

    fn at(text: &str) -> Instant {
        Instant::parse(text).unwrap()
    }

    #[test]
    fn an_entry_leaves_when_its_expires_passes_and_not_before() {
        let lines = vec![
            envelope(1, "agent:test/alice", "2026-11-21T13:00:00Z"),
            envelope(2, "agent:test/alice", "2026-11-22T12:00:00Z"),
            "not an envelope".to_owned(),
        ];
        assert_eq!(unexpired(&lines, at("2026-11-21T12:59:59Z")), lines);
        assert_eq!(
            unexpired(&lines, at("2026-11-21T13:00:00Z")),
            [lines[1].clone(), lines[2].clone()]
        );
    }

    #[test]
    fn the_senders_own_envelopes_are_kept_for_it_and_one_is_dropped_by_its_id() {
        let lines = vec![
            envelope(1, "agent:test/alice", "2026-11-22T12:00:00Z"),
            envelope(2, "agent:test/carol", "2026-11-22T12:00:00Z"),
            envelope(3, "agent:test/alice", "2026-11-22T12:00:00Z"),
        ];
        let alice = of_sender(&lines, "agent:test/alice");
        assert_eq!(alice.len(), 2);
        assert_eq!(alice[1].envelope.message_id.0, id(3));
        assert_eq!(alice[1].text, lines[2]);
        assert_eq!(
            without(&lines, &id(1)),
            [lines[1].clone(), lines[2].clone()]
        );
    }
}
