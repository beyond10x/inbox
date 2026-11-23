//! Adversary cases of `adapters/claude-code/scripts/wake.sh` for
//! story:a-wake-whose-reader-is-gone-marks-nothing: a post that arrives after the harness is gone
//! and before the watch of the hook's parent looked. Each case runs the script with `sh` under a
//! stand-in `inbox`, as `claude_code_adapter.rs` does; none starts Claude Code or the real
//! `inbox`.

use std::fs;
use std::io::Write;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::{each_shell, hook_shell, hook_shell_name, repository, scratch};

const HANG: Duration = Duration::from_secs(10);

const ROWS: &str = "m-1 note Delivered from agent:case/other to agent:case/session thread t-1 expires 2026-11-24T00:00:00Z\n    Hello.\n";

const UTILITIES: [&str; 13] = [
    "cat", "cut", "grep", "mkdir", "mkfifo", "od", "ps", "rm", "sed", "sleep", "stat", "tr", "wc",
];

fn found(name: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|dir| dir.join(name))
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("{name} is not on PATH"))
}

fn alive(pid: &str) -> bool {
    !fs::read_to_string(Path::new("/proc").join(pid).join("stat"))
        .unwrap_or_else(|_| "x) Z".to_owned())
        .split(") ")
        .nth(1)
        .is_some_and(|rest| rest.starts_with('Z'))
}

fn pid_in(dir: &Path, name: &str) -> String {
    let start = Instant::now();
    loop {
        if let Ok(pid) = fs::read_to_string(dir.join(name))
            && pid.ends_with('\n')
        {
            return pid.trim().to_owned();
        }
        assert!(start.elapsed() < HANG, "the stand-in wrote no {name}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Waits until `fact` holds and answers whether it did; [`HANG`] only ends a hang.
fn within(mut fact: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while !fact() {
        if start.elapsed() > HANG {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    true
}

/// A shell loop of the stand-ins that waits for the file `name` in `dir`, 10 s at most.
fn waiting_for(dir: &str, name: &str) -> String {
    format!(
        "n=0\nwhile [ ! -e '{dir}/{name}' ] && [ $n -lt 200 ]; do sleep 0.05; n=$((n + 1)); done\n"
    )
}

/// Claude Code crashes, and a post arrives right after, before the watch of the hook's parent
/// (every 2 s) looked: the hook, alive, reads the rows from its FIFO, so the wake marks the post
/// delivered, and the hook's write to the harness reaches no one. An open limit
/// (`website/docs/limits.md`): closing it needs an acknowledgement from the reader. A stand-in
/// `ps` holds the watch's first look until the post is marked, so the case does not depend on how
/// soon the post comes. Then the watch looks, finds another parent and ends the hook and its
/// wake: a post after that is not marked.
#[test]
fn a_post_right_after_the_harness_is_gone_is_marked_an_open_limit() {
    each_shell(|| {
        let dir = scratch(&format!(
            "adversary-3-harness-gone-post-{}",
            hook_shell_name()
        ));
        let bin = dir.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(dir.join("tmp")).unwrap();
        for utility in UTILITIES {
            if utility != "ps" {
                symlink(found(utility), bin.join(utility)).unwrap();
            }
        }
        fs::create_dir_all(dir.join("state")).unwrap();
        fs::write(
            dir.join("state/outbox.jsonl.opened"),
            "{\"inbox\":\"agent:case/session\",\"adapter\":\"claude_code\"}\n",
        )
        .unwrap();
        let d = dir.to_str().unwrap();
        // The stand-in of a wake: waits for a post, prints it, and marks it once the write
        // succeeded, as `inbox wake` does; then the same for a second post.
        let stand_in = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$$\" > '{d}/started'\n{}\
             printf '%s' '{ROWS}' && printf 'marked\\n' > '{d}/marked'\n{}\
             printf '%s' '{ROWS}' && printf 'marked\\n' > '{d}/marked-2'\n",
            waiting_for(d, "post"),
            waiting_for(d, "post-2"),
        );
        fs::write(bin.join("inbox"), stand_in).unwrap();
        fs::set_permissions(bin.join("inbox"), fs::Permissions::from_mode(0o755)).unwrap();
        // The stand-in of `ps`: the watch's look waits for the case's word, then is the real one.
        let ps = format!(
            "#!/bin/sh\n{}exec '{}' \"$@\"\n",
            waiting_for(d, "look"),
            found("ps").display()
        );
        fs::write(bin.join("ps"), ps).unwrap();
        fs::set_permissions(bin.join("ps"), fs::Permissions::from_mode(0o755)).unwrap();

        let shell = hook_shell();
        let script = repository().join("adapters/claude-code/scripts/wake.sh");
        let mut command = Command::new(&shell);
        command
            .args([
                "-c",
                "\"$0\" \"$1\" < /dev/null & printf '%s\\n' \"$!\" > hook; wait",
                shell.to_str().unwrap(),
                script.to_str().unwrap(),
            ])
            .current_dir(&dir)
            .env_clear()
            .env("PATH", &bin)
            .env("TMPDIR", dir.join("tmp"))
            .env("INBOX_URL", "http://127.0.0.1:9")
            .env("INBOX_SECRET_KEY_FILE", "key-file-of-the-case")
            .env("INBOX_OUTBOX", dir.join("state/outbox.jsonl"))
            .env("HOME", dir.join("home"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut harness = command.spawn().unwrap();
        let _ = harness.stdin.take().unwrap().write_all(b"{}");
        let hook = pid_in(&dir, "hook");
        let wake = pid_in(&dir, "started");
        drop(harness.stdout.take());
        drop(harness.stderr.take());
        harness.kill().unwrap();
        let _ = harness.wait();
        // A post arrives now: the harness is gone and sent the hook no signal.
        fs::write(dir.join("post"), "").unwrap();
        let marked = within(|| dir.join("marked").exists());
        // The watch looks now, finds another parent and ends the hook, which ends its wake.
        fs::write(dir.join("look"), "").unwrap();
        let ended = within(|| !alive(&hook) && !alive(&wake));
        fs::write(dir.join("post-2"), "").unwrap();
        let marked_after = dir.join("marked-2").exists();
        let _ = Command::new("kill").args(["-9", &wake]).status();
        let _ = Command::new("kill").args(["-9", &hook]).status();
        // Turns when the reader acknowledges what it took before the wake marks it.
        assert!(
            marked,
            "a post right after the harness was gone is no longer marked: turn this case"
        );
        assert!(
            ended,
            "the watch did not end the hook and its wake once it looked: hook {} wake {}",
            alive(&hook),
            alive(&wake)
        );
        assert!(
            !marked_after,
            "a post after the watch ended the hook was marked"
        );
    });
}
