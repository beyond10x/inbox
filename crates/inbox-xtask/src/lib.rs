//! Synthesis of `generated/rust/inbox` from `ess/`, and the drift check that holds the checked-in
//! crate to a fresh synthesis. The other tasks of the repository are the modules below.

pub mod animation;
pub mod docs_words;
pub mod scenarios;

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The specification, relative to the repository root.
pub const SPECIFICATION: &str = "ess";
/// The synthesized crate, relative to the repository root.
pub const GENERATED: &str = "generated/rust/inbox";
/// The workspace manifest, a file the repository root holds beside `ess/`.
const MANIFEST: &str = "Cargo.toml";
/// Written by every synthesis with a per-run anchor; never checked in.
const STATE: &str = ".ess-output";
/// The generated crate's own lock file: held by `cargo --locked`, not by synthesis.
const LOCK: &str = "Cargo.lock";
/// A build directory cargo writes when it is run inside the crate; git-ignored.
const BUILD: &str = "target";
/// Top-level entries of the crate directory the comparison does not read.
const IGNORED: [&str; 3] = [STATE, LOCK, BUILD];

/// What a task prints, line by line, and the status it exits with.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Answer {
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
    pub code: u8,
}

/// One way the checked-in crate differs from a fresh synthesis.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Difference {
    /// Both carry the path, with different bytes, or one of them not as a regular file.
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

/// The repository root the tasks run on. With `manifest_dir` (the runtime `CARGO_MANIFEST_DIR`
/// that `cargo run` sets; a relative one is taken from `current`), the directory two levels above
/// it; without, the nearest directory from `current` upwards. Either way the root must hold the
/// directory `ess/` and the file `Cargo.toml`. `generated/rust/` is not required: `generate`
/// creates it.
pub fn repository_root(manifest_dir: Option<&Path>, current: &Path) -> Result<PathBuf, String> {
    let Some(manifest_dir) = manifest_dir else {
        return current
            .ancestors()
            .find(|dir| missing(dir).is_empty())
            .map(Path::to_path_buf)
            .ok_or_else(|| {
                format!(
                    "no directory from {} upwards holds both `{SPECIFICATION}/` and `{MANIFEST}`; run inbox-xtask inside the repository",
                    current.display()
                )
            });
    };
    let manifest_dir = current.join(manifest_dir);
    let root = manifest_dir.ancestors().nth(2).ok_or_else(|| {
        format!(
            "CARGO_MANIFEST_DIR {} has no directory two levels above it",
            manifest_dir.display()
        )
    })?;
    let absent = missing(root);
    if absent.is_empty() {
        Ok(root.to_path_buf())
    } else {
        Err(format!(
            "{} (two levels above CARGO_MANIFEST_DIR) is no repository root: it holds no {}",
            root.display(),
            absent.join(" and no ")
        ))
    }
}

/// What `dir` lacks of a repository root, each named as the messages name it.
fn missing(dir: &Path) -> Vec<String> {
    let mut absent = Vec::new();
    if !dir.join(SPECIFICATION).is_dir() {
        absent.push(format!("`{SPECIFICATION}/`"));
    }
    if !dir.join(MANIFEST).is_file() {
        absent.push(format!("`{MANIFEST}`"));
    }
    absent
}

/// The scratch directory under the repository's own `target/`.
fn scratch(root: &Path, name: &str) -> PathBuf {
    root.join("target").join("inbox-xtask").join(name)
}

/// The program `synthesize`, `generate` and `drift` run: `ess`, found on `PATH`.
pub const ESS: &str = "ess";
/// The arguments `synthesize` passes to `ess`, started in the repository root; the output
/// directory follows them.
pub const SYNTHESIS: [&str; 10] = [
    "generate",
    "synthesize",
    "--strict-requires",
    "--path",
    SPECIFICATION,
    "--target",
    "rust",
    "--layout",
    "crate",
    "--out",
];

/// Run `ess generate synthesize` on `root/ess` into `out`, which is emptied first.
pub fn synthesize(root: &Path, out: &Path) -> Result<(), String> {
    synthesize_with(root, Path::new(ESS), out)
}

/// `synthesize`, running the program `ess` names in place of `ess` from `PATH`.
pub fn synthesize_with(root: &Path, ess: &Path, out: &Path) -> Result<(), String> {
    if out.exists() {
        fs::remove_dir_all(out).map_err(|e| format!("remove {}: {e}", out.display()))?;
    }
    let status = Command::new(ess)
        .current_dir(root)
        .args(SYNTHESIS)
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
/// synthesis state is dropped. On a failed synthesis, or a failed move into place, the checked-in
/// crate is left as it was. A held crate an earlier run left only in `target/inbox-xtask/previous`
/// is moved back first.
pub fn generate(root: &Path) -> Result<(), String> {
    generate_with(root, synthesize)
}

/// `generate`, with `synthesize` (called with the root and the output directory) in place of
/// `synthesize`. The tests pass one that answers the synthesis of the unchanged `ess/` from a
/// cache.
pub fn generate_with(
    root: &Path,
    synthesize: impl Fn(&Path, &Path) -> Result<(), String>,
) -> Result<(), String> {
    let target = root.join(GENERATED);
    let aside = scratch(root, "previous");
    recover(&target, &aside)?;
    let staged = scratch(root, "generate");
    synthesize(root, &staged)?;
    let state = staged.join(STATE);
    if state.exists() {
        fs::remove_dir_all(&state).map_err(|e| format!("remove {}: {e}", state.display()))?;
    }
    let lock = target.join(LOCK);
    if lock.exists() {
        fs::copy(&lock, staged.join(LOCK)).map_err(|e| format!("copy {}: {e}", lock.display()))?;
    }
    replace(&staged, &target, &aside, |from, to| fs::rename(from, to))
}

/// When `target` is absent and `aside` exists, an earlier run moved the held crate aside and could
/// not move it back: put it back before anything reads `target` or removes `aside`.
fn recover(target: &Path, aside: &Path) -> Result<(), String> {
    if target.symlink_metadata().is_ok() || aside.symlink_metadata().is_err() {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    fs::rename(aside, target).map_err(|e| {
        format!(
            "move the held crate back from {} to {}: {e}; nothing else was changed",
            aside.display(),
            target.display()
        )
    })
}

/// Put `staged` at `target`: the held `target` is moved to `aside` first, `staged` takes its place,
/// and `aside` is removed only after that succeeded. When the second move fails, `aside` is moved
/// back; when the first fails, nothing has changed.
fn replace(
    staged: &Path,
    target: &Path,
    aside: &Path,
    mut rename: impl FnMut(&Path, &Path) -> io::Result<()>,
) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    if aside.symlink_metadata().is_ok() {
        fs::remove_dir_all(aside).map_err(|e| format!("remove {}: {e}", aside.display()))?;
    }
    let held = target.symlink_metadata().is_ok();
    if held {
        rename(target, aside).map_err(|e| {
            format!(
                "move {} aside to {}: {e}; {} is unchanged",
                target.display(),
                aside.display(),
                target.display()
            )
        })?;
    }
    if let Err(e) = rename(staged, target) {
        let failed = format!("move {} to {}: {e}", staged.display(), target.display());
        if !held {
            return Err(failed);
        }
        return match rename(aside, target) {
            Ok(()) => Err(format!("{failed}; {} is restored", target.display())),
            Err(back) => Err(format!(
                "{failed}; moving the held crate back from {} failed too: {back}; the next `task generate` moves it back first",
                aside.display()
            )),
        };
    }
    if held {
        fs::remove_dir_all(aside).map_err(|e| format!("remove {}: {e}", aside.display()))?;
    }
    Ok(())
}

/// Compare `generated/rust/inbox` with a fresh synthesis. `Ok` carries the one-line verdict;
/// `Err` names `generated/rust/inbox` and every difference.
pub fn drift(root: &Path) -> Result<String, String> {
    drift_with(root, synthesize)
}

/// `drift`, with `synthesize` in place of `synthesize` (see `generate_with`).
pub fn drift_with(
    root: &Path,
    synthesize: impl Fn(&Path, &Path) -> Result<(), String>,
) -> Result<String, String> {
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

/// What a path below a crate directory is. Read with `symlink_metadata`: a link is never followed.
#[derive(Debug, PartialEq, Eq)]
enum Entry {
    /// A regular file, and whether any executable bit (0o111) is set: the one mode bit git
    /// records. The other bits follow the umask of the checkout and are not read.
    File {
        executable: bool,
    },
    Link(PathBuf),
    /// Anything else that is not a directory: a socket, a FIFO, a device.
    Other,
}

/// Every path that differs between the two crate directories, sorted by path. The synthesis
/// state, the lock file and a build directory at the top of either directory are not read. A
/// path that is a symbolic link or no regular file on one side only, or a file whose executable
/// bit differs, is `Edited`.
pub fn differences(fresh: &Path, held: &Path) -> Result<Vec<Difference>, String> {
    let written = entries(fresh)?;
    let kept = entries(held)?;
    let mut paths: Vec<&PathBuf> = written.keys().chain(kept.keys()).collect();
    paths.sort();
    paths.dedup();
    let mut found = Vec::new();
    for path in paths {
        let same = match (written.get(path), kept.get(path)) {
            (Some(_), None) => {
                found.push(Difference::Missing(path.clone()));
                continue;
            }
            (None, Some(_)) => {
                found.push(Difference::NotGenerated(path.clone()));
                continue;
            }
            (Some(Entry::File { executable: a }), Some(Entry::File { executable: b })) => {
                a == b && read(&fresh.join(path))? == read(&held.join(path))?
            }
            (Some(Entry::Link(a)), Some(Entry::Link(b))) => a == b,
            _ => false,
        };
        if !same {
            found.push(Difference::Edited(path.clone()));
        }
    }
    Ok(found)
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))
}

/// The non-directory entries under `dir`, relative to it; an absent directory has none, and a
/// `dir` that is itself a symbolic link is refused.
fn entries(dir: &Path) -> Result<BTreeMap<PathBuf, Entry>, String> {
    let mut found = BTreeMap::new();
    match dir.symlink_metadata() {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(found),
        Err(e) => return Err(format!("read {}: {e}", dir.display())),
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(format!(
                "{} is a symbolic link, not a directory",
                dir.display()
            ));
        }
        Ok(_) => {}
    }
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let listing =
            fs::read_dir(&current).map_err(|e| format!("read {}: {e}", current.display()))?;
        for item in listing {
            let item = item.map_err(|e| format!("read {}: {e}", current.display()))?;
            let path = item.path();
            let relative = path.strip_prefix(dir).map_err(|e| e.to_string())?;
            if IGNORED.iter().any(|ignored| relative == Path::new(ignored)) {
                continue;
            }
            let meta =
                fs::symlink_metadata(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
            let kind = meta.file_type();
            let entry = if kind.is_dir() {
                pending.push(path);
                continue;
            } else if kind.is_symlink() {
                Entry::Link(
                    fs::read_link(&path).map_err(|e| format!("read {}: {e}", path.display()))?,
                )
            } else if kind.is_file() {
                Entry::File {
                    executable: meta.permissions().mode() & 0o111 != 0,
                }
            } else {
                Entry::Other
            };
            found.insert(relative.to_path_buf(), entry);
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh directory under the repository's own `target/`.
    fn scratch_dir(name: &str) -> PathBuf {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/inbox-xtask-unit")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn layout(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let dir = scratch_dir(name);
        let staged = dir.join("staged");
        let target = dir.join("generated/rust/inbox");
        fs::create_dir_all(&staged).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(staged.join("lib.rs"), "new\n").unwrap();
        fs::write(target.join("lib.rs"), "held\n").unwrap();
        (staged, target, dir.join("previous"))
    }

    #[test]
    fn replace_puts_the_staged_crate_in_place_and_drops_the_held_one() {
        let (staged, target, aside) = layout("replace-ok");
        replace(&staged, &target, &aside, |a, b| fs::rename(a, b)).unwrap();
        assert_eq!(fs::read_to_string(target.join("lib.rs")).unwrap(), "new\n");
        assert!(!aside.exists());
        assert!(!staged.exists());
    }

    #[test]
    fn a_failed_move_into_place_restores_the_held_crate() {
        let (staged, target, aside) = layout("replace-restores");
        let mut calls = 0;
        let error = replace(&staged, &target, &aside, |a, b| {
            calls += 1;
            if calls == 2 {
                Err(io::Error::new(io::ErrorKind::CrossesDevices, "injected"))
            } else {
                fs::rename(a, b)
            }
        })
        .unwrap_err();
        assert!(error.ends_with("is restored"), "{error}");
        assert_eq!(fs::read_to_string(target.join("lib.rs")).unwrap(), "held\n");
        assert_eq!(fs::read_to_string(staged.join("lib.rs")).unwrap(), "new\n");
    }

    #[test]
    fn a_failed_move_aside_changes_nothing() {
        let (staged, target, aside) = layout("replace-aside-fails");
        let error = replace(&staged, &target, &aside, |_, _| {
            Err(io::Error::new(io::ErrorKind::CrossesDevices, "injected"))
        })
        .unwrap_err();
        assert!(error.ends_with("is unchanged"), "{error}");
        assert_eq!(fs::read_to_string(target.join("lib.rs")).unwrap(), "held\n");
        assert!(!aside.exists());
    }
}
