//! The gate fails on a breaking change of the specification.
//!
//! `spec-compat` compares the tree's `ess/` with the one of the base revision through
//! `ess verify diff --compatibility --fail-on breaking-or-unknown` and fails on a change that is
//! breaking or of unknown compatibility in any of the 3 dimensions. The base is the commit
//! `INBOX_SPEC_BASE` names when it is set and not empty (CI names it), else the merge base of
//! `HEAD` with `origin/main`, else with `main`. When the files under `ess/` that git lists have
//! the paths and, on disk and unfiltered, the bytes of the base's, nothing is compared. Otherwise
//! the base's `ess/` is written into `.out/spec-compat/base/` and compared; an acknowledged change
//! is written in [`ACKNOWLEDGEMENTS`]. It fails closed: a base it cannot find, a `git` or an `ess`
//! that fails, an entry under `ess/` that git is told not to look at, and an `ess` that exits 0
//! without exactly one verdict line and one line of the pass, or with more breaking and unknown
//! changes than it says it acknowledged, each fail the step. Every path in a line of the step is
//! relative to the root.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use crate::{Answer, ESS, SPECIFICATION};

/// The one place of an acknowledged breaking change: an `ess-diff-acknowledgements/1` document,
/// relative to the repository root. Absent while nothing is acknowledged.
pub const ACKNOWLEDGEMENTS: &str = ".config/ess-diff-acknowledgements.json";
/// The variable that names the base revision.
pub const BASE_VARIABLE: &str = "INBOX_SPEC_BASE";
/// Where the base's `ess/` is written, relative to the repository root.
const BASE_COPY: &str = ".out/spec-compat/base";
/// The refs whose merge base with `HEAD` is the base when no revision is named, in order.
const MAIN: [(&str, &str); 2] = [
    ("refs/remotes/origin/main", "origin/main"),
    ("refs/heads/main", "main"),
];
/// How many hex digits of a commit the lines show.
const SHORT: usize = 12;

/// One line a task prints, on its stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    Out(String),
    Err(String),
}

/// What a task prints, in order, and the status it exits with.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Said {
    pub lines: Vec<Line>,
    pub code: u8,
}

impl From<Answer> for Said {
    fn from(answer: Answer) -> Said {
        let mut lines: Vec<Line> = answer.stdout.into_iter().map(Line::Out).collect();
        lines.extend(answer.stderr.into_iter().map(Line::Err));
        Said {
            lines,
            code: answer.code,
        }
    }
}

/// Compare `root/ess` with the `ess/` of the base; `named` is the value of [`BASE_VARIABLE`].
/// Exit 0 with one line when nothing differs, or with `ess`'s report and then the step's line
/// with its verdict; exit 1 with `ess`'s lines, when it wrote any, and then one line that names
/// the cause.
pub fn spec_compat(root: &Path, named: Option<&str>) -> Said {
    let mut lines = Vec::new();
    match compare(root, named, &mut lines) {
        Ok(()) => Said { lines, code: 0 },
        Err(cause) => {
            lines.push(Line::Err(format!(
                "spec-compat: {}",
                relative(&cause, root)
            )));
            Said { lines, code: 1 }
        }
    }
}

/// `cause` with every path under `root` written relative to it: a line of the step holds no
/// path of the host, also where it quotes `git` or the system.
fn relative(cause: &str, root: &Path) -> String {
    let mut roots = vec![root.to_path_buf()];
    if let Ok(real) = root.canonicalize()
        && real != root
    {
        roots.push(real);
    }
    let mut cause = cause.to_string();
    for root in roots {
        let root = root.display().to_string();
        let root = root.trim_end_matches('/');
        if root.is_empty() {
            continue;
        }
        cause = cause.replace(&format!("{root}/"), "");
        // The root itself, where no longer name goes on from it.
        let mut rest = cause.as_str();
        let mut done = String::new();
        while let Some(at) = rest.find(root) {
            let after = &rest[at + root.len()..];
            let ends = after
                .chars()
                .next()
                .is_none_or(|c| !(c.is_alphanumeric() || "-_.~".contains(c)));
            done.push_str(&rest[..at]);
            done.push_str(if ends { "." } else { root });
            rest = after;
        }
        done.push_str(rest);
        cause = done;
    }
    cause
}

fn compare(root: &Path, named: Option<&str>, lines: &mut Vec<Line>) -> Result<(), String> {
    let base = base(root, named)?;
    let short = &base[..SHORT.min(base.len())];
    // An entry git is told not to look at hides its edit from `git diff`: such an `ess/` is
    // compared, so that `ess` shows the change, and the step fails on the mark whatever `ess`
    // says.
    let marked = marked(root)?;
    if marked.is_none() && equal(root, &base)? {
        lines.push(Line::Out(format!(
            "spec-compat: ess/ equals the one of {short}; nothing compared"
        )));
        return Ok(());
    }
    let from = write_base(root, &base, short)?;
    let acknowledged = root.join(ACKNOWLEDGEMENTS).symlink_metadata().is_ok();
    let mut command = Command::new(ESS);
    command.current_dir(root).args(["verify", "diff", "--from"]);
    command.arg(&from).args([
        "--to",
        SPECIFICATION,
        "--compatibility",
        "--fail-on",
        "breaking-or-unknown",
    ]);
    if acknowledged {
        command.args(["--acknowledgements", ACKNOWLEDGEMENTS]);
    }
    let output = command
        .output()
        .map_err(|e| format!("ess cannot be run: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    lines.extend(stdout.lines().map(|line| Line::Out(line.to_string())));
    lines.extend(stderr.lines().map(|line| Line::Err(line.to_string())));
    let judged = judge(&output, acknowledged, short, &stdout, &stderr);
    if let Some(mark) = marked {
        return Err(mark);
    }
    let verdict = judged?;
    lines.push(Line::Out(format!(
        "spec-compat: ess/ against {short}: {verdict}"
    )));
    Ok(())
}

/// The verdict line of an `ess verify diff` that passed, or the cause it did not.
fn judge<'a>(
    output: &std::process::Output,
    acknowledged: bool,
    short: &str,
    stdout: &'a str,
    stderr: &str,
) -> Result<&'a str, String> {
    let refused = format!("refused: {ACKNOWLEDGEMENTS}");
    match output.status.code() {
        Some(0) => {}
        Some(1) if acknowledged && stderr.lines().any(|line| line.starts_with(&refused)) => {
            return Err(format!(
                "ess refused {ACKNOWLEDGEMENTS}: the line `refused:` of ess above says why"
            ));
        }
        Some(4) => {
            return Err(format!(
                "ess/ against {short}: a change is breaking or of unknown compatibility (ess verify diff exited 4)"
            ));
        }
        Some(code) => {
            return Err(format!(
                "ess verify diff exited {code}; ess/ against {short} was not compared"
            ));
        }
        None => {
            return Err(format!(
                "ess verify diff was ended by {}; ess/ against {short} was not compared",
                output.status
            ));
        }
    }
    let not_shown = |why: String| format!("{why}; ess/ against {short} is not shown compatible");
    // Each line the verdict rests on comes exactly once: one verdict (`compatibility:` or
    // `no semantic change`) and one line of the pass.
    let verdicts: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("compatibility: ") || line.starts_with(NO_CHANGE))
        .collect();
    let verdict = match verdicts[..] {
        [] => {
            return Err(not_shown(
                "ess verify diff exited 0 without a line `compatibility:` or `no semantic change`"
                    .to_string(),
            ));
        }
        [line] if line.starts_with(NO_CHANGE) => NO_CHANGE,
        [line] => line,
        _ => {
            return Err(not_shown(format!(
                "ess verify diff exited 0 with {} lines `compatibility:` or `no semantic change`, not 1",
                verdicts.len()
            )));
        }
    };
    // Exit 0 with counts: what is breaking or unknown must be acknowledged, as the pass says.
    let [breaking, unknown, _] = if verdict == NO_CHANGE {
        [0; 3]
    } else {
        counts(verdict).ok_or_else(|| {
            not_shown(format!(
                "ess verify diff exited 0 with the line {verdict:?}, which is not `compatibility: N breaking, N unknown, N compatible`"
            ))
        })?
    };
    let passed: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with(PASSED))
        .collect();
    let [line] = passed[..] else {
        return Err(not_shown(format!(
            "ess verify diff exited 0 with {} lines `{PASSED} (N acknowledged)`, not 1",
            passed.len()
        )));
    };
    let acknowledged: u64 = line
        .strip_prefix(PASSED)
        .and_then(|rest| rest.strip_prefix(" ("))
        .and_then(|rest| rest.strip_suffix(" acknowledged)"))
        .and_then(|number| number.parse().ok())
        .ok_or_else(|| {
            not_shown(format!(
                "ess verify diff exited 0 with the line {line:?}, which is not `{PASSED} (N acknowledged)`"
            ))
        })?;
    if breaking + unknown > acknowledged {
        return Err(not_shown(format!(
            "ess verify diff exited 0 with {breaking} breaking and {unknown} unknown, and {acknowledged} acknowledged"
        )));
    }
    Ok(verdict)
}

/// The line of `ess` on standard error that says the gate passed, before ` (N acknowledged)`.
const PASSED: &str = "gate --fail-on breaking-or-unknown: passed";
/// The start of the verdict line of `ess` when the 2 revisions mean the same system.
const NO_CHANGE: &str = "no semantic change";

/// The 3 numbers of `compatibility: N breaking, N unknown, N compatible`.
fn counts(verdict: &str) -> Option<[u64; 3]> {
    let rest = verdict.strip_prefix("compatibility: ")?;
    let mut numbers = [0; 3];
    let parts: Vec<&str> = rest.split(", ").collect();
    let [_, _, _] = parts[..] else {
        return None;
    };
    for ((part, label), number) in parts
        .iter()
        .zip(["breaking", "unknown", "compatible"])
        .zip(&mut numbers)
    {
        *number = part.strip_suffix(label)?.strip_suffix(' ')?.parse().ok()?;
    }
    Some(numbers)
}

/// The full id of the base commit.
fn base(root: &Path, named: Option<&str>) -> Result<String, String> {
    if let Some(named) = named.filter(|named| !named.is_empty()) {
        return commit(root, named)?
            .ok_or_else(|| format!("{BASE_VARIABLE} {named:?} names no commit"));
    }
    for (reference, name) in MAIN {
        // Only a reference git says does not exist (`show-ref --verify --quiet`: exit 1 without
        // a word) is passed over; one that exists and names no commit, or that git cannot read,
        // fails the step. `rev-parse` answers a missing object as it answers a missing
        // reference, so its silent "no commit" is asked again.
        let Some(tip) = commit(root, reference)? else {
            let exists = ["show-ref", "--verify", "--quiet", reference];
            let output = git_output(root, &exists)?;
            if output.status.code() == Some(1) && output.stderr.is_empty() {
                continue;
            }
            if !output.status.success() {
                return Err(failed(&exists, &output));
            }
            return Err(format!("{name} ({reference}) exists and names no commit"));
        };
        let args = ["merge-base", "HEAD", &tip];
        let output = git_output(root, &args)?;
        // Exit 1 without a word is git's "no merge base"; any other failure is named as git
        // wrote it.
        if output.status.code() == Some(1) && output.stderr.is_empty() {
            let shallow = git(root, &["rev-parse", "--is-shallow-repository"])?;
            let shallow = if String::from_utf8_lossy(&shallow).trim() == "true" {
                format!(
                    "; the repository is shallow, so its history may end before the fork: fetch more of it, or name the base with {BASE_VARIABLE}"
                )
            } else {
                String::new()
            };
            return Err(format!(
                "HEAD and {name} have no merge base (git merge-base exited 1){shallow}"
            ));
        }
        if !output.status.success() {
            return Err(failed(&args, &output));
        }
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    Err(format!(
        "no base to compare with: neither origin/main nor main names a commit, and {BASE_VARIABLE} is not set"
    ))
}

/// The full id of the commit `revision` names, or `None` when it names none. Only git's answer
/// "this revision does not exist" (`--verify --quiet`: exit 1 without a word) is read as none;
/// a root that is no git repository, or a git that refuses it, fails with git's first line.
fn commit(root: &Path, revision: &str) -> Result<Option<String>, String> {
    let revision = format!("{revision}^{{commit}}");
    let args = [
        "rev-parse",
        "--verify",
        "--quiet",
        "--end-of-options",
        revision.as_str(),
    ];
    let output = git_output(root, &args)?;
    if output.status.code() == Some(1) && output.stderr.is_empty() {
        return Ok(None);
    }
    if !output.status.success() {
        return Err(failed(&args, &output));
    }
    let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if id.is_empty() {
        return Err(format!("git {} exited 0 without a commit", args.join(" ")));
    }
    Ok(Some(id))
}

/// The line that names the first entry under `ess/` git is told not to look at
/// (assume-unchanged, skip-worktree), or `None` when there is none.
fn marked(root: &Path) -> Result<Option<String>, String> {
    let listing = git(root, &["ls-files", "-v", "-z", "--", SPECIFICATION])?;
    let mut found = Vec::new();
    for entry in listing.split(|byte| *byte == 0).filter(|e| !e.is_empty()) {
        let entry = String::from_utf8_lossy(entry);
        let Some((tag, path)) = entry.split_once(' ') else {
            return Err(format!(
                "git ls-files -v wrote a line of another form: {entry:?}"
            ));
        };
        // `git ls-files -v`: a lowercase tag is assume-unchanged, `S` and `s` skip-worktree.
        let assume = tag.chars().all(|c| c.is_ascii_lowercase());
        let skip = tag == "S" || tag == "s";
        let mark = match (assume, skip) {
            (true, true) => "assume-unchanged and skip-worktree",
            (true, false) => "assume-unchanged",
            (false, true) => "skip-worktree",
            (false, false) => continue,
        };
        found.push((path.to_string(), mark));
    }
    let Some((path, mark)) = found.first() else {
        return Ok(None);
    };
    let more = match found.len() - 1 {
        0 => String::new(),
        1 => " (and 1 more entry under ess/)".to_string(),
        n => format!(" (and {n} more entries under ess/)"),
    };
    Ok(Some(format!(
        "{path} is marked {mark} in the index{more}, so git does not show its edits; clear the mark (git update-index --no-assume-unchanged --no-skip-worktree -- {path}) and run the step again"
    )))
}

/// Whether the working tree's `ess/` holds the files of the base's, decided on their bytes, not
/// on git's view of the working tree (which a clean filter, the `ident` attribute, an fsmonitor
/// or a restored modification time can hide an edit from): the files under `ess/` git lists
/// (tracked, and untracked that it does not ignore) are the paths of the base's tree, each a
/// regular file whose bytes on disk, unfiltered, have the blob id of the base's entry.
fn equal(root: &Path, base: &str) -> Result<bool, String> {
    let tree = git(root, &["ls-tree", "-r", "-z", base, "--", SPECIFICATION])?;
    let mut entries = BTreeMap::new();
    for entry in tree.split(|byte| *byte == 0).filter(|e| !e.is_empty()) {
        let Ok(entry) = std::str::from_utf8(entry) else {
            return Ok(false);
        };
        let (head, path) = entry
            .split_once('\t')
            .ok_or_else(|| format!("git ls-tree wrote a line without a path: {entry:?}"))?;
        let fields: Vec<&str> = head.split(' ').collect();
        let [mode, kind, id] = fields[..] else {
            return Err(format!(
                "git ls-tree wrote a line of another form: {entry:?}"
            ));
        };
        if kind != "blob" || !(mode == "100644" || mode == "100755") {
            return Ok(false);
        }
        entries.insert(path, id);
    }
    let listed = git(
        root,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            SPECIFICATION,
        ],
    )?;
    let mut paths = BTreeSet::new();
    for path in listed.split(|byte| *byte == 0).filter(|e| !e.is_empty()) {
        let Ok(path) = std::str::from_utf8(path) else {
            return Ok(false);
        };
        paths.insert(path);
    }
    if !paths.iter().eq(entries.keys()) {
        return Ok(false);
    }
    for path in &paths {
        if !root
            .join(path)
            .symlink_metadata()
            .is_ok_and(|meta| meta.is_file())
        {
            return Ok(false);
        }
    }
    if paths.is_empty() {
        return Ok(true);
    }
    let mut args = vec!["hash-object", "--no-filters", "--"];
    args.extend(paths.iter().copied());
    let ids = git(root, &args)?;
    let ids = String::from_utf8_lossy(&ids);
    let ids: Vec<&str> = ids.lines().collect();
    if ids.len() != entries.len() {
        return Err(format!(
            "git hash-object wrote {} ids for {} files under ess/",
            ids.len(),
            entries.len()
        ));
    }
    Ok(ids.iter().copied().eq(entries.values().copied()))
}

/// Write the base's `ess/` into [`BASE_COPY`], emptied first; the path to pass as `--from`,
/// relative to the root.
fn write_base(root: &Path, base: &str, short: &str) -> Result<PathBuf, String> {
    let out = root.join(BASE_COPY);
    if out.symlink_metadata().is_ok() {
        fs::remove_dir_all(&out).map_err(|e| format!("remove {BASE_COPY}: {e}"))?;
    }
    let listing = git(root, &["ls-tree", "-r", "-z", base, "--", SPECIFICATION])?;
    let mut files = Vec::new();
    for entry in listing.split(|byte| *byte == 0).filter(|e| !e.is_empty()) {
        let entry = String::from_utf8_lossy(entry);
        let (head, path) = entry
            .split_once('\t')
            .ok_or_else(|| format!("git ls-tree wrote a line without a path: {entry:?}"))?;
        let fields: Vec<&str> = head.split(' ').collect();
        let [mode, kind, id] = fields[..] else {
            return Err(format!(
                "git ls-tree wrote a line of another form: {entry:?}"
            ));
        };
        let plain = Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)));
        if kind != "blob" || !(mode == "100644" || mode == "100755") || !plain {
            return Err(format!(
                "ess/ of {short} holds {path:?}, which is no regular file (mode {mode})"
            ));
        }
        files.push((path.to_string(), id.to_string()));
    }
    if files.is_empty() {
        return Err(format!("{short} holds no ess/"));
    }
    let ids: String = files.iter().map(|(_, id)| format!("{id}\n")).collect();
    let mut blobs = cat_file(root, &ids)?;
    for (path, id) in &files {
        let content = next_blob(&mut blobs, id)?;
        let shown = Path::new(BASE_COPY).join(path);
        let to = out.join(path);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                let parent = shown.parent().unwrap_or(&shown);
                format!("create {}: {e}", parent.display())
            })?;
        }
        fs::write(&to, content).map_err(|e| format!("write {}: {e}", shown.display()))?;
    }
    Ok(Path::new(BASE_COPY).join(SPECIFICATION))
}

/// `git cat-file --batch` on `ids`, one object id per line: the objects, each after its header.
fn cat_file(root: &Path, ids: &str) -> Result<Vec<u8>, String> {
    let mut child = Command::new("git")
        .current_dir(root)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("git cannot be run: {e}"))?;
    let mut stdin = child.stdin.take().expect("stdin is piped");
    let ids = ids.to_string();
    let writer = std::thread::spawn(move || stdin.write_all(ids.as_bytes()));
    let output = child
        .wait_with_output()
        .map_err(|e| format!("git cat-file --batch: {e}"))?;
    writer
        .join()
        .map_err(|_| "git cat-file --batch: the writer panicked".to_string())?
        .map_err(|e| format!("git cat-file --batch: {e}"))?;
    if !output.status.success() {
        return Err(failed(&["cat-file", "--batch"], &output));
    }
    Ok(output.stdout)
}

/// The next object of a `git cat-file --batch` output, which must be the blob `id`.
fn next_blob(rest: &mut Vec<u8>, id: &str) -> Result<Vec<u8>, String> {
    let wrong = || format!("git cat-file --batch did not write the blob {id}");
    let end = rest
        .iter()
        .position(|byte| *byte == b'\n')
        .ok_or_else(wrong)?;
    let header = String::from_utf8_lossy(&rest[..end]).to_string();
    let size: usize = match header.split(' ').collect::<Vec<_>>()[..] {
        [got, "blob", size] if got == id => size.parse().map_err(|_| wrong())?,
        _ => return Err(wrong()),
    };
    let start = end + 1;
    if rest.len() < start + size + 1 || rest[start + size] != b'\n' {
        return Err(wrong());
    }
    let content = rest[start..start + size].to_vec();
    rest.drain(..start + size + 1);
    Ok(content)
}

fn git_output(root: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|e| format!("git cannot be run: {e}"))
}

/// Standard output of `git <args>`, which must exit 0.
fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = git_output(root, args)?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(failed(args, &output))
    }
}

fn failed(args: &[&str], output: &std::process::Output) -> String {
    format!(
        "git {} exited {}{}",
        args.join(" "),
        output
            .status
            .code()
            .map_or_else(|| output.status.to_string(), |code| code.to_string()),
        first_line(&output.stderr)
    )
}

/// `: <the first line>` of a command's standard error, or nothing when it wrote none.
fn first_line(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .lines()
        .next()
        .map(|line| format!(": {line}"))
        .unwrap_or_default()
}
