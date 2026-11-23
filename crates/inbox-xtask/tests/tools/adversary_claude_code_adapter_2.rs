//! Adversary pass 2 on the hook of the Claude Code plugin, `adapters/claude-code/scripts/wake.sh`.
//! Each case runs the script with `sh` under a stand-in `inbox` (a shell script the case writes)
//! on a `PATH` of its own, as `claude_code_adapter.rs` does. No case starts Claude Code or the real
//! `inbox`.

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::{each_shell, hook_shell, hook_shell_name, repository, scratch};

const UTILITIES: [&str; 13] = [
    "cat", "cut", "grep", "mkdir", "mkfifo", "od", "ps", "rm", "sed", "sleep", "tr", "wc", "ln",
];

/// The head line of a message as the printer writes it, with the sender `from`.
fn head(at: usize, from: &str) -> String {
    format!(
        "m-{at} note Sent from {from} to agent:case/session thread t-{at} expires 2026-11-24T00:00:00Z"
    )
}

fn found(name: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|dir| dir.join(name))
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("{name} is not on PATH"))
}

struct Case {
    dir: PathBuf,
    bin: PathBuf,
    tmp: PathBuf,
    outbox: PathBuf,
}

struct Ran {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Case {
    /// A session whose inbox was opened with `claude_code`, its stand-in `inbox` running `end`.
    fn new(name: &str, end: &str) -> Self {
        let dir = scratch(&format!(
            "claude-code-adversary-2-{name}-{}",
            hook_shell_name()
        ));
        let bin = dir.join("bin");
        let tmp = dir.join("tmp");
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(&tmp).unwrap();
        for utility in UTILITIES {
            symlink(found(utility), bin.join(utility)).unwrap();
        }
        let outbox = dir.join("state/outbox.jsonl");
        fs::create_dir_all(outbox.parent().unwrap()).unwrap();
        fs::write(
            dir.join("state/outbox.jsonl.opened"),
            "{\"inbox\":\"agent:case/session\",\"adapter\":\"claude_code\"}\n",
        )
        .unwrap();
        let inbox = bin.join("inbox");
        fs::write(&inbox, format!("#!/bin/sh\n{end}\n")).unwrap();
        fs::set_permissions(&inbox, fs::Permissions::from_mode(0o755)).unwrap();
        Self {
            dir,
            bin,
            tmp,
            outbox,
        }
    }

    /// A stand-in that prints the file `rows` of the case and exits with `status`.
    fn printing(name: &str, rows: &str, status: i32) -> Self {
        let dir = scratch(&format!(
            "claude-code-adversary-2-{name}-rows-{}",
            hook_shell_name()
        ));
        let file = dir.join("rows");
        fs::write(&file, rows).unwrap();
        Self::new(name, &format!("cat '{}'\nexit {status}", file.display()))
    }

    /// The hook started by `sh`, with `extra` variables beside those of the session.
    fn run_with(&self, command: Command, extra: &[(&str, &str)]) -> Ran {
        let mut command = command;
        command
            .current_dir(&self.dir)
            .env_clear()
            .env("PATH", &self.bin)
            .env("TMPDIR", &self.tmp)
            .env("INBOX_URL", "http://127.0.0.1:9")
            .env("INBOX_SECRET_KEY_FILE", "key-file-of-the-case")
            .env("INBOX_OUTBOX", &self.outbox)
            .env("HOME", self.dir.join("home"))
            .stdin(Stdio::null());
        for (name, value) in extra {
            command.env(name, value);
        }
        let output = command.output().unwrap();
        Ran {
            code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    fn run(&self, extra: &[(&str, &str)]) -> Ran {
        let mut command = Command::new(hook_shell());
        command.arg(script());
        self.run_with(command, extra)
    }
}

fn script() -> PathBuf {
    repository().join("adapters/claude-code/scripts/wake.sh")
}

/// `docs/design/wake.md`, "The size": "GNU `cut -c` counts bytes, so there it is also at most
/// 8,000 bytes". GNU coreutils 9.12 counts characters in a UTF-8 locale, the locale a terminal
/// hands Claude Code and its hooks: 20 head lines of 300 characters of 2 bytes each are 12,000
/// bytes, and the frame is over the bound the design states.
#[test]
fn head_lines_of_wide_characters_keep_the_frame_within_8000_bytes() {
    each_shell(|| {
        let wide = "\u{e4}".repeat(400);
        let mut rows = String::new();
        for at in 0..25 {
            rows.push_str(&format!(
                "{}\n    body\n",
                head(at, &format!("agent:case/{wide}"))
            ));
        }
        assert!(rows.len() > 6000);
        let case = Case::printing("wide", &rows, 0);
        let ran = case.run(&[("LC_ALL", "C.UTF-8")]);
        assert_eq!(ran.code, Some(2), "{}", ran.stderr);
        assert!(
            ran.stderr.len() <= 8000,
            "the frame is {} bytes, over the 8,000 bytes of docs/design/wake.md; its longest line is {} bytes",
            ran.stderr.len(),
            ran.stderr.lines().map(str::len).max().unwrap_or(0)
        );
    });
}

/// The released `inbox` 0.4.0 has no `wake` command: clap refuses it with exit 2, as wrong
/// usage. The hook then tells the operator that `INBOX_URL` is not a loopback URL or that the pin
/// does not go with it, which is not the cause.
#[test]
fn an_inbox_without_the_wake_command_is_not_reported_as_a_wrong_url() {
    each_shell(|| {
        let case = Case::new(
            "no-wake-command",
            "printf '%s\\n' \"error: unrecognized subcommand 'wake'\" >&2\nexit 2",
        );
        let ran = case.run(&[]);
        assert_eq!(ran.code, Some(1), "{}{}", ran.stdout, ran.stderr);
        assert_eq!(ran.stdout, "");
        assert!(
            ran.stderr.contains("may have no `wake` command"),
            "an `inbox` without `wake` is reported as a wrong URL: {}",
            ran.stderr
        );
    });
}

/// The design: when a name exists, the next one, `….<pid>.1` up to `.99`. With 5 names taken the
/// hook still makes its directory and wakes the session.
#[test]
fn five_names_taken_are_passed_over() {
    each_shell(|| {
        let case = Case::printing(
            "five-names",
            &format!("{}\n    body\n", head(1, "agent:case/other")),
            0,
        );
        let mut command = Command::new(hook_shell());
        command.args([
        "-c",
        "for n in '' .1 .2 .3 .4; do mkdir \"$TMPDIR/inbox-claude-code.$$$n\" || exit 9; done; exec \"$1\" \"$0\"",
        script().to_str().unwrap(),
        hook_shell().to_str().unwrap(),
    ]);
        let ran = case.run_with(command, &[]);
        assert_eq!(ran.code, Some(2), "{}", ran.stderr);
        let left = fs::read_dir(&case.tmp).unwrap().count();
        assert_eq!(
            left, 5,
            "the hook left a directory or removed one it did not make"
        );
    });
}

/// Exactly 6,001 bytes of rows are handed on as head lines, and exactly 6,000 whole: the bound
/// is `-gt`.
#[test]
fn rows_of_6001_bytes_are_handed_on_as_head_lines() {
    each_shell(|| {
        let first = head(1, "agent:case/other");
        let filler = 6001 - first.len() - 1 - 4 - 1;
        let rows = format!("{first}\n    {}\n", "z".repeat(filler));
        assert_eq!(rows.len(), 6001);
        let case = Case::printing("6001", &rows, 0);
        let ran = case.run(&[]);
        assert_eq!(ran.code, Some(2), "{}", ran.stderr);
        assert!(
            !ran.stderr.contains("zzzz"),
            "a body over the bound was handed on"
        );
        assert!(
            ran.stderr.contains(&format!("\n{first}\n")),
            "{}",
            ran.stderr
        );
    });
}
