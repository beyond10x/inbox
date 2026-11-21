//! The outbox file: every envelope the client posted, kept until it is collected, refused or
//! expired, so `inbox repost` can post it again after a restart of the service lost it.
//!
//! One line per envelope, the exact text that was posted (the generated encoding of
//! `PostMessage`, signature included); nothing secret. Every change runs under an exclusive
//! lock of a file beside it (`<outbox>.lock`), reads the whole file, and replaces it with a
//! file written beside it and renamed over it, so two `inbox` processes that write at once
//! lose no entry and a reader never sees half a file.
//!
//! The epochs file beside it (`<outbox>.epochs`) records, per kept envelope whose post the
//! service said arrived, the epoch of the start that said so ([`Epoch`], the answer's
//! `Inbox-Epoch`), for `repost` to leave the envelope alone in that start: one line per
//! envelope, its message id, a tab and the 32 hex digits. It is changed under the same lock and
//! replaced the same way, after the outbox file where both change, so an interruption leaves an
//! envelope without an epoch (posted again), never an epoch of an envelope that was not kept.
//! Reading it never fails a command: a missing or unreadable file, a line of another form and
//! the lines of a message id named twice give the envelopes concerned no epoch. Each time it is
//! written it holds the entries of the envelopes the outbox file keeps and no other.

use std::collections::{HashMap, HashSet};
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use inbox::messaging::PostMessage;
use inbox::server::json;
use inbox::server::wire;
use inbox_verify::Instant;

use crate::client::Epoch;
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
    /// The epoch of the start that said the post arrived, as the epochs file records it.
    pub epoch: Option<Epoch>,
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
        self.change(now, false, |lines, _| lines.push(text.to_owned()))
    }

    /// Drops the envelope with this message id, and what expired at `now`.
    ///
    /// # Errors
    ///
    /// [`Failure::Local`] when the file cannot be read or written.
    pub fn remove(&self, message_id: &str, now: Instant) -> Result<(), Failure> {
        self.change(now, false, |lines, _| *lines = without(lines, message_id))
    }

    /// Records `epoch` in the epochs file for the envelope with this message id, in place of the
    /// epoch it had, when the outbox file still keeps it; drops what expired at `now`.
    ///
    /// # Errors
    ///
    /// [`Failure::Local`] when a file cannot be read or written.
    pub fn record_epoch(
        &self,
        message_id: &str,
        epoch: &Epoch,
        now: Instant,
    ) -> Result<(), Failure> {
        self.record_epochs(&[(message_id.to_owned(), epoch.clone())], now)
    }

    /// [`Outbox::record_epoch`] for each `(message id, epoch)` of `arrived`, with one read and at
    /// most one write of the epochs file.
    ///
    /// # Errors
    ///
    /// [`Failure::Local`] when a file cannot be read or written.
    pub fn record_epochs(&self, arrived: &[(String, Epoch)], now: Instant) -> Result<(), Failure> {
        if arrived.is_empty() {
            return Ok(());
        }
        self.change(now, true, |lines, epochs| {
            let kept = message_ids(lines);
            for (message_id, epoch) in arrived {
                if kept.contains(message_id.as_str()) {
                    epochs.record(message_id, epoch);
                }
            }
        })
    }

    /// The envelopes kept for `from` that have not expired at `now`, in the order they were
    /// posted, each with the epoch the epochs file records for it, and every line of the file
    /// that does not read as an envelope; drops what expired.
    ///
    /// # Errors
    ///
    /// [`Failure::Local`] when the file cannot be read or written.
    pub fn kept(&self, from: &str, now: Instant) -> Result<Contents, Failure> {
        let mut contents = Contents::default();
        self.change(now, true, |lines, epochs| {
            let mut kept = of_sender(lines, from);
            for entry in &mut kept {
                entry.epoch = epochs.of(&entry.envelope.message_id.0);
            }
            contents = Contents {
                kept,
                unreadable: unreadable(lines),
            };
        })?;
        Ok(contents)
    }

    /// Runs `change` over the lines of the file and, when `with_epochs`, what the epochs file
    /// says, under the lock, after dropping every envelope whose `expires` is at or before `now`;
    /// writes the file again when its lines changed, then the epochs file when `change` recorded
    /// an epoch. Without `with_epochs` the epochs file is neither read nor written. A line that
    /// does not read as an envelope is kept as it is.
    fn change(
        &self,
        now: Instant,
        with_epochs: bool,
        change: impl FnOnce(&mut Vec<String>, &mut Epochs),
    ) -> Result<(), Failure> {
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
        let epochs_path = beside(&self.path, ".epochs");
        let mut epochs = if with_epochs {
            Epochs::parse(&fs::read_to_string(&epochs_path).unwrap_or_default())
        } else {
            Epochs::default()
        };
        change(&mut lines, &mut epochs);
        if lines != before {
            replace(&self.path, &lines).map_err(local)?;
        }
        if epochs.changed {
            replace(&epochs_path, &epochs.lines(&lines))
                .map_err(|error| Failure::Local(format!("{}: {error}", epochs_path.display())))?;
        }
        lock.unlock().map_err(local)
    }
}

/// Replaces the file at `path` with `lines`, each ended by a newline: written to `<path>.new`
/// (mode 0600) and renamed over it.
fn replace(path: &Path, lines: &[String]) -> std::io::Result<()> {
    let staged = beside(path, ".new");
    let mut file: File = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&staged)?;
    let mut out = String::new();
    for line in lines {
        out.push_str(line);
        out.push('\n');
    }
    file.write_all(out.as_bytes())?;
    file.sync_all()?;
    fs::rename(&staged, path)
}

fn beside(path: &Path, suffix: &str) -> PathBuf {
    let mut beside = path.to_owned();
    beside.as_mut_os_string().push(suffix);
    beside
}

/// What the epochs file says: the epoch of each message id one line of the form
/// `<message id>\t<32 lower-case hex digits>` names. A message id that two lines name, whatever
/// their form, has none.
#[derive(Debug, Default)]
struct Epochs {
    recorded: HashMap<String, Epoch>,
    changed: bool,
}

impl Epochs {
    /// Reads the file's text in one pass over its lines.
    fn parse(text: &str) -> Self {
        let mut named: HashMap<&str, (usize, &str)> = HashMap::new();
        for line in text.lines() {
            let (id, digits) = line.split_once('\t').unwrap_or((line, ""));
            let entry = named.entry(id).or_insert((0, digits));
            entry.0 += 1;
        }
        let recorded = named
            .into_iter()
            .filter(|(id, (count, _))| !id.is_empty() && *count == 1)
            .filter_map(|(id, (_, digits))| {
                Epoch::parse(digits).map(|epoch| (id.to_owned(), epoch))
            })
            .collect();
        Self {
            recorded,
            changed: false,
        }
    }

    fn of(&self, message_id: &str) -> Option<Epoch> {
        self.recorded.get(message_id).cloned()
    }

    fn record(&mut self, message_id: &str, epoch: &Epoch) {
        self.recorded.insert(message_id.to_owned(), epoch.clone());
        self.changed = true;
    }

    /// The lines of the file, in the order of the envelopes `kept` holds, for those and no other.
    fn lines(&self, kept: &[String]) -> Vec<String> {
        let mut written = HashSet::new();
        kept.iter()
            .filter_map(|line| decode(line).map(|envelope| envelope.message_id.0))
            .filter_map(|id| {
                let epoch = self.recorded.get(&id)?;
                written
                    .insert(id.clone())
                    .then(|| format!("{id}\t{}", epoch.as_str()))
            })
            .collect()
    }
}

/// The message ids of the envelopes `lines` holds.
fn message_ids(lines: &[String]) -> HashSet<String> {
    lines
        .iter()
        .filter_map(|line| decode(line).map(|envelope| envelope.message_id.0))
        .collect()
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

/// The envelopes `from` sent, in their order, without their epochs.
fn of_sender(lines: &[String], from: &str) -> Vec<Kept> {
    lines
        .iter()
        .filter_map(|line| {
            decode(line)
                .filter(|envelope| envelope.from.0 == from)
                .map(|envelope| Kept {
                    envelope,
                    text: line.clone(),
                    epoch: None,
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

    use super::{Epochs, of_sender, unexpired, unreadable, without};
    use crate::client::Epoch;

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

    const EPOCH: &str = "0123456789abcdef0123456789abcdef";
    const OTHER: &str = "fedcba9876543210fedcba9876543210";

    fn epoch(text: &str) -> Epoch {
        Epoch::parse(text).unwrap()
    }

    #[test]
    fn an_epochs_line_is_a_message_id_a_tab_and_32_hex_digits() {
        let text = format!("{}\t{EPOCH}\n{}\t{OTHER}\n", id(1), id(2));
        let epochs = Epochs::parse(&text);
        assert_eq!(epochs.of(&id(1)), Some(epoch(EPOCH)));
        assert_eq!(epochs.of(&id(2)), Some(epoch(OTHER)));
        assert_eq!(epochs.of(&id(3)), None);
        assert!(!epochs.changed);
    }

    #[test]
    fn a_line_of_another_form_or_an_id_named_twice_gives_no_epoch() {
        let text = [
            format!("{}\t{}", id(1), EPOCH.to_uppercase()),
            format!("{}\t{EPOCH}\textra", id(2)),
            format!("{} {EPOCH}", id(3)),
            format!("\t{EPOCH}"),
            format!("{}\t{EPOCH}", id(4)),
            format!("{}\t{OTHER}", id(4)),
            format!("{}\t{EPOCH}", id(5)),
            format!("{}\tnot hex", id(5)),
            "\u{fffd} not an entry".to_owned(),
            format!("{}\t{EPOCH}", id(6)),
        ]
        .join("\n");
        let epochs = Epochs::parse(&text);
        for n in 1..=5 {
            assert_eq!(epochs.of(&id(n)), None, "{n}");
        }
        assert_eq!(epochs.of(""), None);
        assert_eq!(epochs.of(&id(6)), Some(epoch(EPOCH)));
        assert_eq!(Epochs::parse("").of(&id(6)), None);
    }

    #[test]
    fn a_recorded_epoch_replaces_the_one_before_and_ids_named_twice() {
        let text = format!(
            "{}\t{EPOCH}\n{}\t{EPOCH}\n{}\t{OTHER}\n",
            id(1),
            id(2),
            id(2)
        );
        let mut epochs = Epochs::parse(&text);
        epochs.record(&id(1), &epoch(OTHER));
        epochs.record(&id(2), &epoch(EPOCH));
        assert!(epochs.changed);
        assert_eq!(epochs.of(&id(1)), Some(epoch(OTHER)));
        assert_eq!(epochs.of(&id(2)), Some(epoch(EPOCH)));
        let kept = [
            envelope(1, "agent:test/alice", "2026-11-22T12:00:00Z"),
            envelope(2, "agent:test/alice", "2026-11-22T12:00:00Z"),
        ];
        assert_eq!(
            epochs.lines(&kept),
            [format!("{}\t{OTHER}", id(1)), format!("{}\t{EPOCH}", id(2))]
        );
    }

    #[test]
    fn an_entry_whose_envelope_the_outbox_does_not_keep_is_not_written() {
        let text = format!("{}\t{EPOCH}\n{}\t{EPOCH}\n", id(1), id(9));
        let mut epochs = Epochs::parse(&text);
        epochs.record(&id(2), &epoch(OTHER));
        let kept = [
            envelope(1, "agent:test/alice", "2026-11-22T12:00:00Z"),
            envelope(2, "agent:test/alice", "2026-11-22T12:00:00Z"),
            "not an envelope".to_owned(),
        ];
        assert_eq!(
            epochs.lines(&kept),
            [format!("{}\t{EPOCH}", id(1)), format!("{}\t{OTHER}", id(2))]
        );
    }
}
