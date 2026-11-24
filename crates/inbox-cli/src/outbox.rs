//! The outbox file: every envelope the client posted, kept until it is collected, refused or
//! expired, so `inbox repost` can post it again after a restart of the service lost it.
//!
//! One line per envelope, the exact text that was posted (the generated encoding of
//! `PostMessage`, signature included); nothing secret. Every change runs under an exclusive
//! lock of a file beside it (`<outbox>.lock`), reads the whole file, and replaces it with a
//! file written beside it and renamed over it, so two `inbox` processes that write at once
//! lose no entry and a reader never sees half a file. The file written beside it is created
//! anew, after whatever was at its place is removed: a link there is not followed
//! ([`replace`]).
//!
//! The epochs file beside it (`<outbox>.epochs`) records, per kept envelope whose post the
//! service said arrived, the epoch of the start that said so ([`Epoch`], the answer's
//! `Inbox-Epoch`), for `repost` to leave the envelope alone in that start: one line per
//! envelope, its message id, a tab and the 32 hex digits. An envelope is marked unsent instead
//! (its message id, a tab and `none`) from the moment it is kept ([`Outbox::keep`]), for the
//! reconnect of a later command to post it again ([`Outbox::due`]); the mark stays while its
//! posts fail in transport, are refused `UnknownInbox` or are answered `408` or `5xx` (the
//! request was not taken), and goes when an answer records an epoch, drops the envelope or
//! refuses this request for good. It is changed under the same lock and replaced the same way,
//! before the outbox file where both change, so an interruption between the two leaves a mark
//! of an envelope the outbox file does not keep, which nothing posts and the next write of the
//! epochs file drops.
//! Reading it fails no command that only reads it: a missing or unreadable file, a line of
//! another form and the lines of a message id named twice give the envelopes concerned no
//! epoch. Each time it is written it holds the entries of the envelopes the outbox file keeps
//! and no other. A change of it that cannot read it (no permission, not UTF-8, over its bound,
//! no regular file) fails with [`Failure::Local`] before any write: written with the change's
//! own entry only, it would lose every other entry. A missing file is read as empty.
//!
//! The file of the opened inbox beside it (`<outbox>.opened`) records, per inbox its owner
//! opened with `inbox open` on this host, the adapter it opened it with and the epoch of the
//! answer that said the inbox is open: one JSON object per line, `{"inbox":…,"adapter":…,
//! "epoch":…}`, `epoch` left out when that answer named none. A reconnect reads it to open the
//! inbox again ([`Outbox::due`]); `inbox close` removes the line before it sends the close and
//! puts it back when the close did not happen. It is changed under the same lock and replaced
//! the same way. Reading it fails no command that only reads it either: a missing or unreadable
//! file, a line of another form and an inbox named twice give the inboxes concerned no record,
//! so nothing is opened for them. A change of it that cannot read it fails as a change of the
//! epochs file does, and a missing file is read as empty.
//!
//! The record of the chat posts of an address beside it (`<outbox>.chat.<16 hex digits>`,
//! [`Outbox::record_chat_post`]) holds one JSON line per message `inbox bridge` posted into a
//! chat channel: its message id, the channel, the provider's reference of the post, the thread and
//! the `expires` of the message, its read mark, `posted`, then `read` or `due`
//! ([`Outbox::mark_chat_post`]), and its end, if it has one ([`Outbox::change_chat_post`], which
//! also drops a line). It is changed under the same lock and
//! replaced the same way, and a file that cannot be read, or holds a line that is no entry
//! ([`chat_posts_of`]), fails the change, and the read of [`Outbox::chat_posts`], instead of being
//! read as holding nothing or without that line.
//!
//! No read of these files blocks or fails a command that only reads them
//! ([`Outbox::read_or_nothing`]): each is opened without blocking, read only when it is a
//! regular file, and only up to a bound; anything else holds nothing, and one line on standard
//! error names it, but not in a run that failed to change that file: there the failure is the
//! one line that names it.

use std::cell::RefCell;
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

/// The record of an inbox its owner opened with `inbox open` on this host, as the file of the
/// opened inbox holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opened {
    /// The adapter it was last opened with, as the generated decoder reads one.
    pub adapter: String,
    /// The epoch of the answer that said the inbox is open; `None` when it named none.
    pub epoch: Option<Epoch>,
}

/// What a reconnect does for one caller at the epoch of a new start ([`Outbox::due`]).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Due {
    /// The record to open the caller's own inbox again from (its adapter): the file of the opened
    /// inbox holds it at another epoch, or at none.
    pub open: Option<Opened>,
    /// Whether a kept envelope of the caller is recorded at another epoch than the new start's,
    /// or marked unsent.
    pub repost: bool,
}

/// What [`Outbox::record_opened`] did with the record of an inbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recorded {
    /// The record is written.
    Written,
    /// A reconnect's open found no record: a `close` took it meanwhile. Nothing is written.
    Gone,
    /// A reconnect's open found another record than the one it started from: a typed `open`
    /// wrote it meanwhile. It is left as it is.
    Other,
}

/// The state of a `ChatPost` once its post is made and recorded.
pub const POSTED: &str = "posted";
/// The state of a `ChatPost` whose message the service marked read on a reaction (or that was
/// read meanwhile, or is gone).
pub const READ: &str = "read";
/// The state of a `ChatPost` whose message's deadline had passed at the read mark.
pub const DUE: &str = "due";
/// The end of a `ChatPost` whose message the person answered or refused.
pub const ANSWERED: &str = "answered";
/// The end of a `ChatPost` whose decision silence decided, once its line is written.
pub const DECIDED: &str = "decided";
/// The end of a `ChatPost` whose message expired unanswered, once its line is written.
pub const EXPIRED: &str = "expired";
/// The end of a `ChatPost` whose message is gone before its `expires`.
pub const GONE: &str = "gone";

/// One line of the record of the chat posts of `inbox bridge`: a `ChatPost`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatPost {
    /// The message the post shows.
    pub message_id: String,
    /// The chat channel the post was made in; `None` in a line of a build before the channel was
    /// recorded, which the bridge counts as of its bound channel.
    pub channel: Option<String>,
    /// The chat provider's reference of the post (its `ts`).
    pub reference: String,
    /// The read mark: [`POSTED`], [`READ`] or [`DUE`].
    pub state: String,
    /// The thread of the message; `None` in a line of a build before it was recorded.
    pub thread: Option<String>,
    /// The `expires` of the message when it was posted; `None` in a line of a build before it
    /// was recorded.
    pub expires: Option<String>,
    /// How the message ended: [`ANSWERED`], [`DECIDED`], [`EXPIRED`] or [`GONE`]; `None` while
    /// the bridge follows it.
    pub end: Option<String>,
    /// The reading of the bridge's clock at which the bridge first found the message of an entry
    /// without `expires` gone from the service: its retention counts from there. `None` for every
    /// other entry.
    pub gone_at: Option<String>,
}

/// The outbox file.
///
/// A run names a file beside it that it fails to change in one line, the failure: it keeps the
/// files a change failed on (`failed`), and a read of such a file in the same run says nothing.
/// The word of the read of [`Outbox::kept`] waits in `held` until [`Outbox::speak`], so that a
/// change failing after it in the same run (`repost`) drops it.
#[derive(Debug)]
pub struct Outbox {
    path: PathBuf,
    failed: RefCell<HashSet<PathBuf>>,
    held: RefCell<Vec<(PathBuf, String)>>,
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
        Ok(Self {
            path,
            failed: RefCell::default(),
            held: RefCell::default(),
        })
    }

    /// Writes the words held back by [`Outbox::kept`], each one line on standard error, but
    /// those of a file a change failed on in this run.
    pub fn speak(&self) {
        let failed = self.failed.borrow();
        for (path, line) in self.held.borrow_mut().drain(..) {
            if !failed.contains(&path) {
                crate::output::warn(&line);
            }
        }
    }

    /// The failure of a change of the file at `path`, which could not be read for `why`; the
    /// file is kept as one this run failed on.
    fn unreadable(&self, path: &Path, why: &Unread) -> Failure {
        self.failed.borrow_mut().insert(path.to_owned());
        Failure::Local(format!("{}: {why}", path.display()))
    }

    /// The text of the file at `path` for a read that never fails a command: [`read_bounded`],
    /// and nothing when it cannot be read, with the word of [`held_nothing`] on standard error,
    /// but none on a file a change failed on in this run (its failure names it).
    fn read_or_nothing(&self, path: &Path, most: u64) -> String {
        match read_bounded(path, most) {
            Ok(text) => text.unwrap_or_default(),
            Err(why) => {
                if let Some(line) = held_nothing(path, &why)
                    && !self.failed.borrow().contains(path)
                {
                    crate::output::warn(&line);
                }
                String::new()
            }
        }
    }

    /// The path of the file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Keeps `text`, the envelope as it is about to be posted, marked unsent in the epochs file
    /// in the same change (so a post killed on its way is due for a later command,
    /// [`Outbox::due`]), and drops what expired at `now`. The answer to the post settles the mark
    /// ([`Outbox::record_epoch`], [`Outbox::clear_unsent`], [`Outbox::remove`]). When only the
    /// epochs file cannot be written, the envelope is kept without its mark and one line on
    /// standard error says so: the post goes on.
    ///
    /// # Errors
    ///
    /// [`Failure::Local`] when the outbox file cannot be read or written, or the epochs file
    /// cannot be read: then nothing is written and the envelope is not kept.
    pub fn keep(&self, text: &str, now: Instant) -> Result<(), Failure> {
        let unmarked = self.change_both(now, true, |lines, epochs| {
            lines.push(text.to_owned());
            if let Some(envelope) = decode(text) {
                epochs.mark_unsent(&envelope.message_id.0);
            }
        })?;
        if let Some(cause) = unmarked {
            crate::output::warn(&format!(
                "inbox: the envelope is kept without its mark unsent: {}",
                cause.message()
            ));
        }
        Ok(())
    }

    /// Removes the mark unsent of the envelope with this message id and records no epoch: the
    /// service answered its post for another reason than the two that keep the mark (a refusal
    /// of its own such as `413`, or an arrival whose answer named no epoch). Drops what expired at
    /// `now`.
    ///
    /// # Errors
    ///
    /// [`Failure::Local`] when a file cannot be read or written.
    pub fn clear_unsent(&self, message_id: &str, now: Instant) -> Result<(), Failure> {
        self.change(now, true, |_, epochs| epochs.clear_unsent(message_id))
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
        self.change(now, true, |lines, epochs| {
            if lines.iter().any(|line| {
                decode(line).is_some_and(|envelope| envelope.message_id.0 == message_id)
            }) {
                epochs.record(message_id, epoch);
            }
        })
    }

    /// Marks the envelope with this message id unsent in the epochs file (`<message id>\tnone`)
    /// when the outbox file still keeps it and no epoch is recorded for it: its post failed in
    /// transport or was refused `UnknownInbox`, so the reconnect of a later command posts it
    /// again ([`Outbox::due`]). Drops what expired at `now`.
    ///
    /// # Errors
    ///
    /// [`Failure::Local`] when a file cannot be read or written.
    pub fn mark_unsent(&self, message_id: &str, now: Instant) -> Result<(), Failure> {
        self.change(now, true, |lines, epochs| {
            if lines.iter().any(|line| {
                decode(line).is_some_and(|envelope| envelope.message_id.0 == message_id)
            }) {
                epochs.mark_unsent(message_id);
            }
        })
    }

    /// The envelopes kept for `from` that have not expired at `now`, in the order they were
    /// posted, each with the epoch the epochs file records for it, and every line of the file
    /// that does not read as an envelope; drops what expired. The word on an epochs file it
    /// reads as holding nothing waits for [`Outbox::speak`].
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

    /// What a reconnect does for the caller `principal` (whose own inbox has the same text)
    /// when an answer names the epoch `current`: open its inbox again when the file of the
    /// opened inbox records it at another epoch or at none, and post its kept envelopes again
    /// when one of them, unexpired at `now`, is recorded at another epoch or marked unsent (kept
    /// and not answered yet, or its post failed in transport or was refused `UnknownInbox`),
    /// except the envelope `own` the command itself just posted. An envelope the epochs file
    /// says nothing of starts none: a version before the mark kept it, or the service refused it
    /// for a reason of its own (`413`), or answered `duplicate-key`. Read without the lock (each
    /// file is replaced whole, so a reader sees one version of it) and never fails nor blocks
    /// ([`Outbox::read_or_nothing`]). The steps themselves read under the lock again.
    pub fn due(&self, principal: &str, current: &Epoch, now: Instant, own: Option<&str>) -> Due {
        let open = Records::parse(
            &self.read_or_nothing(&beside(&self.path, ".opened"), MOST_OPENED_BYTES),
        )
        .of(principal)
        .filter(|record| record.epoch.as_ref() != Some(current))
        .cloned();
        let text = self.read_or_nothing(&self.path, MOST_BYTES);
        let lines: Vec<String> = text.lines().map(str::to_owned).collect();
        let kept: Vec<Kept> = of_sender(&unexpired(&lines, now), principal)
            .into_iter()
            .filter(|kept| Some(kept.envelope.message_id.0.as_str()) != own)
            .collect();
        let repost = !kept.is_empty() && {
            let epochs_text = self.read_or_nothing(&beside(&self.path, ".epochs"), MOST_BYTES);
            let epochs = Epochs::parse(epoch_lines(&epochs_text));
            kept.iter().any(|kept| {
                let id = &kept.envelope.message_id.0;
                match epochs.of(id) {
                    Some(epoch) => &epoch != current,
                    None => epochs.is_unsent(id),
                }
            })
        };
        Due { open, repost }
    }

    /// The record of the inbox `inbox` in the file of the opened inbox, if it holds one; read as
    /// [`Outbox::due`] reads it.
    pub fn opened(&self, inbox: &str) -> Option<Opened> {
        Records::parse(&self.read_or_nothing(&beside(&self.path, ".opened"), MOST_OPENED_BYTES))
            .of(inbox)
            .cloned()
    }

    /// Removes the record of `inbox` from the file of the opened inbox and hands it back: its
    /// owner is about to close the inbox ([`Outbox::put_back_opened`] when the close did not
    /// happen).
    ///
    /// # Errors
    ///
    /// [`Failure::Local`] when the file cannot be read (it is left as it is) or written.
    pub fn take_opened(&self, inbox: &str) -> Result<Option<Opened>, Failure> {
        let mut taken = None;
        self.change_opened(|records| {
            taken = records.of(inbox).cloned();
            records.remove(inbox);
        })?;
        Ok(taken)
    }

    /// Puts `record` back as the record of `inbox`, unless the file holds one for it again (an
    /// `inbox open` since).
    ///
    /// # Errors
    ///
    /// [`Failure::Local`] when the file cannot be read (it is left as it is) or written.
    pub fn put_back_opened(&self, inbox: &str, record: Opened) -> Result<(), Failure> {
        self.change_opened(|records| {
            if records.of(inbox).is_none() {
                records.set(inbox, record);
            }
        })
    }

    /// Records in the file of the opened inbox that `inbox` was opened with `adapter` by an
    /// answer that named `epoch`, in place of the record it had. With `from` (a reconnect, the
    /// record it started from) the record is replaced only while it is still that record (same
    /// adapter, same epoch): a `close` that removed it while the reconnect's open was on its way
    /// is not undone ([`Recorded::Gone`]), and the record of a typed `open` since is left as it
    /// is ([`Recorded::Other`]).
    ///
    /// # Errors
    ///
    /// [`Failure::Local`] when the file cannot be read (it is left as it is) or written.
    pub fn record_opened(
        &self,
        inbox: &str,
        adapter: &str,
        epoch: Option<&Epoch>,
        from: Option<&Opened>,
    ) -> Result<Recorded, Failure> {
        let mut recorded = Recorded::Written;
        self.change_opened(|records| {
            if let Some(from) = from {
                match records.of(inbox) {
                    None => {
                        recorded = Recorded::Gone;
                        return;
                    }
                    Some(held) if held != from => {
                        recorded = Recorded::Other;
                        return;
                    }
                    Some(_) => {}
                }
            }
            records.set(
                inbox,
                Opened {
                    adapter: adapter.to_owned(),
                    epoch: epoch.cloned(),
                },
            );
        })?;
        Ok(recorded)
    }

    /// Records the `ChatPost` `post` in the record of the chat posts of `address` beside the
    /// outbox file, `<outbox>.chat.<16 hex digits>` (the digits of the lock of the wake,
    /// [`crate::wake::digits`]). One JSON line per message id,
    /// `{"message_id","channel","reference","state","thread","expires","end"}`, in place of the
    /// line that id had; the other lines are kept as they are. Changed under the lock of the
    /// outbox file and replaced whole, as the other files beside it (mode 0600).
    ///
    /// # Errors
    ///
    /// [`Failure::Local`] when the file cannot be read (also when it is no regular file or over
    /// its bound: its lines are never given up for a new one) or written.
    pub fn record_chat_post(&self, address: &str, post: &ChatPost) -> Result<(), Failure> {
        self.change_chat_posts(address, |lines| {
            lines.retain(|line| chat_post_id(line).as_deref() != Some(post.message_id.as_str()));
            lines.push(chat_line(post));
            true
        })
        .map(drop)
    }

    /// Replaces the line of the record of `address` that is `post`, as [`Outbox::chat_posts`]
    /// read it, with `to`, or removes it when `to` is `None`, under the lock of the outbox file.
    /// A line that changed meanwhile is left as it is, and so is the record when no line is
    /// `post`. Answers whether a line changed.
    ///
    /// # Errors
    ///
    /// [`Failure::Local`] when the record cannot be read or written.
    pub fn change_chat_post(
        &self,
        address: &str,
        post: &ChatPost,
        to: Option<&ChatPost>,
    ) -> Result<bool, Failure> {
        self.change_chat_posts(address, |lines| {
            let mut changed = false;
            lines.retain_mut(|line| {
                if chat_post(line).as_ref() != Some(post) {
                    return true;
                }
                changed = true;
                match to {
                    Some(to) => {
                        *line = chat_line(to);
                        true
                    }
                    None => false,
                }
            });
            changed
        })
    }

    /// The `ChatPost`s of the record of the chat posts of `address`, in the order of its lines
    /// ([`chat_posts_of`]). A missing record holds none.
    ///
    /// # Errors
    ///
    /// [`Failure::Local`] naming the record when it cannot be read: no regular file, over its
    /// bound, not readable, or a line that is no entry.
    pub fn chat_posts(&self, address: &str) -> Result<Vec<ChatPost>, Failure> {
        let path = self.chat_path(address);
        let local =
            |why: &dyn std::fmt::Display| Failure::Local(format!("{}: {why}", path.display()));
        let text = read_bounded(&path, MOST_BYTES)
            .map_err(|why| local(&why))?
            .unwrap_or_default();
        chat_posts_of(&text).map_err(|why| local(&why))
    }

    /// Gives the line of the record of `address` that is `post`, as [`Outbox::chat_posts`] read
    /// it, the state `state` (`read` or `due`), under the lock of the outbox file. A line that
    /// changed meanwhile (the message posted again, with another reference) is left as it is, and
    /// so is the record when no line is `post`. Answers whether a line changed.
    ///
    /// # Errors
    ///
    /// [`Failure::Local`] when the record cannot be read or written.
    pub fn mark_chat_post(
        &self,
        address: &str,
        post: &ChatPost,
        state: &str,
    ) -> Result<bool, Failure> {
        self.change_chat_post(
            address,
            post,
            Some(&ChatPost {
                state: state.to_owned(),
                ..post.clone()
            }),
        )
    }

    /// The record of the chat posts of `address` beside the outbox file.
    fn chat_path(&self, address: &str) -> PathBuf {
        beside(
            &self.path,
            &format!(".chat.{}", crate::wake::digits(address)),
        )
    }

    /// Runs `change` over the lines of the record of the chat posts of `address` under the lock,
    /// and replaces the record when it answers that it changed them. A record that cannot be read
    /// ([`Outbox::chat_posts`], a line that is no entry included) fails it before `change` runs,
    /// and nothing is written, so no change drops a line it could not read; a missing one is read
    /// as empty.
    fn change_chat_posts(
        &self,
        address: &str,
        change: impl FnOnce(&mut Vec<String>) -> bool,
    ) -> Result<bool, Failure> {
        let path = self.chat_path(address);
        let local =
            |why: &dyn std::fmt::Display| Failure::Local(format!("{}: {why}", path.display()));
        let lock = self.lock()?;
        let text = read_bounded(&path, MOST_BYTES)
            .map_err(|why| local(&why))?
            .unwrap_or_default();
        chat_posts_of(&text).map_err(|why| local(&why))?;
        let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
        let changed = change(&mut lines);
        if changed {
            replace(&path, &lines).map_err(|error| local(&error))?;
        }
        lock.unlock().map_err(|error| local(&error))?;
        Ok(changed)
    }

    /// Runs `change` over the records of the file of the opened inbox under the lock, and
    /// replaces the file when a record changed. A file that cannot be read fails it before
    /// `change` runs, and nothing is written; a missing one is read as empty.
    fn change_opened(&self, change: impl FnOnce(&mut Records)) -> Result<(), Failure> {
        let path = beside(&self.path, ".opened");
        let local = |error: std::io::Error| Failure::Local(format!("{}: {error}", path.display()));
        let lock = self.lock()?;
        let text = match read_bounded(&path, MOST_OPENED_BYTES) {
            Ok(text) => text.unwrap_or_default(),
            Err(why) => return Err(self.unreadable(&path, &why)),
        };
        let mut records = Records::parse(&text);
        change(&mut records);
        if records.changed {
            replace(&path, &records.lines()).map_err(local)?;
        }
        lock.unlock().map_err(local)
    }

    /// The lock of the outbox file and the files beside it, taken: `<outbox>.lock`, created (and
    /// its directory, mode 0700) when absent.
    fn lock(&self) -> Result<File, Failure> {
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
        Ok(lock)
    }

    /// Runs `change` over the lines of the file and, when `with_epochs`, what the epochs file
    /// says, under the lock, after dropping every envelope whose `expires` is at or before `now`;
    /// writes the epochs file again when `change` changed it, then the file when its lines
    /// changed (also when the epochs file could not be written). Without `with_epochs` the
    /// epochs file is neither read nor written. With it, an epochs file that cannot be read is
    /// read as holding nothing when `change` asks no change of it, and fails the change before
    /// any write when it asks one. A line that does not read as an envelope is kept as it is.
    fn change(
        &self,
        now: Instant,
        with_epochs: bool,
        change: impl FnOnce(&mut Vec<String>, &mut Epochs),
    ) -> Result<(), Failure> {
        match self.change_both(now, with_epochs, change)? {
            None => Ok(()),
            Some(epochs_failed) => Err(epochs_failed),
        }
    }

    /// [`Outbox::change`], answering apart the failure to write the epochs file when the outbox
    /// file was written after it (`Ok(Some(..))`), for [`Outbox::keep`]: its envelope is kept also
    /// when its mark cannot be written.
    fn change_both(
        &self,
        now: Instant,
        with_epochs: bool,
        change: impl FnOnce(&mut Vec<String>, &mut Epochs),
    ) -> Result<Option<Failure>, Failure> {
        let shown = self.path.display();
        let local = |error: std::io::Error| Failure::Local(format!("{shown}: {error}"));
        let lock = self.lock()?;
        let text = match read_bounded(&self.path, MOST_BYTES) {
            Ok(text) => text.unwrap_or_default(),
            Err(why) => return Err(Failure::Local(format!("{shown}: {why}"))),
        };
        let before: Vec<String> = text.lines().map(str::to_owned).collect();
        let mut lines = unexpired(&before, now);
        let epochs_path = beside(&self.path, ".epochs");
        let (mut epochs, unread) = if with_epochs {
            match read_bounded(&epochs_path, MOST_BYTES) {
                Ok(text) => (Epochs::parse(epoch_lines(&text.unwrap_or_default())), None),
                Err(why) => (Epochs::unread(), Some(why)),
            }
        } else {
            (Epochs::default(), None)
        };
        change(&mut lines, &mut epochs);
        if let Some(why) = unread {
            if epochs.changed {
                return Err(self.unreadable(&epochs_path, &why));
            }
            if let Some(line) = held_nothing(&epochs_path, &why) {
                self.held.borrow_mut().push((epochs_path.clone(), line));
            }
        }
        let epochs_failed = if epochs.changed {
            replace(&epochs_path, &epochs.lines(&lines))
                .err()
                .map(|error| Failure::Local(format!("{}: {error}", epochs_path.display())))
        } else {
            None
        };
        if lines != before {
            replace(&self.path, &lines).map_err(local)?;
        }
        lock.unlock().map_err(local)?;
        Ok(epochs_failed)
    }
}

/// The most bytes a read takes of the outbox file and of the epochs file: 64 MiB, room for
/// some 160 envelopes of the longest body or tens of thousands of short ones. A file over it is
/// not read in part: the outbox file is a local failure, and so is the epochs file to a change
/// of it; to a read it holds nothing.
const MOST_BYTES: u64 = 64 * 1024 * 1024;
/// The most bytes a read takes of the file of the opened inbox: a line per inbox.
const MOST_OPENED_BYTES: u64 = 1024 * 1024;

/// Why a file beside the outbox file (or the outbox file) was not read.
#[derive(Debug)]
enum Unread {
    /// The open or the read failed: the error of the OS.
    Io(std::io::Error),
    /// It is no regular file: a FIFO, a device, a directory, or a link to one.
    NotRegular,
    /// It holds more than the bound.
    OverBound(u64),
    /// Its bytes are not UTF-8.
    NotUtf8,
}

impl std::fmt::Display for Unread {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(out, "{error}"),
            Self::NotRegular => write!(out, "it is no regular file"),
            Self::OverBound(most) => write!(out, "it holds more than {most} bytes"),
            Self::NotUtf8 => write!(out, "stream did not contain valid UTF-8"),
        }
    }
}

/// The text of the file at `path`, or `None` when there is none, read as `src/clock_file.rs`
/// reads the clock file: opened with `O_NONBLOCK` (a FIFO without a writer does not hold the
/// open), a regular file only (a link is followed, so a link to a device is refused before a
/// byte is read), at most `most` bytes.
fn read_bounded(path: &Path, most: u64) -> Result<Option<String>, Unread> {
    use std::io::Read;
    let file = match OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(Unread::Io(error)),
    };
    if !file.metadata().map_err(Unread::Io)?.is_file() {
        return Err(Unread::NotRegular);
    }
    let mut bytes = Vec::new();
    file.take(most + 1)
        .read_to_end(&mut bytes)
        .map_err(Unread::Io)?;
    if bytes.len() as u64 > most {
        return Err(Unread::OverBound(most));
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| Unread::NotUtf8)
}

/// The word on the file at `path` that was not read for `why` and is read as holding nothing:
/// one line for a file that is no regular file or holds more than its bound; none for a
/// missing file, one its owner may not read and one that is not UTF-8, as before.
fn held_nothing(path: &Path, why: &Unread) -> Option<String> {
    matches!(why, Unread::NotRegular | Unread::OverBound(_))
        .then(|| format!("inbox: {}: {why}; read as holding nothing", path.display()))
}

/// Replaces the file at `path` with `lines`, each ended by a newline: written to `<path>.new`
/// and renamed over it. What is at `<path>.new` already (a file an interrupted run left, a link,
/// a FIFO) is removed, not followed nor opened, and the staged file is created anew with mode
/// 0600 (`O_CREAT | O_EXCL`, which follows no link); a directory there is a failure. Every
/// change of the outbox file, the epochs file and the file of the opened inbox writes through
/// here.
fn replace(path: &Path, lines: &[String]) -> std::io::Result<()> {
    let staged = beside(path, ".new");
    if let Err(error) = fs::remove_file(&staged)
        && error.kind() != ErrorKind::NotFound
    {
        return Err(error);
    }
    let mut file: File = OpenOptions::new()
        .write(true)
        .create_new(true)
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

#[cfg(test)]
thread_local! {
    /// The steps [`Epochs`] took on this thread, read by the unit case that holds a read of the
    /// epochs file to steps linear in its lines (story:no-test-holds-a-wall-clock-time: a test
    /// of a read's cost counts work, it holds no time).
    static STEPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// The most steps the case allows before a step panics, so a read in pairs fails at once
    /// instead of running for seconds.
    static MOST: std::cell::Cell<usize> = const { std::cell::Cell::new(usize::MAX) };
}

/// One step of [`Epochs`]: a line of the file split, a message id of it compared or read as text
/// ([`LineId`]), an epoch asked for or recorded, a line written. Counted in a test build of this
/// crate only, where a step past `MOST` panics; in every other build it does nothing.
fn step() {
    #[cfg(test)]
    STEPS.with(|steps| {
        let taken = steps.get() + 1;
        steps.set(taken);
        let most = MOST.with(std::cell::Cell::get);
        assert!(
            taken <= most,
            "the read of the epochs file took more than {most} steps"
        );
    });
}

/// One line of the epochs file's text, split at its first tab: the message id and what follows
/// it (the epoch's digits when the line is of its form).
struct EpochLine<'a> {
    id: LineId<'a>,
    digits: &'a str,
}

/// The message id of a line of the epochs file, as [`Epochs::parse`] gets it. Each comparison of
/// two `LineId` values and each read of its text ([`LineId::text`]) is a [`step`], so a read that
/// compares every `LineId` with every other is stopped at the bound. Not counted: a hash (a hash
/// map rehashes its entries as it grows, and compares only the ids whose hashes meet), and a
/// comparison of the texts themselves once taken through `text()`, so a read in pairs written
/// over the texts passes the step case.
#[derive(Clone, Copy)]
struct LineId<'a>(&'a str);

impl<'a> LineId<'a> {
    /// The id's text.
    fn text(self) -> &'a str {
        step();
        self.0
    }
}

impl PartialEq for LineId<'_> {
    fn eq(&self, other: &Self) -> bool {
        step();
        self.0 == other.0
    }
}

impl Eq for LineId<'_> {}

impl std::hash::Hash for LineId<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// The lines of the epochs file's text, each a [`step`].
fn epoch_lines(text: &str) -> impl Iterator<Item = EpochLine<'_>> {
    text.lines().map(|line| {
        step();
        let (id, digits) = line.split_once('\t').unwrap_or((line, ""));
        EpochLine {
            id: LineId(id),
            digits,
        }
    })
}

/// The text after the tab of a line of the epochs file that marks its envelope unsent.
const UNSENT: &str = "none";

/// What the epochs file says: the epoch of each message id one line of the form
/// `<message id>\t<32 lower-case hex digits>` names, and the message ids one line
/// `<message id>\tnone` marks unsent. A message id that two lines name, whatever their form, has
/// neither. Of a file that could not be read ([`Epochs::unread`]) it says nothing, and every
/// change asked of it counts as a change, so that [`Outbox::change_both`] fails it.
#[derive(Debug, Default)]
struct Epochs {
    recorded: HashMap<String, Epoch>,
    unsent: HashSet<String>,
    changed: bool,
    unread: bool,
}

impl Epochs {
    /// What a file that could not be read says: nothing.
    fn unread() -> Self {
        Self {
            unread: true,
            ..Self::default()
        }
    }

    /// Reads the file's lines ([`epoch_lines`]) in one pass.
    fn parse<'a>(lines: impl Iterator<Item = EpochLine<'a>>) -> Self {
        let mut named: HashMap<LineId<'a>, (usize, &'a str)> = HashMap::new();
        for line in lines {
            let entry = named.entry(line.id).or_insert((0, line.digits));
            entry.0 += 1;
        }
        let mut recorded = HashMap::new();
        let mut unsent = HashSet::new();
        for (id, (count, digits)) in named {
            if count != 1 {
                continue;
            }
            let id = id.text();
            if id.is_empty() {
                continue;
            }
            if digits == UNSENT {
                unsent.insert(id.to_owned());
            } else if let Some(epoch) = Epoch::parse(digits) {
                recorded.insert(id.to_owned(), epoch);
            }
        }
        Self {
            recorded,
            unsent,
            changed: false,
            unread: false,
        }
    }

    fn of(&self, message_id: &str) -> Option<Epoch> {
        step();
        self.recorded.get(message_id).cloned()
    }

    /// Whether a line marks the envelope unsent.
    fn is_unsent(&self, message_id: &str) -> bool {
        step();
        self.unsent.contains(message_id)
    }

    fn record(&mut self, message_id: &str, epoch: &Epoch) {
        step();
        self.unsent.remove(message_id);
        self.recorded.insert(message_id.to_owned(), epoch.clone());
        self.changed = true;
    }

    fn clear_unsent(&mut self, message_id: &str) {
        step();
        if self.unsent.remove(message_id) || self.unread {
            self.changed = true;
        }
    }

    /// Marks the envelope unsent, unless an epoch is recorded for it: that start held it, and a
    /// new start is told by the epoch.
    fn mark_unsent(&mut self, message_id: &str) {
        step();
        if (!self.recorded.contains_key(message_id) && self.unsent.insert(message_id.to_owned()))
            || self.unread
        {
            self.changed = true;
        }
    }

    /// The lines of the file, in the order of the envelopes `kept` holds, for those and no other.
    fn lines(&self, kept: &[String]) -> Vec<String> {
        let mut written = HashSet::new();
        kept.iter()
            .filter_map(|line| decode(line).map(|envelope| envelope.message_id.0))
            .filter_map(|id| {
                step();
                let value = match self.recorded.get(&id) {
                    Some(epoch) => epoch.as_str(),
                    None if self.unsent.contains(&id) => UNSENT,
                    None => return None,
                };
                written.insert(id.clone()).then(|| format!("{id}\t{value}"))
            })
            .collect()
    }
}

/// What the file of the opened inbox says: the record of each inbox one line of the form
/// `{"inbox":…,"adapter":…,"epoch":…}` names (`epoch` may be left out; no other member, none
/// twice, the adapter one the generated decoder reads, the epoch 32 lower-case hex digits). An
/// inbox that two such lines name has none. Written again, the file holds the records in the
/// order read, the one set last at the end, and no line of another form.
#[derive(Debug, Default)]
struct Records {
    held: Vec<(String, Opened)>,
    changed: bool,
}

impl Records {
    fn parse(text: &str) -> Self {
        let mut read: Vec<(String, Opened)> = Vec::new();
        let mut named: HashMap<String, usize> = HashMap::new();
        for (inbox, record) in text.lines().filter_map(record) {
            *named.entry(inbox.clone()).or_default() += 1;
            read.push((inbox, record));
        }
        Self {
            held: read
                .into_iter()
                .filter(|(inbox, _)| named.get(inbox) == Some(&1))
                .collect(),
            changed: false,
        }
    }

    fn of(&self, inbox: &str) -> Option<&Opened> {
        self.held
            .iter()
            .find(|(held, _)| held == inbox)
            .map(|(_, record)| record)
    }

    fn set(&mut self, inbox: &str, record: Opened) {
        self.held.retain(|(held, _)| held != inbox);
        self.held.push((inbox.to_owned(), record));
        self.changed = true;
    }

    fn remove(&mut self, inbox: &str) {
        let before = self.held.len();
        self.held.retain(|(held, _)| held != inbox);
        self.changed |= self.held.len() != before;
    }

    fn lines(&self) -> Vec<String> {
        self.held
            .iter()
            .map(|(inbox, record)| {
                let mut line = String::from("{");
                json::member(&mut line, "inbox");
                json::push_text(&mut line, inbox);
                json::member(&mut line, "adapter");
                json::push_text(&mut line, &record.adapter);
                if let Some(epoch) = &record.epoch {
                    json::member(&mut line, "epoch");
                    json::push_text(&mut line, epoch.as_str());
                }
                line.push('}');
                line
            })
            .collect()
    }
}

/// The inbox and the record one line of the file of the opened inbox holds, when it is of the
/// form [`Records`] reads.
fn record(line: &str) -> Option<(String, Opened)> {
    let json::Value::Object(members) = json::parse(line).ok()? else {
        return None;
    };
    let text = |name: &str| {
        let mut found = members.iter().filter(|(held, _)| held == name);
        match (found.next(), found.next()) {
            (Some((_, json::Value::Text(text))), None) => Some(Some(text.clone())),
            (None, None) => Some(None),
            _ => None,
        }
    };
    if members
        .iter()
        .any(|(name, _)| !["inbox", "adapter", "epoch"].contains(&name.as_str()))
    {
        return None;
    }
    let inbox = text("inbox")?.filter(|inbox| !inbox.is_empty())?;
    let adapter = text("adapter")??;
    wire::decode_inbox_messaging_adapter(&json::Value::Text(adapter.clone()), "adapter").ok()?;
    let epoch = match text("epoch")? {
        Some(digits) => Some(Epoch::parse(&digits)?),
        None => None,
    };
    Some((inbox, Opened { adapter, epoch }))
}

/// The message id a line of the record of the chat posts names, when it is a JSON object with
/// the text member `message_id`.
fn chat_post_id(line: &str) -> Option<String> {
    match json::parse(line).ok()?.member("message_id") {
        Some(json::Value::Text(message_id)) => Some(message_id.clone()),
        _ => None,
    }
}

/// The entries of the text of a record of the chat posts, one per line. The bridge writes the
/// record whole, each line with its line end, and renames it over the old one, so it never leaves
/// a line cut short or an empty line: a line that is no entry ([`chat_post`]), an empty one
/// included, makes the whole record one that cannot be read, so the entry it held is never taken
/// as missing (its message would be posted a second time). A last line without its line end is
/// read as every other line: whole when it is an entry. A line of an earlier build, without
/// members written later, is an entry.
///
/// # Errors
///
/// The number of the first line that is no entry.
fn chat_posts_of(text: &str) -> Result<Vec<ChatPost>, String> {
    text.lines()
        .enumerate()
        .map(|(at, line)| {
            chat_post(line).ok_or_else(|| format!("line {} is no entry of the record", at + 1))
        })
        .collect()
}

/// The `ChatPost` a line of the record of the chat posts holds, when it is a JSON object with the
/// text members `message_id`, `reference` and `state`, and the text members `channel`, `thread`,
/// `expires`, `end` and `gone_at` when the line has them (a line of a build before them has not).
fn chat_post(line: &str) -> Option<ChatPost> {
    let value = json::parse(line).ok()?;
    let text = |name: &str| match value.member(name) {
        Some(json::Value::Text(text)) => Some(text.clone()),
        _ => None,
    };
    Some(ChatPost {
        message_id: text("message_id")?,
        channel: text("channel"),
        reference: text("reference")?,
        state: text("state")?,
        thread: text("thread"),
        expires: text("expires"),
        end: text("end"),
        gone_at: text("gone_at"),
    })
}

/// The line of the record of the chat posts that holds `post`:
/// `{"message_id":…,"channel":…,"reference":…,"state":…,"thread":…,"expires":…,"end":…,
/// "gone_at":…}`, each of `channel`, `thread`, `expires`, `end` and `gone_at` left out when the
/// post has none.
fn chat_line(post: &ChatPost) -> String {
    let mut line = String::from("{");
    json::member(&mut line, "message_id");
    json::push_text(&mut line, &post.message_id);
    if let Some(channel) = &post.channel {
        json::member(&mut line, "channel");
        json::push_text(&mut line, channel);
    }
    json::member(&mut line, "reference");
    json::push_text(&mut line, &post.reference);
    json::member(&mut line, "state");
    json::push_text(&mut line, &post.state);
    for (name, value) in [
        ("thread", &post.thread),
        ("expires", &post.expires),
        ("end", &post.end),
        ("gone_at", &post.gone_at),
    ] {
        if let Some(value) = value {
            json::member(&mut line, name);
            json::push_text(&mut line, value);
        }
    }
    line.push('}');
    line
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

    use super::{Epochs, chat_posts_of, epoch_lines, of_sender, unexpired, unreadable, without};
    use crate::client::Epoch;

    /// A record of the chat posts reads only whole: a line cut short, an empty line or a line
    /// that is no object with the 3 text members makes it one that cannot be read, naming the
    /// line; a line of an earlier build (no `channel`, `thread`, `expires`, `end`) reads, and so
    /// does a last line without its line end. A record with no line holds no entry.
    #[test]
    fn a_record_of_the_chat_posts_with_a_line_that_is_no_entry_cannot_be_read() {
        let whole = r#"{"message_id":"M1","channel":"C0000000001","reference":"1700000000.000001","state":"posted","thread":"T1","expires":"2026-11-24T12:00:00Z"}"#;
        let older = r#"{"message_id":"M2","reference":"1700000000.000002","state":"read"}"#;
        let read = chat_posts_of(&format!("{whole}\n{older}\n")).expect("whole lines read");
        assert_eq!(
            read.iter()
                .map(|post| post.message_id.as_str())
                .collect::<Vec<_>>(),
            ["M1", "M2"]
        );
        assert_eq!(read[1].channel, None);
        assert_eq!(read[1].expires, None);
        let without_end = chat_posts_of(&format!("{whole}\n{older}")).expect("a last line reads");
        assert_eq!(without_end, read);
        assert_eq!(chat_posts_of(""), Ok(Vec::new()));
        let cut = &older[..older.len() - 2];
        assert_eq!(
            chat_posts_of(&format!("{whole}\n{cut}\n")),
            Err("line 2 is no entry of the record".to_owned())
        );
        assert_eq!(
            chat_posts_of(&format!("{whole}\n\n{older}\n")),
            Err("line 2 is no entry of the record".to_owned())
        );
        assert_eq!(
            chat_posts_of(r#"{"message_id":"M3","state":"posted"}"#),
            Err("line 1 is no entry of the record".to_owned())
        );
    }

    /// A record with a line that is no entry: the read names the record and the line, and no
    /// change (a new entry, a changed one, a read mark, a dropped one) writes it, so the line it
    /// could not read is never dropped.
    #[test]
    fn a_record_of_the_chat_posts_with_a_line_that_is_no_entry_is_never_rewritten() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/tmp/chat-record-cut")
            .join(std::process::id().to_string());
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the directory is made");
        let outbox = super::Outbox::locate(Some(&dir.join("outbox.jsonl"))).expect("the outbox");
        let address = "agent:test/bob";
        let first = super::ChatPost {
            message_id: "M1".to_owned(),
            channel: Some("C0000000001".to_owned()),
            reference: "1700000000.000001".to_owned(),
            state: super::POSTED.to_owned(),
            thread: Some("M1".to_owned()),
            expires: Some("2099-01-01T00:00:00Z".to_owned()),
            end: None,
            gone_at: None,
        };
        outbox.record_chat_post(address, &first).expect("recorded");
        let path = outbox.chat_path(address);
        let whole = std::fs::read_to_string(&path).expect("the record");
        let cut = r#"{"message_id":"M2","reference":"1700000000.000002","state":"pos"#;
        let text = format!("{whole}{cut}\n");
        std::fs::write(&path, &text).expect("a line cut short");
        let failed = outbox
            .chat_posts(address)
            .expect_err("a record that cannot be read");
        assert!(
            failed.message().contains(&path.display().to_string())
                && failed
                    .message()
                    .contains("line 2 is no entry of the record"),
            "{}",
            failed.message()
        );
        let second = super::ChatPost {
            message_id: "M3".to_owned(),
            ..first.clone()
        };
        assert!(outbox.record_chat_post(address, &second).is_err());
        assert!(outbox.mark_chat_post(address, &first, super::READ).is_err());
        assert!(outbox.change_chat_post(address, &first, None).is_err());
        assert_eq!(std::fs::read_to_string(&path).expect("the record"), text);
        let _ = std::fs::remove_dir_all(&dir);
    }

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
        let epochs = Epochs::parse(epoch_lines(&text));
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
        let epochs = Epochs::parse(epoch_lines(&text));
        for n in 1..=5 {
            assert_eq!(epochs.of(&id(n)), None, "{n}");
        }
        assert_eq!(epochs.of(""), None);
        assert_eq!(epochs.of(&id(6)), Some(epoch(EPOCH)));
        assert_eq!(Epochs::parse(epoch_lines("")).of(&id(6)), None);
    }

    #[test]
    fn a_recorded_epoch_replaces_the_one_before_and_ids_named_twice() {
        let text = format!(
            "{}\t{EPOCH}\n{}\t{EPOCH}\n{}\t{OTHER}\n",
            id(1),
            id(2),
            id(2)
        );
        let mut epochs = Epochs::parse(epoch_lines(&text));
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
        let mut epochs = Epochs::parse(epoch_lines(&text));
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

    #[test]
    fn an_opened_line_names_the_inbox_the_adapter_and_the_epoch_and_is_written_back_the_same() {
        let text = format!(
            "{{\"inbox\":\"agent:test/bob\",\"adapter\":\"slack\",\"epoch\":\"{EPOCH}\"}}\n\
             {{\"inbox\":\"agent:test/carol\",\"adapter\":\"terminal\"}}\n"
        );
        let records = super::Records::parse(&text);
        assert_eq!(
            records.of("agent:test/bob"),
            Some(&super::Opened {
                adapter: "slack".to_owned(),
                epoch: Some(epoch(EPOCH)),
            })
        );
        assert_eq!(
            records
                .of("agent:test/carol")
                .map(|record| record.epoch.clone()),
            Some(None)
        );
        assert_eq!(records.of("agent:test/alice"), None);
        assert_eq!(records.lines().join("\n") + "\n", text);
    }

    #[test]
    fn an_opened_line_of_another_form_or_an_inbox_named_twice_gives_no_record() {
        let wrong = [
            "not json".to_owned(),
            r#"["agent:test/a","slack"]"#.to_owned(),
            r#"{"inbox":"agent:test/a","adapter":"pager"}"#.to_owned(),
            format!(
                r#"{{"inbox":"agent:test/a","adapter":"slack","epoch":"{}"}}"#,
                EPOCH.to_uppercase()
            ),
            r#"{"inbox":"agent:test/a","adapter":"slack","other":1}"#.to_owned(),
            r#"{"inbox":"agent:test/a","adapter":"slack","adapter":"terminal"}"#.to_owned(),
            r#"{"inbox":"","adapter":"slack"}"#.to_owned(),
            r#"{"adapter":"slack"}"#.to_owned(),
            r#"{"inbox":"agent:test/a","adapter":7}"#.to_owned(),
        ];
        for line in &wrong {
            assert!(super::record(line).is_none(), "{line}");
        }
        let twice = "{\"inbox\":\"agent:test/b\",\"adapter\":\"slack\"}\n\
                     {\"inbox\":\"agent:test/b\",\"adapter\":\"terminal\"}\n\
                     {\"inbox\":\"agent:test/c\",\"adapter\":\"slack\"}\n";
        let records = super::Records::parse(twice);
        assert_eq!(records.of("agent:test/b"), None);
        assert!(records.of("agent:test/c").is_some());
    }

    #[test]
    fn a_record_set_replaces_the_one_before_and_a_removed_one_is_gone() {
        let mut records =
            super::Records::parse("{\"inbox\":\"agent:test/b\",\"adapter\":\"terminal\"}\n");
        records.remove("agent:test/c");
        assert!(
            !records.changed,
            "removing an inbox without a record changes nothing"
        );
        records.set(
            "agent:test/b",
            super::Opened {
                adapter: "slack".to_owned(),
                epoch: Some(epoch(OTHER)),
            },
        );
        assert!(records.changed);
        assert_eq!(
            records.lines(),
            [format!(
                "{{\"inbox\":\"agent:test/b\",\"adapter\":\"slack\",\"epoch\":\"{OTHER}\"}}"
            )]
        );
        records.remove("agent:test/b");
        assert!(records.lines().is_empty());
    }

    /// What a reconnect is due to do: an open where the record names another epoch or none, a
    /// repost where a kept envelope of the caller is recorded at another epoch or at none
    /// (round 2: an envelope at no epoch is posted too); nothing for the envelope the command
    /// itself posted, an envelope recorded at the current epoch, another sender's envelope, or a
    /// record of another inbox.
    #[test]
    fn a_reconnect_is_due_where_a_record_or_an_envelope_names_another_epoch() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/tmp/reconnect-due")
            .join(std::process::id().to_string());
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the directory is made");
        let path = dir.join("outbox.jsonl");
        let outbox = super::Outbox::locate(Some(&path)).expect("the outbox");
        let now = at("2026-11-22T12:00:00Z");
        let (current, other) = (epoch(EPOCH), epoch(OTHER));
        let nothing = super::Due::default();
        assert_eq!(outbox.due("agent:test/alice", &current, now, None), nothing);

        let alice = envelope(1, "agent:test/alice", "2099-01-01T00:00:00Z");
        let carol = envelope(2, "agent:test/carol", "2099-01-01T00:00:00Z");
        std::fs::write(&path, format!("{alice}\n{carol}\n")).expect("the outbox file");
        assert_eq!(
            outbox.due("agent:test/alice", &current, now, None),
            nothing,
            "an envelope the epochs file says nothing of"
        );
        outbox.mark_unsent(&id(1), now).expect("written");
        outbox.mark_unsent(&id(3), now).expect("written");
        assert_eq!(
            std::fs::read_to_string(dir.join("outbox.jsonl.epochs")).expect("the epochs file"),
            format!("{}\tnone\n", id(1)),
            "a kept envelope is marked, one not kept is not"
        );
        assert!(
            outbox.due("agent:test/alice", &current, now, None).repost,
            "an envelope marked unsent"
        );
        assert_eq!(
            outbox.due("agent:test/alice", &current, now, Some(&id(1))),
            nothing,
            "the command's own envelope"
        );
        outbox.record_epoch(&id(1), &current, now).expect("written");
        outbox.mark_unsent(&id(1), now).expect("written");
        assert_eq!(
            outbox.due("agent:test/alice", &current, now, None),
            nothing,
            "an envelope recorded at an epoch is not marked"
        );
        std::fs::write(
            dir.join("outbox.jsonl.epochs"),
            format!("{}\t{OTHER}\n{}\t{OTHER}\n", id(1), id(2)),
        )
        .expect("the epochs file");
        assert!(outbox.due("agent:test/alice", &current, now, None).repost);
        assert!(!outbox.due("agent:test/alice", &other, now, None).repost);
        assert!(!outbox.due("agent:test/bob", &current, now, None).repost);

        let slack = super::Opened {
            adapter: "slack".to_owned(),
            epoch: Some(other.clone()),
        };
        assert_eq!(
            outbox
                .record_opened("agent:test/alice", "slack", Some(&other), Some(&slack))
                .expect("written"),
            super::Recorded::Gone,
            "from a record: no record yet"
        );
        assert_eq!(outbox.opened("agent:test/alice"), None);
        assert_eq!(
            outbox
                .record_opened("agent:test/alice", "slack", Some(&other), None)
                .expect("written"),
            super::Recorded::Written
        );
        let due = outbox.due("agent:test/alice", &current, now, None);
        assert_eq!(due.open, Some(slack.clone()));
        assert_eq!(outbox.due("agent:test/alice", &other, now, None).open, None);
        assert_eq!(outbox.due("agent:test/bob", &current, now, None).open, None);
        let terminal = super::Opened {
            adapter: "terminal".to_owned(),
            epoch: None,
        };
        assert_eq!(
            outbox
                .record_opened("agent:test/alice", "slack", Some(&current), Some(&terminal))
                .expect("written"),
            super::Recorded::Other,
            "from another record: left as it is"
        );
        assert_eq!(outbox.opened("agent:test/alice"), Some(slack.clone()));
        assert_eq!(
            outbox
                .record_opened("agent:test/alice", "terminal", None, Some(&slack))
                .expect("written"),
            super::Recorded::Written,
            "from the record held: replaced"
        );
        assert_eq!(
            outbox.due("agent:test/alice", &other, now, None).open,
            Some(terminal),
            "a record at no epoch is opened again"
        );
        let taken = outbox.take_opened("agent:test/alice").expect("written");
        assert_eq!(outbox.opened("agent:test/alice"), None);
        let taken = taken.expect("the record taken");
        assert_eq!(taken.adapter, "terminal");
        outbox
            .put_back_opened("agent:test/alice", taken.clone())
            .expect("written");
        assert_eq!(outbox.opened("agent:test/alice"), Some(taken));
        outbox
            .record_opened("agent:test/alice", "slack", None, None)
            .expect("written");
        outbox
            .put_back_opened(
                "agent:test/alice",
                super::Opened {
                    adapter: "terminal".to_owned(),
                    epoch: None,
                },
            )
            .expect("written");
        assert_eq!(
            outbox
                .opened("agent:test/alice")
                .map(|record| record.adapter),
            Some("slack".to_owned()),
            "a record written since is not replaced"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The most steps a read of `lines` lines and 1 envelope may take: 3 per line (it is split,
    /// the id of a line kept is read as text, and room for the comparisons of ids whose hashes
    /// meet), a few for the envelope.
    fn most_steps(lines: u32) -> usize {
        3 * lines as usize + 16
    }

    /// The steps `Outbox::kept` (what `outbox` and `repost` read) takes over an outbox file of
    /// one envelope and an epochs file of `lines` lines, the first of them that envelope's, and
    /// the epoch it found for the envelope. A step past [`most_steps`] panics, so a read in pairs
    /// fails within its first few thousand comparisons.
    fn steps_of_a_read(lines: u32) -> (usize, Option<Epoch>) {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/tmp/epochs-steps")
            .join(format!("{lines}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the directory is made");
        let path = dir.join("outbox.jsonl");
        let line = envelope(0, "agent:test/alice", "2099-01-01T00:00:00Z");
        std::fs::write(&path, format!("{line}\n")).expect("the outbox file");
        let mut text = String::new();
        for n in 0..lines {
            text.push_str(&format!("{}\t{EPOCH}\n", id(n)));
        }
        std::fs::write(dir.join("outbox.jsonl.epochs"), text).expect("the epochs file");
        let outbox = super::Outbox::locate(Some(&path)).expect("the outbox");
        super::STEPS.with(|steps| steps.set(0));
        super::MOST.with(|most| most.set(most_steps(lines)));
        let contents = outbox
            .kept("agent:test/alice", at("2026-11-22T12:00:00Z"))
            .expect("the outbox is read");
        super::MOST.with(|most| most.set(usize::MAX));
        let steps = super::STEPS.with(std::cell::Cell::get);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(contents.kept.len(), 1, "the one envelope");
        (steps, contents.kept[0].epoch.clone())
    }

    /// The steps of `during`, counted on this thread.
    fn steps_of<T>(during: impl FnOnce() -> T) -> (usize, T) {
        let before = super::STEPS.with(std::cell::Cell::get);
        let value = during();
        (super::STEPS.with(std::cell::Cell::get) - before, value)
    }

    /// The step case catches a read in pairs only through these steps: one comparison of two
    /// `LineId` values is 1 step, whether they are equal or not, and one read of an id's text is
    /// 1 step.
    #[test]
    fn a_comparison_of_two_line_ids_and_a_read_of_one_text_are_one_step_each() {
        let (a, b, c) = (super::LineId("a"), super::LineId("a"), super::LineId("b"));
        assert_eq!(steps_of(|| a == b), (1, true), "a comparison of equal ids");
        assert_eq!(
            steps_of(|| a == c),
            (1, false),
            "a comparison of different ids"
        );
        assert_eq!(steps_of(|| c.text()), (1, "b"), "a read of an id's text");
    }

    /// F3 of wave 8's adversary pass 1: every command that touches the outbox file reads the
    /// epochs file, and a read that compared every line with every other took 7.67 s for 20 001
    /// lines. The read is held to steps linear in the file's lines, counted, not timed. A message
    /// id reaches the read as a `LineId`, whose every comparison with another `LineId` and every
    /// read of its text is a step, so a read in pairs over `LineId` values is stopped at the
    /// bound (one that compares the texts themselves is not counted): at most 3 steps per line,
    /// and at least 1 (each line is split once), so a counter that counts nothing fails too.
    #[test]
    fn an_epochs_file_of_20000_lines_is_read_in_steps_linear_in_its_lines() {
        for lines in [2_001, 20_001] {
            let (steps, found) = steps_of_a_read(lines);
            assert_eq!(found, Some(epoch(EPOCH)), "the entry of {lines} lines");
            assert!(
                steps >= lines as usize,
                "{steps} steps for {lines} lines: fewer than one per line"
            );
            assert!(
                steps <= most_steps(lines),
                "{steps} steps for {lines} lines"
            );
        }
    }
}
