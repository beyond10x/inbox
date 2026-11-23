//! Adversary cases of story:a-wake-whose-reader-is-gone-marks-nothing: the check of the reader
//! after the rows of a round are written and before their first mark, which none of the cases of
//! `reader_gone.rs` holds (their mutant m5b, that check removed, lives).

use std::fs;
use std::io::Write as _;
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

use crate::support;

use support::{BOB, LOCAL, World};

/// How long a case waits for a fact or an exit: it only ends a hang.
const HANG: Duration = support::STAND_IN_WAIT;

/// The page of a pipe and the slots of a pipe of 65,536 bytes: 15 full pages leave one slot.
const PAGE: usize = 4096;
const SLOTS: usize = 16;

/// The state of `message_id` in bob's `Pending`, or an empty text when it is not listed.
fn state(world: &World, message_id: &str) -> String {
    let listed = world.client("bob", &["pending", "--json"]).ok();
    support::rows(&listed.json())
        .iter()
        .find(|row| support::text(row, "message_id") == message_id)
        .map(|row| support::text(row, "state"))
        .unwrap_or_default()
}

/// Whether the main thread of `pid` is in splice(2) (x86-64 number 275): the check of the reader
/// on a pipe that is full waits there for room or for the end of the reader.
fn main_thread_in_splice(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/task/{pid}/syscall"))
        .is_ok_and(|line| line.starts_with("275 "))
}

fn end(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// A reader that is alive and reads nothing: 15 pages of the pipe hold bytes before the wake
/// starts, and the rows of the round take the last slot. Its reader then closes the pipe without
/// reading the rows. The check after the write waits on the full pipe and then sees the reader
/// gone: exit 4, one line, and the post stays in `Sent`. Without that check the post is marked
/// at once, though no reader ever took it.
#[test]
fn rows_in_a_full_pipe_whose_reader_closes_unread_are_not_marked() {
    let world = World::new("adversary-reader-gone-full");
    world.client("bob", &["open"]).ok();
    let message = world
        .client("alice", &["post", "--to", BOB, "--body", "short"])
        .ok()
        .stdout
        .trim()
        .to_owned();
    let (reader, mut writer) = std::io::pipe().expect("a pipe");
    writer
        .write_all(&vec![b'.'; PAGE * (SLOTS - 1)])
        .expect("15 pages fit the pipe");
    let mut command = world.client_command("bob", &["wake"]);
    command.stdout(Stdio::from(writer)).stderr(Stdio::piped());
    let mut child = command.spawn().expect("the binary starts");
    let pid = child.id();
    let started = Instant::now();
    loop {
        if main_thread_in_splice(pid) {
            break;
        }
        let now = state(&world, &message);
        if now == "Delivered" {
            end(&mut child);
            panic!(
                "the post was marked delivered while its rows sat unread in a full pipe, before \
                 the reader closed it: no check of the reader ran after the write"
            );
        }
        if started.elapsed() > HANG {
            end(&mut child);
            panic!("the wake neither checked its reader nor marked within {HANG:?}: {now}");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    drop(reader);
    let started = Instant::now();
    while child.try_wait().expect("asked").is_none() {
        if started.elapsed() > HANG {
            end(&mut child);
            panic!("the wake did not end within {HANG:?} after its reader closed");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().expect("waited");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(output.status.code(), Some(LOCAL), "{stderr}");
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.contains("reader is gone"), "{stderr}");
    assert_eq!(state(&world, &message), "Sent", "{stderr}");
}
