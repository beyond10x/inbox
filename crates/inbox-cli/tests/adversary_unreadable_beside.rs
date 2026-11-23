//! Adversary pass 1 on story:an-unreadable-epochs-file-keeps-its-other-lines. The story's claim:
//! a command that has to change `<outbox>.epochs` or `<outbox>.opened` and cannot read it exits 4
//! with one line on standard error naming the file and the reason, and writes nothing; `inbox
//! open` fails only after the service opened the inbox. Every key is made at run time.

use crate::support;

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

use support::{BOB, LOCAL, Run, World};

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

/// `inbox repost` over an epochs file that is not UTF-8, against a service that answers its
/// post `503` (it is stopping; the request was not taken): the envelope is to be marked unsent,
/// a change of the epochs file, which cannot be read. The story and `docs/design/service.md`
/// say such a repost exits 4 after the report; it exits as a refusal instead and names the file
/// only in a warning.
#[test]
fn a_repost_answered_503_over_an_unreadable_epochs_file_exits_4() {
    let world = World::new("adv-unreadable-503");
    world.client("bob", &["open"]).ok();
    world
        .client("alice", &["post", "--to", BOB, "--body", "first"])
        .ok();
    let outbox = world.outbox("alice");
    let path = beside(&outbox, ".epochs");
    let mut bytes = fs::read(&path).expect("the epochs file");
    bytes.extend([0xff, 0xfe, b'\n']);
    fs::write(&path, &bytes).expect("written");
    let (url, served) = support::stand_in_answers(vec![(503, None, r#"{"refused":"stopping"}"#)]);
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
    assert_eq!(fs::read(&path).expect("the epochs file"), bytes);
    assert_eq!(run.status, LOCAL, "{run:#?}");
}

/// `inbox repost` over an epochs file that is a FIFO, and over one past its bound: one failure,
/// so one line on standard error names the file. The read of the kept envelopes warns
/// "read as holding nothing" first, and the failed change names the same file again: 2 lines,
/// the first of them saying the file was read and the command went on.
#[test]
fn a_repost_over_an_epochs_file_it_cannot_read_names_it_in_one_line() {
    let world = World::new("adv-unreadable-lines");
    world.client("bob", &["open"]).ok();
    world
        .client("alice", &["post", "--to", BOB, "--body", "first"])
        .ok();
    let path = beside(&world.outbox("alice"), ".epochs");
    let epochs = fs::read(&path).expect("the epochs file");
    let mut wrong = Vec::new();
    for state in ["fifo", "over its bound"] {
        if state == "fifo" {
            fifo(&path);
        } else {
            fs::write(&path, &epochs).expect("written");
            fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .expect("the file")
                .set_len(64 * 1024 * 1024 + 1)
                .expect("grown");
        }
        let run = world.client("alice", &["repost"]);
        let lines = naming(&run, &path);
        if run.status != LOCAL || lines.len() != 1 {
            wrong.push(format!(
                "{state}: exit {}, {} lines name the file: {lines:#?}",
                run.status,
                lines.len()
            ));
        }
        let _ = fs::remove_file(&path);
        fs::write(&path, &epochs).expect("written");
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// `inbox post` over an epochs file in each state it cannot be read in: exactly one line on
/// standard error names the file, and none of the file's bytes is in it (a line of the old
/// warning beside the failure would be a second line).
#[test]
fn a_post_over_an_unreadable_epochs_file_writes_one_line_without_its_content() {
    let world = World::new("adv-unreadable-post-line");
    world.client("bob", &["open"]).ok();
    world
        .client("alice", &["post", "--to", BOB, "--body", "first"])
        .ok();
    let path = beside(&world.outbox("alice"), ".epochs");
    let epochs = fs::read(&path).expect("the epochs file");
    let secret = "SENTINEL-CONTENT";
    let mut wrong = Vec::new();
    for state in ["no permission", "not utf-8", "over its bound", "fifo"] {
        match state {
            "no permission" => {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).expect("chmod");
                if fs::File::open(&path).is_ok() {
                    wrong.push("this user reads a file of mode 0000".to_owned());
                }
            }
            "not utf-8" => {
                let mut bytes = epochs.clone();
                bytes.extend(secret.as_bytes());
                bytes.extend([0xc3, b'\n']);
                fs::write(&path, bytes).expect("written");
            }
            "over its bound" => {
                fs::OpenOptions::new()
                    .write(true)
                    .open(&path)
                    .expect("the file")
                    .set_len(64 * 1024 * 1024 + 1)
                    .expect("grown");
            }
            _ => fifo(&path),
        }
        let run = world.client("alice", &["post", "--to", BOB, "--body", "second"]);
        let lines = naming(&run, &path);
        if run.status != LOCAL
            || lines.len() != 1
            || run.stderr.lines().count() != 1
            || run.stderr.contains(secret)
        {
            wrong.push(format!("{state}: {run:#?}"));
        }
        let _ = fs::remove_file(&path);
        fs::write(&path, &epochs).expect("written");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("chmod");
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// `inbox open` over a file of the opened inbox it cannot read: the service opened the inbox
/// before the file was read (a post to it arrives), the outcome is printed, and the command exits
/// 4. An `open` that read the file before its request would leave the inbox closed.
#[test]
fn an_open_over_an_unreadable_opened_file_opens_the_inbox_and_prints_it() {
    let world = World::new("adv-unreadable-open");
    world.client("bob", &["open"]).ok();
    world.client("bob", &["close"]).ok();
    let path = beside(&world.outbox("bob"), ".opened");
    let bytes = [0xffu8, b'\n'];
    fs::write(&path, bytes).expect("written");
    let run = world.client("bob", &["open"]);
    assert_eq!(run.status, LOCAL, "{run:#?}");
    assert!(run.stdout.trim().ends_with(BOB), "{run:#?}");
    assert_eq!(fs::read(&path).expect("the file"), bytes);
    world
        .client("alice", &["post", "--to", BOB, "--body", "open"])
        .ok();
}

/// `inbox post` over an epochs file that is no regular file in other ways than a FIFO without a
/// writer: a directory, a socket, a FIFO with a writer, a link to `/dev/null`, a link loop, a
/// link to a directory. Each exits 4 within 2 s and the thing at the path is as it was.
#[test]
fn a_post_over_an_epochs_path_of_another_kind_exits_4_and_leaves_it() {
    let world = World::new("adv-unreadable-kinds");
    world.client("bob", &["open"]).ok();
    world
        .client("alice", &["post", "--to", BOB, "--body", "first"])
        .ok();
    let path = beside(&world.outbox("alice"), ".epochs");
    let elsewhere = world.dir.join("elsewhere");
    fs::create_dir_all(&elsewhere).expect("a directory");
    let mut wrong = Vec::new();
    for kind in [
        "directory",
        "socket",
        "fifo with a writer",
        "link to /dev/null",
        "link loop",
        "link to a directory",
    ] {
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(&path);
        let mut _held = None;
        let mut _listener = None;
        match kind {
            "directory" => fs::create_dir(&path).expect("a directory"),
            "socket" => {
                // A socket's path is bound to at most 107 bytes: bound under a short name in
                // the test's directory (one process per case under nextest), then renamed.
                std::env::set_current_dir(&world.dir.path).expect("cd");
                _listener = Some(std::os::unix::net::UnixListener::bind("s").expect("a socket"));
                fs::rename(world.dir.join("s"), &path).expect("the socket is renamed");
            }
            "fifo with a writer" => {
                fifo(&path);
                _held = Some(
                    fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(&path)
                        .expect("held open"),
                );
            }
            "link to /dev/null" => {
                std::os::unix::fs::symlink("/dev/null", &path).expect("a link");
            }
            "link loop" => std::os::unix::fs::symlink(&path, &path).expect("a link"),
            _ => std::os::unix::fs::symlink(&elsewhere, &path).expect("a link"),
        }
        let before = fs::symlink_metadata(&path).expect("there");
        let started = std::time::Instant::now();
        let run = world.client("alice", &["post", "--to", BOB, "--body", "second"]);
        let took = started.elapsed();
        let after = fs::symlink_metadata(&path).expect("still there");
        if run.status != LOCAL
            || took > std::time::Duration::from_secs(2)
            || before.ino() != after.ino()
            || before.mode() != after.mode()
            || naming(&run, &path).len() != 1
        {
            wrong.push(format!("{kind}: {took:?} {run:#?}"));
        }
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(&path);
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
