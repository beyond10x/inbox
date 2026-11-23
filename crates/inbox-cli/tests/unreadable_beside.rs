//! The cases of story:an-unreadable-epochs-file-keeps-its-other-lines: a command that has to
//! change the epochs file beside the outbox file (`<outbox>.epochs`) or the file of the opened
//! inbox (`<outbox>.opened`) and cannot read it fails with exit status 4, names the file and the
//! reason in a line on standard error, and leaves the file as it was. "Cannot read" is each of 4
//! states: no permission to read, not UTF-8, over its bound, no regular file. A file that does
//! not exist is read as empty. Every key is made at run time with the binary itself.

use crate::support;

use std::fs;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

use support::{BOB, LOCAL, Run, World, ids};

/// The bound of a read of the epochs file (`MOST_BYTES` of `src/outbox.rs`).
const MOST_EPOCHS_BYTES: u64 = 64 * 1024 * 1024;
/// The bound of a read of the file of the opened inbox (`MOST_OPENED_BYTES`).
const MOST_OPENED_BYTES: u64 = 1024 * 1024;

/// A state in which a file cannot be read.
#[derive(Debug, Clone, Copy)]
enum State {
    NoPermission,
    NotUtf8,
    OverBound,
    NotRegular,
}

const STATES: [State; 4] = [
    State::NoPermission,
    State::NotUtf8,
    State::OverBound,
    State::NotRegular,
];

fn beside(outbox: &Path, suffix: &str) -> PathBuf {
    let mut path = outbox.as_os_str().to_owned();
    path.push(suffix);
    PathBuf::from(path)
}

/// Puts the regular file at `path` in `state`, its readable lines kept where the state allows,
/// and answers the words of the reason a line on standard error names.
fn make(path: &Path, state: State, most: u64) -> String {
    match state {
        State::NoPermission => {
            fs::set_permissions(path, fs::Permissions::from_mode(0o000)).expect("chmod 0000");
            assert!(
                fs::File::open(path).is_err(),
                "{}: this test's user reads a file of mode 0000 (it runs as root or holds \
                 CAP_DAC_OVERRIDE), so the case of no permission to read cannot be made here",
                path.display()
            );
            "Permission denied".to_owned()
        }
        State::NotUtf8 => {
            let mut bytes = fs::read(path).expect("the file");
            bytes.extend([0xff, 0xfe, b'\t', 0x80, b'\n']);
            fs::write(path, bytes).expect("written");
            "valid UTF-8".to_owned()
        }
        State::OverBound => {
            // Sparse, with no block of its own: a write of any byte of it would take one.
            let file = fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(path)
                .expect("the file");
            file.set_len(most + 1)
                .expect("the file grows past its bound");
            let blocks = fs::metadata(path).expect("the file").blocks();
            assert_eq!(
                blocks,
                0,
                "{}: a file made sparse holds {blocks} blocks; this file system keeps no \
                 sparse file, so the over-bound case cannot tell a write here",
                path.display()
            );
            format!("more than {most} bytes")
        }
        State::NotRegular => {
            fs::remove_file(path).expect("the file is removed");
            let made = Command::new("mkfifo")
                .arg(path)
                .status()
                .expect("mkfifo runs");
            assert!(made.success(), "mkfifo {}", path.display());
            "no regular file".to_owned()
        }
    }
}

/// What a file is: its inode, mode, kind, length and blocks used, and the bytes of a regular
/// file within the bound of the epochs file (read under mode 0600 for a moment when its owner
/// may not read it, and put back). A file over that bound is made sparse ([`make`]): its bytes
/// are not read back, and 0 blocks before and after says no byte of it was written.
#[derive(Debug, PartialEq, Eq)]
struct Held {
    inode: u64,
    mode: u32,
    fifo: bool,
    len: u64,
    blocks: u64,
    bytes: Option<Vec<u8>>,
}

fn held(path: &Path) -> Held {
    let meta = fs::symlink_metadata(path).expect("the file is there");
    let mode = meta.permissions().mode();
    let read = meta.file_type().is_file() && meta.len() <= MOST_EPOCHS_BYTES;
    let bytes = read.then(|| {
        if mode & 0o400 == 0 {
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).expect("chmod 0600");
            let bytes = fs::read(path).expect("the file");
            fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("chmod back");
            bytes
        } else {
            fs::read(path).expect("the file")
        }
    });
    Held {
        inode: meta.ino(),
        mode,
        fifo: meta.file_type().is_fifo(),
        len: meta.len(),
        blocks: meta.blocks(),
        bytes,
    }
}

/// Puts `text` back at `path` as a regular file of mode 0600, whatever is there.
fn restore(path: &Path, text: &[u8]) {
    let _ = fs::remove_file(path);
    fs::write(path, text).expect("written");
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).expect("chmod 0600");
}

/// What is wrong with one run of a command that has to change the file at `path` in a state it
/// cannot be read in: its exit status is not 4, not exactly one line on standard error names the
/// file (the failure, with the reason), or the file changed. Empty when nothing is.
fn wrong(run: &Run, path: &Path, reason: &str, before: &Held) -> Vec<String> {
    let mut wrong = Vec::new();
    if run.status != LOCAL {
        wrong.push(format!("exit status {}, not {LOCAL}", run.status));
    }
    let shown = path.display().to_string();
    let naming: Vec<&str> = run
        .stderr
        .lines()
        .filter(|line| line.contains(&shown))
        .collect();
    if naming.len() != 1 || !naming[0].contains(reason) {
        wrong.push(format!(
            "not exactly one line on standard error names {shown}, with {reason:?}: {naming:#?}"
        ));
    }
    let after = held(path);
    if &after != before {
        wrong.push(format!(
            "the file changed: inode {} -> {}, mode {:o} -> {:o}, fifo {} -> {}, length {} -> {}, \
             blocks {} -> {}, bytes read back {} -> {}",
            before.inode,
            after.inode,
            before.mode,
            after.mode,
            before.fifo,
            after.fifo,
            before.len,
            after.len,
            before.blocks,
            after.blocks,
            before.bytes.as_ref().map_or(0, Vec::len),
            after.bytes.as_ref().map_or(0, Vec::len),
        ));
    }
    wrong
}

/// `inbox post` (the epochs file is changed before the request) and `inbox repost` (changed after
/// the answer of the service) in each of the 4 states of the epochs file: exit 4, a line names
/// the file, the file is as it was. `post` sends nothing: the recipient holds only the first
/// message, and the outbox file is as it was too. One case per command, so the two run side by
/// side.
fn a_change_of_an_unreadable_epochs_file(command: &str) {
    let world = World::new(&format!("unreadable-epochs-{command}"));
    world.client("bob", &["open"]).ok();
    let first = world
        .client("alice", &["post", "--to", BOB, "--body", "first"])
        .ok()
        .stdout
        .trim()
        .to_owned();
    let outbox = world.outbox("alice");
    let path = beside(&outbox, ".epochs");
    let epochs = fs::read(&path).expect("the epochs file");
    let kept = fs::read(&outbox).expect("the outbox file");
    assert!(!epochs.is_empty(), "the first post recorded its epoch");

    let mut failed = Vec::new();
    for state in STATES {
        let reason = make(&path, state, MOST_EPOCHS_BYTES);
        let before = held(&path);
        let run = match command {
            "post" => world.client("alice", &["post", "--to", BOB, "--body", "second"]),
            _ => world.client("alice", &["repost"]),
        };
        let mut wrong = wrong(&run, &path, &reason, &before);
        if fs::read(&outbox).expect("the outbox file") != kept {
            wrong.push("the outbox file changed".to_owned());
        }
        if command == "post" {
            let pending = world.client("bob", &["pending", "--json"]).ok().json();
            if ids(&pending) != std::slice::from_ref(&first) {
                wrong.push(format!("the post was sent: bob holds {:?}", ids(&pending)));
            }
        }
        for what in wrong {
            failed.push(format!("{state:?}, inbox {command}: {what}"));
        }
        restore(&path, &epochs);
        restore(&outbox, &kept);
    }
    assert!(failed.is_empty(), "\n{}", failed.join("\n"));
}

#[test]
fn a_post_over_an_unreadable_epochs_file_fails_with_4_and_leaves_it_as_it_was() {
    a_change_of_an_unreadable_epochs_file("post");
}

#[test]
fn a_repost_over_an_unreadable_epochs_file_fails_with_4_and_leaves_it_as_it_was() {
    a_change_of_an_unreadable_epochs_file("repost");
}

/// An arrival whose answer names no epoch takes the mark unsent off its envelope, a change of the
/// epochs file too: against a service that sends no `Inbox-Epoch`, an `inbox repost` over an
/// epochs file that is not UTF-8 posts the envelope, then exits 4 and leaves the file as it was.
#[test]
fn an_answer_without_an_epoch_over_an_unreadable_epochs_file_fails_with_4() {
    let world = World::new("unreadable-no-epoch");
    world.client("bob", &["open"]).ok();
    world
        .client("alice", &["post", "--to", BOB, "--body", "first"])
        .ok();
    let outbox = world.outbox("alice");
    let path = beside(&outbox, ".epochs");
    let reason = make(&path, State::NotUtf8, MOST_EPOCHS_BYTES);
    let before = held(&path);
    let (url, served) =
        support::stand_in_answers(vec![(202, None, r#"{"outcome":"posted","published":[]}"#)]);
    let shown = outbox.display().to_string();
    let run = world.client_env(
        "alice",
        &["repost"],
        &[
            ("INBOX_URL", url.as_str()),
            ("INBOX_OUTBOX", shown.as_str()),
        ],
    );
    let requests = served.join().expect("the stand-in ends");
    assert_eq!(requests.len(), 1, "{requests:?}");
    assert!(
        requests[0].starts_with("POST /messaging/commands/post "),
        "{requests:?}"
    );
    let wrong = wrong(&run, &path, &reason, &before);
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// `inbox close` (the record is removed before the request) and `inbox open` (written after the
/// answer of the service) in each of the 4 states of the file of the opened inbox, which holds
/// the records of 2 inboxes: exit 4, one line names the file, the file is as it was. `close`
/// sends nothing: the owner's inbox is still open, so a post to it arrives. `open` starts with
/// the inbox closed at the service and sends its request: its outcome is printed, and the inbox
/// is open after it, so a post to it arrives. One case per command, so the two run side by side.
fn a_change_of_an_unreadable_file_of_the_opened_inbox(command: &str) {
    let world = World::new(&format!("unreadable-opened-{command}"));
    world.client("bob", &["open"]).ok();
    let path = beside(&world.outbox("bob"), ".opened");
    let own = fs::read_to_string(&path).expect("the file of the opened inbox");
    let other = own.replace(BOB, "agent:test/carol");
    assert_ne!(own, other, "the record names bob's inbox: {own:?}");
    let records = format!("{other}{own}").into_bytes();
    restore(&path, &records);

    let mut failed = Vec::new();
    for state in STATES {
        if command == "open" {
            world.client("bob", &["close"]).ok();
            restore(&path, &records);
        }
        let reason = make(&path, state, MOST_OPENED_BYTES);
        let before = held(&path);
        let run = world.client("bob", &[command]);
        let mut wrong = wrong(&run, &path, &reason, &before);
        restore(&path, &records);
        if command == "open" && !run.stdout.trim().ends_with(BOB) {
            wrong.push(format!("the outcome of the open is not printed: {run:#?}"));
        }
        let post = world.client("alice", &["post", "--to", BOB, "--body", "open"]);
        if post.status != 0 {
            wrong.push(format!(
                "bob's inbox is not open after inbox {command}: a post to bob ended {post:#?}"
            ));
            world.client("bob", &["open"]).ok();
            restore(&path, &records);
        }
        for what in wrong {
            failed.push(format!("{state:?}, inbox {command}: {what}"));
        }
    }
    assert!(failed.is_empty(), "\n{}", failed.join("\n"));
}

#[test]
fn a_close_over_an_unreadable_file_of_the_opened_inbox_fails_with_4_and_leaves_it_as_it_was() {
    a_change_of_an_unreadable_file_of_the_opened_inbox("close");
}

#[test]
fn an_open_over_an_unreadable_file_of_the_opened_inbox_fails_with_4_and_leaves_it_as_it_was() {
    a_change_of_an_unreadable_file_of_the_opened_inbox("open");
}

/// `inbox outbox` only reads the epochs file: over one that is no regular file or over its
/// bound it exits 0 and names the file in its warning "read as holding nothing" as release 0.6.0
/// does, twice: once by its read of the kept envelopes, once by the reconnect after its answer,
/// which reads the file again because alice keeps an envelope. The file is as it was.
#[test]
fn a_command_that_only_reads_an_unreadable_epochs_file_warns_as_before() {
    let world = World::new("unreadable-read-only");
    world.client("bob", &["open"]).ok();
    world
        .client("alice", &["post", "--to", BOB, "--body", "first"])
        .ok();
    let path = beside(&world.outbox("alice"), ".epochs");
    let epochs = fs::read(&path).expect("the epochs file");
    let shown = path.display().to_string();
    let mut failed = Vec::new();
    for state in [State::OverBound, State::NotRegular] {
        let reason = make(&path, state, MOST_EPOCHS_BYTES);
        let before = held(&path);
        let run = world.client("alice", &["outbox"]);
        let naming: Vec<&str> = run
            .stderr
            .lines()
            .filter(|line| line.contains(&shown))
            .collect();
        if run.status != 0
            || naming.len() != 2
            || !naming
                .iter()
                .all(|line| line.contains(&reason) && line.ends_with("read as holding nothing"))
        {
            failed.push(format!("{state:?}: {run:#?}"));
        }
        if held(&path) != before {
            failed.push(format!("{state:?}: the file changed"));
        }
        restore(&path, &epochs);
    }
    assert!(failed.is_empty(), "\n{}", failed.join("\n"));
}

/// A file that does not exist is read as empty, as before: `inbox post` writes the epochs file
/// with its mark and epoch, and `inbox open` the file of the opened inbox with its record.
#[test]
fn an_absent_epochs_file_or_file_of_the_opened_inbox_is_read_as_empty() {
    let world = World::new("unreadable-absent");
    world.client("bob", &["open"]).ok();
    let opened = beside(&world.outbox("bob"), ".opened");
    fs::remove_file(&opened).expect("the file of the opened inbox is removed");
    world.client("bob", &["open"]).ok();
    let record = fs::read_to_string(&opened).expect("written again");
    assert_eq!(record.lines().count(), 1, "{record:?}");
    assert!(record.contains(BOB), "{record:?}");

    let epochs = beside(&world.outbox("alice"), ".epochs");
    assert!(!epochs.exists());
    let message_id = world
        .client("alice", &["post", "--to", BOB, "--body", "absent"])
        .ok()
        .stdout
        .trim()
        .to_owned();
    let text = fs::read_to_string(&epochs).expect("the epochs file is written");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1, "{text:?}");
    assert!(lines[0].starts_with(&format!("{message_id}\t")), "{text:?}");
}
