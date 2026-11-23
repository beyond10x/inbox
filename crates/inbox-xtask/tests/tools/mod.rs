//! The repository's tools that are not synthesis: the scenario checks, the animation writer, the
//! words of the end-user pages, the path decision of the CI job, the test step of the gate and
//! the build settings of `.cargo/config.toml`, and the hook of the Claude Code plugin.
//! Each case runs the binary (or the shell script) on a copy of the parts of the tree it reads.

mod adversary_claude_code_adapter;
mod adversary_claude_code_adapter_2;
mod adversary_claude_code_adapter_3;
mod adversary_claude_code_adapter_4;
mod animation;
mod cargo_config;
mod claude_code_adapter;
mod docs_words;
mod gate_paths;
mod scenarios;
mod test_step;

use std::cell::RefCell;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

pub fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("tools")
        .join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// A repository root of its own (`ess/` beside a `Cargo.toml`) holding copies of `parts`, paths
/// relative to the repository: directories or files.
pub fn tree(name: &str, parts: &[&str]) -> PathBuf {
    let root = scratch(name);
    fs::write(root.join("Cargo.toml"), "[workspace]\n").unwrap();
    fs::create_dir_all(root.join("ess")).unwrap();
    for part in parts {
        let from = repository().join(part);
        let to = root.join(part);
        if from.is_dir() {
            copy_tree(&from, &to);
        } else {
            fs::create_dir_all(to.parent().unwrap()).unwrap();
            fs::copy(&from, &to).unwrap();
        }
    }
    root
}

pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    pub fn stdout_lines(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }
}

fn finish(command: &mut Command) -> Run {
    let output = command.output().unwrap();
    Run {
        code: output.status.code().unwrap(),
        stdout: String::from_utf8(output.stdout).unwrap(),
        stderr: String::from_utf8(output.stderr).unwrap(),
    }
}

/// `inbox-xtask <args>` started in `root`, as a task starts it outside cargo.
pub fn xtask(root: &Path, args: &[&str]) -> Run {
    finish(
        Command::new(env!("CARGO_BIN_EXE_inbox-xtask"))
            .args(args)
            .env_remove("CARGO_MANIFEST_DIR")
            .current_dir(root),
    )
}

/// `sh <script> <args>` started in `dir`.
pub fn shell(dir: &Path, script: &Path, args: &[&str]) -> Run {
    finish(Command::new("sh").arg(script).args(args).current_dir(dir))
}

/// The first file named `name` on the `PATH` of the test.
pub fn on_path(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(name))
        .find(|path| path.is_file())
}

/// The shells the cases of the Claude Code hook run it under: `sh`, and `dash` as well when it is
/// on `PATH` and is not what `sh` is. The forge's `sh` is dash; a host whose `sh` is another
/// shell covers dash only with `dash` on `PATH`.
pub fn hook_shells() -> Vec<PathBuf> {
    let sh = on_path("sh").expect("sh is not on PATH");
    let mut shells = vec![sh.clone()];
    if let Some(dash) = on_path("dash")
        && fs::canonicalize(&dash).ok() != fs::canonicalize(&sh).ok()
    {
        shells.push(dash);
    }
    shells
}

thread_local! {
    static HOOK_SHELL: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// The shell the running case starts the hook with: the one [`each_shell`] runs it under, else
/// `sh`.
pub fn hook_shell() -> PathBuf {
    HOOK_SHELL
        .with(|shell| shell.borrow().clone())
        .unwrap_or_else(|| on_path("sh").expect("sh is not on PATH"))
}

/// The file name of [`hook_shell`], for the names of a case's own directories.
pub fn hook_shell_name() -> String {
    hook_shell()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

/// Runs `case` once under each of [`hook_shells`]; a failure names the shell it failed under.
pub fn each_shell(case: impl Fn()) {
    for shell in hook_shells() {
        HOOK_SHELL.with(|held| *held.borrow_mut() = Some(shell.clone()));
        let result = panic::catch_unwind(AssertUnwindSafe(&case));
        HOOK_SHELL.with(|held| *held.borrow_mut() = None);
        if let Err(payload) = result {
            let message = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| {
                    payload
                        .downcast_ref::<&str>()
                        .map(|text| (*text).to_owned())
                })
                .unwrap_or_default();
            panic!("under {}: {message}", shell.display());
        }
    }
}

/// `path` with the first `from` replaced by `to`; fails when `from` is not in it.
pub fn replace(path: &Path, from: &str, to: &str) {
    let text = fs::read_to_string(path).unwrap();
    assert!(text.contains(from), "{from:?} not in {}", path.display());
    fs::write(path, text.replacen(from, to, 1)).unwrap();
}
