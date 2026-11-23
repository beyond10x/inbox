//! The repository's tools that are not synthesis: the scenario checks, the animation writer, the
//! words of the end-user pages, the path decision of the CI job, the test step of the gate and
//! the build settings of `.cargo/config.toml`, and the hook of the Claude Code plugin.
//! Each case runs the binary (or the shell script) on a copy of the parts of the tree it reads.

mod adversary_claude_code_adapter;
mod adversary_claude_code_adapter_2;
mod animation;
mod cargo_config;
mod claude_code_adapter;
mod docs_words;
mod gate_paths;
mod scenarios;
mod test_step;

use std::fs;
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

/// `path` with the first `from` replaced by `to`; fails when `from` is not in it.
pub fn replace(path: &Path, from: &str, to: &str) {
    let text = fs::read_to_string(path).unwrap();
    assert!(text.contains(from), "{from:?} not in {}", path.display());
    fs::write(path, text.replacen(from, to, 1)).unwrap();
}
