//! Adversary cases of `adapters/claude-code/scripts/wake.sh` for
//! story:a-wake-whose-reader-is-gone-marks-nothing, round 2: the file `text` the hook keeps the
//! rows in. Each case runs the script with `sh` under a stand-in `inbox`; none starts Claude Code
//! or the real `inbox`.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::{hook_shell, hook_shells, on_path, repository, scratch};

const HANG: Duration = Duration::from_secs(10);

fn setup(name: &str, stand_in: &str) -> PathBuf {
    let dir = scratch(name);
    fs::create_dir_all(dir.join("bin")).unwrap();
    fs::create_dir_all(dir.join("tmp")).unwrap();
    fs::create_dir_all(dir.join("state")).unwrap();
    fs::write(
        dir.join("state/outbox.jsonl.opened"),
        "{\"inbox\":\"agent:case/session\",\"adapter\":\"claude_code\"}\n",
    )
    .unwrap();
    fs::write(dir.join("bin/inbox"), stand_in).unwrap();
    fs::set_permissions(dir.join("bin/inbox"), fs::Permissions::from_mode(0o755)).unwrap();
    dir
}

fn hook(dir: &Path, wrapper: &str) -> Command {
    hook_under(&hook_shell(), dir, wrapper)
}

fn hook_under(shell: &Path, dir: &Path, wrapper: &str) -> Command {
    let mut command = Command::new(shell);
    command
        .args([
            "-c",
            wrapper,
            shell.to_str().unwrap(),
            repository()
                .join("adapters/claude-code/scripts/wake.sh")
                .to_str()
                .unwrap(),
        ])
        .current_dir(dir)
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", dir.join("bin").display()),
        )
        .env("TMPDIR", dir.join("tmp"))
        .env("INBOX_URL", "http://127.0.0.1:9")
        .env("INBOX_SECRET_KEY_FILE", "key-file-of-the-case")
        .env("INBOX_OUTBOX", dir.join("state/outbox.jsonl"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command
}

/// A full or over-quota `$TMPDIR`: the file `text` is made (an empty file takes no block) and
/// every write to it fails. The stand-in is a file size limit of 0 with SIGXFSZ ignored, which
/// fails the writes as ENOSPC does. The wake writes into the FIFO, which no limit holds, and marks
/// its row; the hook drops the line it could not keep. A row the wake marked must reach the
/// session, or the hook must say it did not: release 0.6.0 had the wake fail its own write and
/// mark nothing.
#[test]
fn a_full_tmpdir_does_not_drop_a_row_the_wake_marked() {
    let name = "adversary-4-full-tmpdir";
    let base = scratch(name);
    let d = base.to_str().unwrap().to_owned();
    let stand_in = format!(
        "#!/bin/sh\nprintf 'm-1 note Delivered from agent:case/other\\n    Hello from the full disk.\\n' && : > '{d}/marked'\n"
    );
    let dir = setup(name, &stand_in);
    let output = hook(
        &dir,
        "trap '' XFSZ; ulimit -f 0; exec \"$0\" \"$1\" < /dev/null",
    )
    .output()
    .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let marked = dir.join("marked").exists();
    assert!(
        !marked || stderr.contains("Hello from the full disk.") || output.status.code() == Some(1),
        "the wake marked its row and the hook handed on nothing and said nothing: exit {:?}, standard error {stderr:?}",
        output.status.code()
    );
}

/// The shells of [`hook_shells`] and `bash` started by that name: bash as `sh` ends the `read` a
/// trapped signal cut and throws its bytes away; bash by its own name goes on with the `read`
/// after the trap, to the end of the line or of the output, and keeps a cut last line in the
/// variable, as dash (by its source) does when it ends the `read` at the signal. So the case
/// holds each way.
fn signal_shells() -> Vec<PathBuf> {
    let mut shells = hook_shells();
    shells.extend(on_path("bash"));
    shells
}

/// SIGTERM reaches the hook while its `read` waits inside a line (the wake's write of a round
/// over the 64 KiB of the FIFO stopped mid-line, or the wake wrote the line in two pieces). The
/// cut line is dropped whole: the frame holds `m-1 note`, its body line and `m-2 note`, whole
/// lines, and counts 2 messages; no fragment of `    second bo` is in it. A stand-in that
/// finishes its line after the signal (`dy`) puts no fragment of it in the frame either: the line
/// is in it whole (a shell that went on with its `read`) or not at all, and the count stays 2.
/// The real wake is ended by that signal before it marks the round, so the cut message comes
/// again.
#[test]
fn a_signal_inside_a_line_drops_the_cut_line_whole() {
    for shell in signal_shells() {
        for (finish, ending) in [("", "blocks"), ("printf \"dy\\n\"; ", "finishes its line")] {
            let name = format!(
                "adversary-4-signal-inside-a-line-{}-{}",
                shell.file_name().unwrap().to_string_lossy(),
                finish.len()
            );
            let base = scratch(&name);
            let d = base.to_str().unwrap().to_owned();
            let stand_in = format!(
                "#!/bin/sh\ntrap '{finish}exit 143' TERM\nprintf '%s\\n' \"$$\" > '{d}/wake'\n\
                 printf 'm-1 note\\n    first body\\nm-2 note\\n    second bo'\n\
                 : > '{d}/written'\nwhile :; do sleep 0.05; done\n"
            );
            let dir = setup(&name, &stand_in);
            let child = hook_under(&shell, &dir, "exec \"$0\" \"$1\" < /dev/null")
                .spawn()
                .unwrap();
            let start = Instant::now();
            while !dir.join("written").exists() {
                assert!(start.elapsed() < HANG, "the stand-in wrote nothing");
                std::thread::sleep(Duration::from_millis(5));
            }
            std::thread::sleep(Duration::from_millis(300));
            let _ = Command::new("kill")
                .args(["-TERM", &child.id().to_string()])
                .status();
            let output = child.wait_with_output().unwrap();
            let stderr = String::from_utf8_lossy(&output.stderr);
            let what = format!("{}, a stand-in that {ending}: {stderr:?}", shell.display());
            assert_eq!(output.status.code(), Some(2), "{what}");
            assert!(
                stderr.contains(": 2 message(s) follow"),
                "the count is not of the 2 whole head lines: {what}"
            );
            let inside = stderr
                .split_once('\n')
                .map(|(_, rest)| rest)
                .and_then(|rest| rest.rsplit_once("inbox ").map(|(inside, _)| inside))
                .unwrap_or_default();
            assert!(
                inside == "m-1 note\n    first body\nm-2 note\n"
                    || inside == "m-1 note\n    first body\nm-2 note\n    second body\n",
                "the frame is not whole lines the wake wrote, or a fragment of the line the \
                 signal cut is in it: {what}"
            );
        }
    }
}
