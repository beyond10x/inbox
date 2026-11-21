//! Synthesis of `generated/rust/inbox` from `ess/`, and the drift check that holds the checked-in
//! crate to a fresh synthesis.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The specification, relative to the repository root.
pub const SPECIFICATION: &str = "ess";
/// The synthesized crate, relative to the repository root.
pub const GENERATED: &str = "generated/rust/inbox";
/// Written by every synthesis with a per-run anchor; never checked in.
const STATE: &str = ".ess-output";
/// The generated crate's own lock file: held by `cargo --locked`, not by synthesis.
const LOCK: &str = "Cargo.lock";
/// Top-level entries of the crate directory the comparison does not read.
const IGNORED: [&str; 2] = [STATE, LOCK];

/// One way the checked-in crate differs from a fresh synthesis.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Difference {
    /// Both carry the file, with different bytes.
    Edited(PathBuf),
    /// The synthesis writes the file; the checked-in crate lacks it.
    Missing(PathBuf),
    /// The checked-in crate carries a file the synthesis no longer writes.
    NotGenerated(PathBuf),
}

impl fmt::Display for Difference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Difference::Edited(path) => write!(f, "edited: {}", path.display()),
            Difference::Missing(path) => write!(f, "missing: {}", path.display()),
            Difference::NotGenerated(path) => write!(f, "not generated: {}", path.display()),
        }
    }
}

/// The scratch directory under the repository's own `target/`.
fn scratch(root: &Path, name: &str) -> PathBuf {
    root.join("target").join("inbox-xtask").join(name)
}

/// Run `ess generate synthesize` on `root/ess` into `out`, which is emptied first.
pub fn synthesize(root: &Path, out: &Path) -> Result<(), String> {
    if out.exists() {
        fs::remove_dir_all(out).map_err(|e| format!("remove {}: {e}", out.display()))?;
    }
    let status = Command::new("ess")
        .current_dir(root)
        .args(["generate", "synthesize", "--strict-requires"])
        .args(["--path", SPECIFICATION])
        .args(["--target", "rust", "--layout", "crate", "--out"])
        .arg(out)
        .status()
        .map_err(|e| format!("run ess: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("ess generate synthesize exited with {status}"))
    }
}

/// Rewrite `generated/rust/inbox` from a fresh synthesis. The crate's `Cargo.lock` is kept and the
/// synthesis state is dropped; on a failed synthesis the checked-in crate is left as it was.
pub fn generate(root: &Path) -> Result<(), String> {
    let staged = scratch(root, "generate");
    synthesize(root, &staged)?;
    let state = staged.join(STATE);
    if state.exists() {
        fs::remove_dir_all(&state).map_err(|e| format!("remove {}: {e}", state.display()))?;
    }
    let target = root.join(GENERATED);
    let lock = target.join(LOCK);
    if lock.exists() {
        fs::copy(&lock, staged.join(LOCK)).map_err(|e| format!("copy {}: {e}", lock.display()))?;
    }
    if target.exists() {
        fs::remove_dir_all(&target).map_err(|e| format!("remove {}: {e}", target.display()))?;
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    fs::rename(&staged, &target)
        .map_err(|e| format!("move {} to {}: {e}", staged.display(), target.display()))
}

/// Compare `generated/rust/inbox` with a fresh synthesis. `Ok` carries the one-line verdict;
/// `Err` names `generated/rust/inbox` and every difference.
pub fn drift(root: &Path) -> Result<String, String> {
    let fresh = scratch(root, "drift");
    synthesize(root, &fresh)?;
    let found = differences(&fresh, &root.join(GENERATED))?;
    if found.is_empty() {
        Ok(format!(
            "{GENERATED}: no drift from a fresh synthesis of {SPECIFICATION}/"
        ))
    } else {
        let lines: Vec<String> = found.iter().map(ToString::to_string).collect();
        Err(format!(
            "{GENERATED} drifted from a fresh synthesis of {SPECIFICATION}/ (run `task generate`):\n  {}",
            lines.join("\n  ")
        ))
    }
}

/// Every file that differs between the two crate directories, sorted by path. The synthesis state
/// and the lock file at the top of either directory are not read.
pub fn differences(fresh: &Path, held: &Path) -> Result<Vec<Difference>, String> {
    let written = files(fresh)?;
    let kept = files(held)?;
    let mut found = Vec::new();
    for path in written.union(&kept) {
        match (written.contains(path), kept.contains(path)) {
            (true, false) => found.push(Difference::Missing(path.clone())),
            (false, true) => found.push(Difference::NotGenerated(path.clone())),
            _ => {
                let a = read(&fresh.join(path))?;
                let b = read(&held.join(path))?;
                if a != b {
                    found.push(Difference::Edited(path.clone()));
                }
            }
        }
    }
    Ok(found)
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))
}

/// The files under `dir`, relative to it; an absent directory has none.
fn files(dir: &Path) -> Result<BTreeSet<PathBuf>, String> {
    let mut found = BTreeSet::new();
    if !dir.exists() {
        return Ok(found);
    }
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries =
            fs::read_dir(&current).map_err(|e| format!("read {}: {e}", current.display()))?;
        for entry in entries {
            let path = entry.map_err(|e| e.to_string())?.path();
            let relative = path.strip_prefix(dir).map_err(|e| e.to_string())?;
            if IGNORED.iter().any(|ignored| relative == Path::new(ignored)) {
                continue;
            }
            if path.is_dir() {
                pending.push(path);
            } else {
                found.insert(relative.to_path_buf());
            }
        }
    }
    Ok(found)
}
