//! Adversary pass 2 on story:an-unreadable-epochs-file-keeps-its-other-lines: the exit statuses
//! and the lines of `inbox repost` for each answer of the service and a transport failure after
//! 0, 1 and 2 posts, over a readable and an unreadable epochs file; and 2 unreadable files beside
//! one outbox in one run, each named once. Every key is made at run time.

use crate::support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use support::{BOB, LOCAL, REFUSED, Run, TRANSPORT, World};

fn beside(outbox: &Path, suffix: &str) -> PathBuf {
    let mut path = outbox.as_os_str().to_owned();
    path.push(suffix);
    PathBuf::from(path)
}

/// The lines of standard error that name `path`.
fn naming(run: &Run, path: &Path) -> Vec<String> {
    let shown = path.display().to_string();
    run.stderr
        .lines()
        .filter(|line| line.contains(&shown))
        .map(str::to_owned)
        .collect()
}

fn fifo(path: &Path) {
    let _ = fs::remove_file(path);
    let made = Command::new("mkfifo")
        .arg(path)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "mkfifo {}", path.display());
}

/// The state of the epochs file a run starts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Epochs {
    /// Readable and empty: no envelope has an epoch, so every one is posted and no epoch asked.
    Empty,
    /// Not UTF-8: unreadable, and read without a word.
    NotUtf8,
    /// A FIFO: unreadable, and read with the warning "read as holding nothing".
    Fifo,
}

fn put(path: &Path, state: Epochs) {
    let _ = fs::remove_file(path);
    match state {
        Epochs::Empty => fs::write(path, b"").expect("written"),
        Epochs::NotUtf8 => fs::write(path, [0xff, 0xfe, b'\n']).expect("written"),
        Epochs::Fifo => fifo(path),
    }
}

/// What the epochs file is after a run: its bytes, or `None` for a FIFO.
fn bytes_of(path: &Path) -> Option<Vec<u8>> {
    let meta = fs::symlink_metadata(path).expect("the epochs path is there");
    meta.file_type()
        .is_file()
        .then(|| fs::read(path).expect("the epochs file"))
}

/// A world in which alice keeps `n` envelopes to bob: the outbox, the epochs path, the bytes of
/// the outbox file.
fn kept(name: &str, n: usize) -> (World, PathBuf, PathBuf, Vec<u8>) {
    let world = World::new(name);
    world.client("bob", &["open"]).ok();
    for at in 0..n {
        world
            .client(
                "alice",
                &["post", "--to", BOB, "--body", &format!("kept {at}")],
            )
            .ok();
    }
    let outbox = world.outbox("alice");
    let epochs = beside(&outbox, ".epochs");
    let kept = fs::read(&outbox).expect("the outbox file");
    assert_eq!(
        String::from_utf8_lossy(&kept).lines().count(),
        n,
        "{n} envelopes kept"
    );
    (world, outbox, epochs, kept)
}

/// `inbox repost` of one kept envelope against a stand-in that answers its post with each of
/// `408`, `500`, `503`, `UnknownInbox`, `413` and a final declared refusal, over a readable
/// (empty), a not-UTF-8 and a FIFO epochs file. The story, `docs/design/service.md` ("exits 4
/// after the report") and the CHANGELOG: over an unreadable file every answer but a final
/// refusal (which changes only the outbox file) exits 4, the outbox and the epochs file are as
/// they were, and exactly one line names the epochs file; over a readable file each exits 1. A
/// final refusal over a FIFO keeps the warning "read as holding nothing" of release 0.6.0: the run
/// changed no epochs file, so the word held back by its read is written (`Outbox::speak`).
#[test]
fn a_repost_over_an_unreadable_epochs_file_exits_by_the_answer_as_the_table_says() {
    let (world, outbox, epochs, kept_bytes) = kept("adv2-repost-table", 1);
    let shown = outbox.display().to_string();
    let answers: [(&str, u16, &'static str, bool); 6] = [
        ("408", 408, r#"{"refused":"timeout"}"#, false),
        ("500", 500, r#"{"refused":"internal"}"#, false),
        ("503", 503, r#"{"refused":"stopping"}"#, false),
        (
            "UnknownInbox",
            404,
            r#"{"outcome":"refused","error":"inbox.messaging.UnknownInbox"}"#,
            false,
        ),
        ("413", 413, r#"{"refused":"too large"}"#, false),
        (
            "final",
            409,
            r#"{"outcome":"refused","error":"inbox.messaging.Expired"}"#,
            true,
        ),
    ];
    let mut wrong = Vec::new();
    let mut table = Vec::new();
    for (label, status, body, final_refusal) in answers {
        for state in [Epochs::Empty, Epochs::NotUtf8, Epochs::Fifo] {
            put(&epochs, state);
            fs::write(&outbox, &kept_bytes).expect("the outbox file put back");
            let before = bytes_of(&epochs);
            let (url, served) = support::stand_in_answers(vec![(status, None, body)]);
            let run = world.client_env(
                "alice",
                &["repost"],
                &[
                    ("INBOX_URL", url.as_str()),
                    ("INBOX_OUTBOX", shown.as_str()),
                ],
            );
            let requests = served.join().expect("the stand-in ends");
            let lines = naming(&run, &epochs);
            let outbox_after = fs::read(&outbox).expect("the outbox file");
            table.push(format!(
                "{label} {state:?}: exit {}, {} lines name the epochs file",
                run.status,
                lines.len()
            ));
            let unreadable = state != Epochs::Empty;
            let (exit, named) = match (unreadable, final_refusal) {
                (true, false) => (LOCAL, 1),
                (true, true) => (REFUSED, usize::from(state == Epochs::Fifo)),
                (false, _) => (REFUSED, 0),
            };
            if requests.len() != 1
                || run.status != exit
                || lines.len() != named
                || (unreadable && bytes_of(&epochs) != before)
                || (final_refusal != (outbox_after != kept_bytes))
            {
                wrong.push(format!(
                    "{label} {state:?}: wanted exit {exit} and {named} lines naming the epochs \
                     file, the outbox file {}: {requests:?} {run:#?}",
                    if final_refusal {
                        "without it"
                    } else {
                        "as it was"
                    }
                ));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "\n{}\n\n{}",
        table.join("\n"),
        wrong.join("\n")
    );
}

/// `inbox repost` of 3 kept envelopes against a stand-in that takes 0, 1 or 2 posts (`202`
/// without an epoch) and then is gone: the next post fails in transport and ends the run with 3.
/// Over an unreadable epochs file the change before it failed (the mark of the envelope whose post
/// failed, or the mark taken off an arrival), and that local failure is named in exactly one line
/// ("a local failure before it is named in one line"); over a readable one no line names the
/// file. Over an unreadable file the outbox and the epochs file are as they were.
#[test]
fn a_repost_ended_by_a_transport_failure_exits_3_and_names_the_local_failure_once() {
    let (world, outbox, epochs, kept_bytes) = kept("adv2-repost-transport", 3);
    let shown = outbox.display().to_string();
    let mut wrong = Vec::new();
    for taken in 0..3 {
        for state in [Epochs::Empty, Epochs::NotUtf8, Epochs::Fifo] {
            put(&epochs, state);
            fs::write(&outbox, &kept_bytes).expect("the outbox file put back");
            let before = bytes_of(&epochs);
            let answers = (0..taken)
                .map(|_| (202, None, r#"{"outcome":"posted","published":[]}"#))
                .collect();
            let (url, served) = support::stand_in_answers(answers);
            // Without an answer the stand-in ends at once and its port is closed.
            let mut served = Some(served);
            if taken == 0 {
                served
                    .take()
                    .map(|thread| thread.join().expect("the stand-in ends"));
            }
            let run = world.client_env(
                "alice",
                &["repost"],
                &[
                    ("INBOX_URL", url.as_str()),
                    ("INBOX_OUTBOX", shown.as_str()),
                ],
            );
            if let Some(thread) = served {
                let requests = thread.join().expect("the stand-in ends");
                if requests.len() != taken {
                    wrong.push(format!("{taken} {state:?}: requests {requests:?}"));
                }
            }
            let lines = naming(&run, &epochs);
            let unreadable = state != Epochs::Empty;
            let named = usize::from(unreadable);
            if run.status != TRANSPORT
                || lines.len() != named
                || (unreadable && bytes_of(&epochs) != before)
                || fs::read(&outbox).expect("the outbox file") != kept_bytes
            {
                wrong.push(format!(
                    "after {taken} posts, {state:?}: wanted exit {TRANSPORT} and {named} lines \
                     naming the epochs file, the outbox file as it was: {run:#?}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// The 2 files beside one outbox unreadable in one run (both FIFOs). `inbox open` of alice, who
/// keeps an envelope, fails on `.opened` (one line names it) and its reconnect reads `.epochs`
/// only: one warning "read as holding nothing" names it, not dropped for the other file. `inbox
/// repost` fails on `.epochs` (one line names it) and its reconnect reads `.opened` only: one
/// warning names it.
#[test]
fn two_unreadable_files_beside_one_outbox_are_each_named_once() {
    let (world, outbox, epochs, _) = kept("adv2-two-files", 1);
    world.client("alice", &["open"]).ok();
    let opened = beside(&outbox, ".opened");
    let mut wrong = Vec::new();
    for command in ["open", "repost"] {
        fifo(&opened);
        fifo(&epochs);
        let run = world.client("alice", &[command]);
        let on_opened = naming(&run, &opened);
        let on_epochs = naming(&run, &epochs);
        let (failed, read) = if command == "open" {
            (&on_opened, &on_epochs)
        } else {
            (&on_epochs, &on_opened)
        };
        if run.status != LOCAL
            || failed.len() != 1
            || failed[0].ends_with("read as holding nothing")
            || read.len() != 1
            || !read[0].ends_with("read as holding nothing")
        {
            wrong.push(format!("inbox {command}: {run:#?}"));
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// What is planted at a staged file (`<file>.new`) before a change: what an interrupted run
/// leaves there, or what another process put there.
#[derive(Debug, Clone, Copy)]
enum Staged {
    /// A symbolic link to the file the staged file replaces.
    LinkToTheFile,
    /// A symbolic link to another file in the directory.
    LinkElsewhere,
    /// A FIFO without a reader or writer.
    Fifo,
    /// A regular file, left by a run that ended between its write and its rename.
    Left,
}

/// An entry at `<outbox>.epochs.new` or `<outbox>.new` before `inbox post` (answered `413`): a
/// link to the file it is staged for, a link to another file of the directory, a FIFO, and a
/// regular file left behind. The change does not follow it: it removes it and creates the staged
/// file anew with mode 0600 (`replace` in `src/outbox.rs`). The post exits by its answer (1); the
/// epochs file and the outbox file are regular files of mode 0600 that still hold their other
/// entries; the other file is byte for byte as it was; no `.new` entry is left.
#[test]
fn a_stale_entry_at_a_staged_file_is_removed_not_followed() {
    use std::os::unix::fs::PermissionsExt;

    let (world, outbox, epochs, _) = kept("adv2-staged", 1);
    let shown = outbox.display().to_string();
    let other = world.dir.join("other");
    let other_bytes = b"another file of the directory\n".to_vec();
    let mut wrong = Vec::new();
    let mut good = [
        fs::read_to_string(&epochs).expect("the epochs file"),
        fs::read_to_string(&outbox).expect("the outbox file"),
    ];
    for file in [epochs.clone(), outbox.clone()] {
        for planted in [
            Staged::LinkToTheFile,
            Staged::LinkElsewhere,
            Staged::Fifo,
            Staged::Left,
        ] {
            fs::write(&other, &other_bytes).expect("the other file");
            // A row that left a file that is no regular file gets it back as the last row read
            // it, so each row is told apart.
            for (path, text) in [&epochs, &outbox].into_iter().zip(good.iter_mut()) {
                match fs::symlink_metadata(path) {
                    Ok(meta) if meta.file_type().is_file() => {
                        *text = fs::read_to_string(path).expect("the file");
                    }
                    _ => {
                        let _ = fs::remove_file(path);
                        fs::write(path, text.as_bytes()).expect("written");
                        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
                            .expect("chmod 0600");
                    }
                }
            }
            let [entries, envelopes] = good.clone();
            let staged = beside(&file, ".new");
            let _ = fs::remove_file(&staged);
            match planted {
                Staged::LinkToTheFile => {
                    std::os::unix::fs::symlink(file.file_name().expect("a name"), &staged)
                        .expect("a link")
                }
                Staged::LinkElsewhere => {
                    std::os::unix::fs::symlink("other", &staged).expect("a link");
                }
                Staged::Fifo => fifo(&staged),
                Staged::Left => fs::write(&staged, b"half a file").expect("written"),
            }
            let (url, served) =
                support::stand_in_answers(vec![(413, None, r#"{"refused":"too large"}"#)]);
            let run = world.client_env(
                "alice",
                &["post", "--to", BOB, "--body", "second"],
                &[
                    ("INBOX_URL", url.as_str()),
                    ("INBOX_OUTBOX", shown.as_str()),
                ],
            );
            served.join().expect("the stand-in ends");
            let row = format!("{planted:?} at {}", staged.display());
            if run.status != REFUSED {
                wrong.push(format!(
                    "{row}: exit {}, not {REFUSED}: {run:#?}",
                    run.status
                ));
            }
            for (path, before) in [(&epochs, &entries), (&outbox, &envelopes)] {
                let meta = fs::symlink_metadata(path).expect("the file is there");
                if !meta.file_type().is_file() || meta.permissions().mode() & 0o777 != 0o600 {
                    wrong.push(format!(
                        "{row}: {} is no regular file of mode 0600: {meta:?}",
                        path.display()
                    ));
                    continue;
                }
                let after = fs::read_to_string(path).expect("the file");
                let lost: Vec<&str> = before
                    .lines()
                    .filter(|line| !after.lines().any(|held| held == *line))
                    .collect();
                if !lost.is_empty() {
                    wrong.push(format!("{row}: {} lost {lost:?}", path.display()));
                }
            }
            if fs::read(&other).expect("the other file") != other_bytes {
                wrong.push(format!("{row}: the other file changed"));
            }
            for left in [beside(&epochs, ".new"), beside(&outbox, ".new")] {
                if fs::symlink_metadata(&left).is_ok() {
                    wrong.push(format!("{row}: {} is left", left.display()));
                    let _ = fs::remove_file(&left);
                }
            }
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// `inbox repost` over an epochs file that is not UTF-8, 2 kept envelopes: the first arrives (its
/// epoch cannot be recorded, the change fails), the second is refused for good. As the design and
/// the CHANGELOG say, the run exits 4 and leaves the epochs file as it was, but drops from the
/// outbox file the envelope refused for good in the same run: the one that arrived stays kept.
#[test]
fn a_failed_repost_drops_the_envelope_refused_for_good_and_keeps_the_one_that_arrived() {
    let (world, outbox, epochs, kept_bytes) = kept("adv2-final-drop", 2);
    put(&epochs, Epochs::NotUtf8);
    let unreadable = fs::read(&epochs).expect("the epochs file");
    let shown = outbox.display().to_string();
    let (url, served) = support::stand_in_answers(vec![
        (202, None, r#"{"outcome":"posted","published":[]}"#),
        (
            409,
            None,
            r#"{"outcome":"refused","error":"inbox.messaging.Expired"}"#,
        ),
    ]);
    let run = world.client_env(
        "alice",
        &["repost"],
        &[
            ("INBOX_URL", url.as_str()),
            ("INBOX_OUTBOX", shown.as_str()),
        ],
    );
    served.join().expect("the stand-in ends");
    assert_eq!(run.status, LOCAL, "{run:#?}");
    let kept_text = String::from_utf8_lossy(&kept_bytes).into_owned();
    let first = kept_text.lines().next().expect("the first envelope");
    assert_eq!(
        String::from_utf8_lossy(&fs::read(&outbox).expect("the outbox file")),
        format!("{first}\n"),
        "{run:#?}"
    );
    assert_eq!(fs::read(&epochs).expect("the epochs file"), unreadable);
}
