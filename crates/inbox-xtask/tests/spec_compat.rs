//! `inbox-xtask spec-compat`, the step of `task check` that compares `ess/` with the one of the
//! base revision (`ess verify diff --compatibility --fail-on breaking-or-unknown`). Each case makes
//! a git repository of its own from a copy of the fixture `tests/fixtures/spec-compat/ess/` (not
//! the tree's `ess/`), with a made-up identity and without the git configuration of the host.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

use crate::tools::{Run, copy_tree, on_path, replace, scratch, spec_compat_fixture};

const ACKNOWLEDGEMENTS: &str = ".config/ess-diff-acknowledgements.json";
const DOMAIN: &str = "ess/domains/box.yaml";
const OPEN_SUMMARY: &str = "summary: Open a box with a label.";
const CLOSE_OUTCOMES: &str = "    outcomes:\n      - {name: unknown-box, unknown_instance: true, error: shelf.box.UnknownBox}";
const NOT_YOURS: &str = "      - name: not-yours\n        when: label != caller.shelf\n        error: shelf.box.NotYours\n";
const PASSED: &str = "gate --fail-on breaking-or-unknown: passed (0 acknowledged)";

/// The environment of every git command of a case: no configuration of the host, a made-up
/// identity, no hook.
const GIT_ENV: [(&str, &str); 6] = [
    ("GIT_CONFIG_GLOBAL", "/dev/null"),
    ("GIT_CONFIG_NOSYSTEM", "1"),
    ("GIT_AUTHOR_NAME", "inbox-test"),
    ("GIT_AUTHOR_EMAIL", "inbox-test@example.invalid"),
    ("GIT_COMMITTER_NAME", "inbox-test"),
    ("GIT_COMMITTER_EMAIL", "inbox-test@example.invalid"),
];

/// Variables of the test's own environment that would point git elsewhere, or name a base.
const DROPPED: [&str; 6] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "CARGO_MANIFEST_DIR",
    "INBOX_SPEC_BASE",
];

struct Repo {
    root: PathBuf,
}

impl Repo {
    /// A repository root (`ess/` beside a `Cargo.toml`) with one commit on `branch`.
    fn new(name: &str, branch: &str) -> Repo {
        let root = scratch(&format!("spec-compat-{name}"));
        fs::write(root.join("Cargo.toml"), "[workspace]\n").unwrap();
        copy_tree(&spec_compat_fixture(), &root.join("ess"));
        let repo = Repo { root };
        repo.git(&["init", "-q", "-b", branch]);
        repo.commit("the specification");
        repo
    }

    fn git(&self, args: &[&str]) -> String {
        let mut command = Command::new("git");
        command
            .args(["-c", "core.hooksPath=/dev/null"])
            .args(args)
            .current_dir(&self.root);
        for name in DROPPED {
            command.env_remove(name);
        }
        let output = command.envs(GIT_ENV).output().unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }

    fn commit(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", message]);
    }

    /// The first 12 characters of the commit `revision` names: how the step names a base.
    fn short(&self, revision: &str) -> String {
        self.git(&["rev-parse", revision])[..12].to_string()
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// One summary sentence added: compatible.
    fn compatible_change(&self) {
        replace(
            &self.path(DOMAIN),
            OPEN_SUMMARY,
            "summary: Open a box with a label, or say it is open.",
        );
    }

    /// An input `CloseBox` requires: breaking for callers.
    fn breaking_change(&self) {
        replace(
            &self.path(DOMAIN),
            CLOSE_OUTCOMES,
            &format!("      - {{name: reason, type: String}}\n{CLOSE_OUTCOMES}"),
        );
    }

    /// An input `CloseBox` takes when it is given: of unknown compatibility.
    fn optional_input(&self) {
        replace(
            &self.path(DOMAIN),
            CLOSE_OUTCOMES,
            &format!("      - {{name: note, type: Optional<String>}}\n{CLOSE_OUTCOMES}"),
        );
    }

    /// The command outcome `not-yours` of `OpenBox` removed; the specification stays valid.
    /// ESS 0.98.0 rates a removed outcome unknown for callers and readers.
    fn removed_outcome(&self) {
        replace(&self.path(DOMAIN), NOT_YOURS, "");
    }

    /// `inbox-xtask spec-compat` in the root, with `env` set; `PATH` is the test's unless `env`
    /// names it.
    fn spec_compat(&self, env: &[(&str, &str)]) -> Run {
        let mut command = Command::new(env!("CARGO_BIN_EXE_inbox-xtask"));
        command.arg("spec-compat").current_dir(&self.root);
        for name in DROPPED {
            command.env_remove(name);
        }
        let output = command
            .envs(GIT_ENV)
            .envs(env.iter().copied())
            .output()
            .unwrap();
        Run {
            code: output.status.code().unwrap(),
            stdout: String::from_utf8(output.stdout).unwrap(),
            stderr: String::from_utf8(output.stderr).unwrap(),
        }
    }
}

fn shown(run: &Run) -> String {
    format!(
        "exit {}\n--- stdout\n{}--- stderr\n{}",
        run.code, run.stdout, run.stderr
    )
}

fn last(text: &str) -> &str {
    text.lines().last().unwrap_or("")
}

/// The run passed and its last line is the step's line with the counts `ess` printed.
fn passed(run: &Run, base: &str, counts: &str) {
    assert_eq!(run.code, 0, "{}", shown(run));
    assert!(
        run.stdout.lines().any(|line| line == counts),
        "{counts:?} not printed by ess:\n{}",
        shown(run)
    );
    assert_eq!(
        last(&run.stdout),
        format!("spec-compat: ess/ against {base}: {counts}"),
        "{}",
        shown(run)
    );
    assert_eq!(last(&run.stderr), PASSED, "{}", shown(run));
}

/// The run failed with `ess`'s line `fails ...: <change>` and then the step's `cause`.
fn failed_on(run: &Run, change: &str, cause: &str) {
    assert_eq!(run.code, 1, "{}", shown(run));
    let line = format!("fails --fail-on breaking-or-unknown: {change}");
    assert!(
        run.stderr.lines().any(|l| l == line),
        "{line:?} not printed:\n{}",
        shown(run)
    );
    assert_eq!(last(&run.stderr), cause, "{}", shown(run));
}

fn failed_with(run: &Run, line: &str) {
    assert_eq!(run.code, 1, "{}", shown(run));
    assert_eq!(last(&run.stderr), line, "{}", shown(run));
}

/// A directory with a stand-in `ess` that records each call in the file `called` beside it and
/// prints `lines`, and `errors` on standard error, then exits `code`; and the `PATH` that finds it
/// before any other `ess`.
fn stand_in_ess(repo: &Repo, lines: &str, errors: &str, code: i32) -> (PathBuf, String) {
    let dir = repo.root.parent().unwrap().join(format!(
        "{}-bin",
        repo.root.file_name().unwrap().to_str().unwrap()
    ));
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(&dir).unwrap();
    let script = dir.join("ess");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\necho \"$*\" >> '{}'\nprintf '%s' '{lines}'\nprintf '%s' '{errors}' >&2\nexit {code}\n",
            dir.join("called").display()
        ),
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!("{}:{}", dir.display(), std::env::var("PATH").unwrap());
    (dir.join("called"), path)
}

/// A `PATH` of one directory that holds `git` and nothing else.
fn path_without_ess(repo: &Repo) -> String {
    let dir = repo.root.parent().unwrap().join(format!(
        "{}-git-only",
        repo.root.file_name().unwrap().to_str().unwrap()
    ));
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(&dir).unwrap();
    std::os::unix::fs::symlink(on_path("git").unwrap(), dir.join("git")).unwrap();
    dir.display().to_string()
}

fn ack_refused() -> String {
    format!(
        "spec-compat: ess refused {ACKNOWLEDGEMENTS}: the line `refused:` of ess above says why"
    )
}

fn breaking_or_unknown(base: &str) -> String {
    format!(
        "spec-compat: ess/ against {base}: a change is breaking or of unknown compatibility (ess verify diff exited 4)"
    )
}

/// The digest `ess` printed after `label` (`before` or `after`) in the head of its report.
fn digest(run: &Run, label: &str) -> String {
    run.stdout
        .lines()
        .find_map(|line| line.trim().strip_prefix(label))
        .map(|rest| rest.trim().to_string())
        .unwrap_or_else(|| panic!("no {label} digest:\n{}", shown(run)))
}

#[test]
fn ess_equal_to_the_base_passes_without_a_comparison() {
    let repo = Repo::new("equal", "main");
    let (called, path) = stand_in_ess(&repo, "", "", 97);
    let run = repo.spec_compat(&[("PATH", &path)]);
    assert_eq!(run.code, 0, "{}", shown(&run));
    assert_eq!(
        run.stdout_lines(),
        [format!(
            "spec-compat: ess/ equals the one of {}; nothing compared",
            repo.short("main")
        )]
    );
    assert_eq!(run.stderr, "");
    assert!(!called.exists(), "ess was called: {}", shown(&run));
    // A file outside ess/ that differs, tracked or not, is no reason to compare.
    fs::write(repo.path("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
    fs::write(repo.path("notes.txt"), "outside\n").unwrap();
    let run = repo.spec_compat(&[("PATH", &path)]);
    assert_eq!(run.code, 0, "{}", shown(&run));
    assert!(!called.exists(), "ess was called: {}", shown(&run));
}

#[test]
fn a_compatible_change_passes_with_the_counts() {
    let repo = Repo::new("compatible", "main");
    let base = repo.short("main");
    repo.git(&["checkout", "-q", "-b", "work"]);
    repo.compatible_change();
    repo.commit("one summary sentence");
    let run = repo.spec_compat(&[]);
    passed(
        &run,
        &base,
        "compatibility: 0 breaking, 0 unknown, 1 compatible",
    );
    assert!(
        run.stdout
            .lines()
            .any(|line| line.trim() == "command/shelf.box.OpenBox/summary-changed"),
        "{}",
        shown(&run)
    );
}

#[test]
fn a_breaking_change_fails_with_the_line_of_ess() {
    let repo = Repo::new("breaking", "main");
    let base = repo.short("main");
    repo.git(&["checkout", "-q", "-b", "work"]);
    repo.breaking_change();
    repo.commit("an input CloseBox requires");
    let run = repo.spec_compat(&[]);
    failed_on(
        &run,
        "command/shelf.box.CloseBox/input-added/reason",
        &breaking_or_unknown(&base),
    );
    assert!(
        run.stdout
            .lines()
            .any(|line| line == "compatibility: 1 breaking, 0 unknown, 0 compatible"),
        "{}",
        shown(&run)
    );
}

#[test]
fn a_removed_command_outcome_fails_as_a_change_of_unknown_compatibility() {
    let repo = Repo::new("removed-outcome", "main");
    let base = repo.short("main");
    repo.git(&["checkout", "-q", "-b", "work"]);
    repo.removed_outcome();
    repo.commit("OpenBox without not-yours");
    let run = repo.spec_compat(&[]);
    failed_on(
        &run,
        "command/shelf.box.OpenBox/outcome-removed/not-yours",
        &breaking_or_unknown(&base),
    );
    assert!(
        run.stdout
            .lines()
            .any(|line| line == "compatibility: 0 breaking, 1 unknown, 0 compatible"),
        "{}",
        shown(&run)
    );
    // The same tree without the change passes.
    repo.git(&["revert", "--no-edit", "HEAD"]);
    let run = repo.spec_compat(&[]);
    assert_eq!(run.code, 0, "{}", shown(&run));
}

#[test]
fn an_acknowledged_change_passes_and_the_file_kept_for_another_change_fails() {
    let repo = Repo::new("acknowledged", "main");
    let base = repo.short("main");
    repo.git(&["checkout", "-q", "-b", "work"]);
    repo.removed_outcome();
    repo.commit("OpenBox without not-yours");
    let change = "command/shelf.box.OpenBox/outcome-removed/not-yours";
    let first = repo.spec_compat(&[]);
    failed_on(&first, change, &breaking_or_unknown(&base));
    fs::create_dir_all(repo.path(".config")).unwrap();
    fs::write(
        repo.path(ACKNOWLEDGEMENTS),
        format!(
            "{{\"format\": \"ess-diff-acknowledgements/1\", \"before\": \"{}\", \"after\": \"{}\", \"acknowledged\": [\"{change}\"]}}\n",
            digest(&first, "before"),
            digest(&first, "after")
        ),
    )
    .unwrap();
    let run = repo.spec_compat(&[]);
    assert_eq!(run.code, 0, "{}", shown(&run));
    assert!(
        run.stderr
            .lines()
            .any(|line| line == format!("acknowledged: {change}")),
        "{}",
        shown(&run)
    );
    assert_eq!(
        last(&run.stdout),
        format!(
            "spec-compat: ess/ against {base}: compatibility: 0 breaking, 1 unknown, 0 compatible"
        ),
        "{}",
        shown(&run)
    );
    assert_eq!(
        last(&run.stderr),
        "gate --fail-on breaking-or-unknown: passed (1 acknowledged)"
    );
    // The file kept for another comparison: ess refuses it, and the step points at its line.
    repo.compatible_change();
    let run = repo.spec_compat(&[]);
    failed_with(&run, &ack_refused());
    assert!(
        run.stderr
            .lines()
            .any(|line| line.starts_with(&format!("refused: {ACKNOWLEDGEMENTS}"))),
        "{}",
        shown(&run)
    );
}

#[test]
fn an_untracked_file_under_ess_is_compared() {
    let repo = Repo::new("untracked", "main");
    let base = repo.short("main");
    fs::write(repo.path("ess/notes.yaml"), "notes: not listed\n").unwrap();
    let run = repo.spec_compat(&[]);
    assert_eq!(run.code, 0, "{}", shown(&run));
    assert!(
        run.stdout
            .lines()
            .any(|line| line == "no semantic change: these two revisions mean the same system"),
        "{}",
        shown(&run)
    );
    assert_eq!(
        last(&run.stdout),
        format!("spec-compat: ess/ against {base}: no semantic change")
    );
}

#[test]
fn an_uncommitted_edit_is_compared() {
    let repo = Repo::new("uncommitted", "main");
    let base = repo.short("main");
    repo.compatible_change();
    passed(
        &repo.spec_compat(&[]),
        &base,
        "compatibility: 0 breaking, 0 unknown, 1 compatible",
    );
    // Staged and not committed: compared too.
    repo.git(&["add", "-A"]);
    passed(
        &repo.spec_compat(&[]),
        &base,
        "compatibility: 0 breaking, 0 unknown, 1 compatible",
    );
}

#[test]
fn inbox_spec_base_names_the_base() {
    let repo = Repo::new("named-base", "main");
    let first = repo.git(&["rev-parse", "HEAD"]);
    repo.compatible_change();
    repo.commit("one summary sentence, on main");
    let equal = format!(
        "spec-compat: ess/ equals the one of {}; nothing compared",
        repo.short("main")
    );
    let run = repo.spec_compat(&[]);
    assert_eq!(run.stdout_lines(), [equal.as_str()], "{}", shown(&run));
    let run = repo.spec_compat(&[("INBOX_SPEC_BASE", "")]);
    assert_eq!(run.stdout_lines(), [equal.as_str()], "{}", shown(&run));
    let counts = "compatibility: 0 breaking, 0 unknown, 1 compatible";
    for named in [first.as_str(), "HEAD^1"] {
        let run = repo.spec_compat(&[("INBOX_SPEC_BASE", named)]);
        passed(&run, &first[..12], counts);
    }
}

#[test]
fn inbox_spec_base_that_names_no_commit_fails() {
    let repo = Repo::new("named-nothing", "main");
    let (called, path) = stand_in_ess(&repo, "", "", 97);
    for named in ["no-such-revision", "HEAD:ess", "--all"] {
        let run = repo.spec_compat(&[("INBOX_SPEC_BASE", named), ("PATH", &path)]);
        failed_with(
            &run,
            &format!("spec-compat: INBOX_SPEC_BASE {named:?} names no commit"),
        );
        assert_eq!(run.stdout, "", "{}", shown(&run));
    }
    assert!(!called.exists());
}

#[test]
fn no_origin_main_and_no_main_fails() {
    let repo = Repo::new("no-main", "work");
    let run = repo.spec_compat(&[]);
    failed_with(
        &run,
        "spec-compat: no base to compare with: neither origin/main nor main names a commit, and INBOX_SPEC_BASE is not set",
    );
    // A main with no history in common with HEAD has no merge base.
    repo.git(&["checkout", "-q", "--orphan", "main"]);
    repo.commit("unrelated");
    repo.git(&["checkout", "-q", "work"]);
    let run = repo.spec_compat(&[]);
    failed_with(
        &run,
        "spec-compat: HEAD and main have no merge base (git merge-base exited 1)",
    );
}

#[test]
fn a_tree_that_merged_a_newer_main_compares_only_its_own_change() {
    let repo = Repo::new("merged-main", "main");
    repo.git(&["checkout", "-q", "-b", "work"]);
    repo.compatible_change();
    repo.commit("one summary sentence, on work");
    repo.git(&["checkout", "-q", "main"]);
    repo.optional_input();
    repo.commit("an optional input, on main");
    let tip = repo.short("main");
    repo.git(&["checkout", "-q", "work"]);
    repo.git(&["merge", "-q", "--no-edit", "main"]);
    passed(
        &repo.spec_compat(&[]),
        &tip,
        "compatibility: 0 breaking, 0 unknown, 1 compatible",
    );
}

#[test]
fn a_main_that_moved_on_after_the_fork_compares_from_the_fork() {
    let repo = Repo::new("moved-on", "main");
    let fork = repo.short("main");
    repo.git(&["checkout", "-q", "-b", "work"]);
    repo.compatible_change();
    repo.commit("one summary sentence, on work");
    repo.git(&["checkout", "-q", "main"]);
    repo.optional_input();
    repo.commit("an optional input, on main");
    let tip = repo.git(&["rev-parse", "main"]);
    repo.git(&["checkout", "-q", "work"]);
    // Only origin/main: the base is the merge base with it.
    repo.git(&["update-ref", "refs/remotes/origin/main", &tip]);
    repo.git(&["branch", "-q", "-D", "main"]);
    passed(
        &repo.spec_compat(&[]),
        &fork,
        "compatibility: 0 breaking, 0 unknown, 1 compatible",
    );
}

#[test]
fn an_ess_that_cannot_compare_fails() {
    let repo = Repo::new("no-ess", "main");
    let base = repo.short("main");
    repo.compatible_change();
    let run = repo.spec_compat(&[("PATH", &path_without_ess(&repo))]);
    assert_eq!(run.code, 1, "{}", shown(&run));
    assert!(
        last(&run.stderr).starts_with("spec-compat: ess cannot be run: "),
        "{}",
        shown(&run)
    );
    // An exit 0 without the verdict line is no pass.
    let (called, path) = stand_in_ess(&repo, "inbox v1 → v1\n", "", 0);
    let run = repo.spec_compat(&[("PATH", &path)]);
    failed_with(
        &run,
        &format!(
            "spec-compat: ess verify diff exited 0 without a line `compatibility:` or `no semantic change`; ess/ against {base} is not shown compatible"
        ),
    );
    assert!(called.exists());
    // Any other exit is no pass either.
    let (_, path) = stand_in_ess(&repo, "", "", 2);
    let run = repo.spec_compat(&[("PATH", &path)]);
    failed_with(
        &run,
        &format!("spec-compat: ess verify diff exited 2; ess/ against {base} was not compared"),
    );
}

/// Where the step puts the base's `ess/`: under `.out/`, written anew on each run.
#[test]
fn the_base_is_written_under_out_anew_on_each_run() {
    let repo = Repo::new("written", "main");
    repo.compatible_change();
    let stale = repo.path(".out/spec-compat/base/ess/stale.yaml");
    fs::create_dir_all(stale.parent().unwrap()).unwrap();
    fs::write(&stale, "stale\n").unwrap();
    let run = repo.spec_compat(&[]);
    assert_eq!(run.code, 0, "{}", shown(&run));
    assert!(!stale.exists());
    let written = repo.path(".out/spec-compat/base/ess/domains/box.yaml");
    assert_eq!(
        fs::read(&written).unwrap(),
        fs::read(spec_compat_fixture().join("domains/box.yaml")).unwrap()
    );
}

/// An entry under `ess/` git is told not to look at fails the step with one line that names it
/// and its mark, also when nothing is hidden; with the mark cleared as the line says, it passes.
#[test]
fn an_entry_git_is_told_not_to_look_at_fails_with_its_mark() {
    let repo = Repo::new("marked", "main");
    let line = |path: &str, mark: &str, more: &str| {
        format!(
            "spec-compat: {path} is marked {mark} in the index{more}, so git does not show its edits; clear the mark (git update-index --no-assume-unchanged --no-skip-worktree -- {path}) and run the step again"
        )
    };
    repo.git(&["update-index", "--skip-worktree", DOMAIN]);
    failed_with(&repo.spec_compat(&[]), &line(DOMAIN, "skip-worktree", ""));
    repo.git(&["update-index", "--assume-unchanged", DOMAIN]);
    failed_with(
        &repo.spec_compat(&[]),
        &line(DOMAIN, "assume-unchanged and skip-worktree", ""),
    );
    repo.git(&["update-index", "--no-skip-worktree", DOMAIN]);
    repo.git(&["update-index", "--assume-unchanged", "ess/system.yaml"]);
    failed_with(
        &repo.spec_compat(&[]),
        &line(DOMAIN, "assume-unchanged", " (and 1 more entry under ess/)"),
    );
    for path in [DOMAIN, "ess/system.yaml"] {
        repo.git(&[
            "update-index",
            "--no-assume-unchanged",
            "--no-skip-worktree",
            "--",
            path,
        ]);
    }
    let run = repo.spec_compat(&[]);
    assert_eq!(run.code, 0, "{}", shown(&run));
    assert!(
        last(&run.stdout).ends_with("nothing compared"),
        "{}",
        shown(&run)
    );
    // A hidden breaking edit: ess shows it, and the mark is still the cause the step names.
    repo.breaking_change();
    repo.git(&["update-index", "--skip-worktree", DOMAIN]);
    failed_on(
        &repo.spec_compat(&[]),
        "command/shelf.box.CloseBox/input-added/reason",
        &line(DOMAIN, "skip-worktree", ""),
    );
}

/// A merge base git cannot compute (the object of the fork is missing) is named by the line of
/// git, not as "no merge base".
#[test]
fn a_merge_base_git_cannot_compute_is_named_by_the_line_of_git() {
    let repo = Repo::new("broken-merge-base", "main");
    let fork = repo.git(&["rev-parse", "HEAD"]);
    repo.git(&["checkout", "-q", "-b", "work"]);
    repo.compatible_change();
    repo.commit("one summary sentence, on work");
    repo.git(&["checkout", "-q", "main"]);
    repo.optional_input();
    repo.commit("an optional input, on main");
    repo.git(&["checkout", "-q", "work"]);
    fs::remove_file(repo.path(&format!(".git/objects/{}/{}", &fork[..2], &fork[2..]))).unwrap();
    let run = repo.spec_compat(&[]);
    assert_eq!(run.code, 1, "{}", shown(&run));
    let line = last(&run.stderr);
    assert!(
        line.starts_with("spec-compat: git merge-base HEAD ")
            && !line.contains("no merge base")
            && line.contains(" exited ")
            && line.contains(&fork),
        "{}",
        shown(&run)
    );
}

/// A root git refuses (a `.git` file that points nowhere) fails with git's own line, not with
/// "names no commit", and the line names the root relative to itself, not by its path on the
/// host.
#[test]
fn a_root_git_refuses_is_named_by_the_line_of_git_without_a_path_of_the_host() {
    let repo = Repo::new("refused-root", "main");
    fs::remove_dir_all(repo.path(".git")).unwrap();
    let nowhere = repo.path("nowhere");
    fs::write(
        repo.path(".git"),
        format!("gitdir: {}\n", nowhere.display()),
    )
    .unwrap();
    let root = repo.root.display().to_string();
    for env in [vec![], vec![("INBOX_SPEC_BASE", "HEAD^1")]] {
        let run = repo.spec_compat(&env);
        assert_eq!(run.code, 1, "{}", shown(&run));
        let line = last(&run.stderr);
        assert!(
            line.starts_with("spec-compat: git rev-parse ")
                && line.contains(" exited 128: fatal: ")
                && !line.contains("names no commit")
                && !line.contains(&root),
            "{}",
            shown(&run)
        );
    }
}

/// A shallow clone whose history ends before the fork: the line says that the history may end
/// there and how to go on; with the base named, the step compares.
#[test]
fn a_shallow_history_that_ends_before_the_fork_is_named() {
    let repo = Repo::new("shallow-origin", "main");
    let fork = repo.short("main");
    repo.git(&["checkout", "-q", "-b", "work"]);
    repo.compatible_change();
    repo.commit("one summary sentence, on work");
    let clone = Repo {
        root: scratch("spec-compat-shallow-clone"),
    };
    let address = format!("file://{}", repo.root.display());
    let into = clone.root.display().to_string();
    repo.git(&[
        "clone",
        "-q",
        "--depth",
        "1",
        "--no-single-branch",
        "-b",
        "work",
        &address,
        &into,
    ]);
    assert_eq!(clone.git(&["rev-parse", "--is-shallow-repository"]), "true");
    failed_with(
        &clone.spec_compat(&[]),
        "spec-compat: HEAD and origin/main have no merge base (git merge-base exited 1); the repository is shallow, so its history may end before the fork: fetch more of it, or name the base with INBOX_SPEC_BASE",
    );
    passed(
        &clone.spec_compat(&[("INBOX_SPEC_BASE", "origin/main")]),
        &fork,
        "compatibility: 0 breaking, 0 unknown, 1 compatible",
    );
}

/// A line of git that ends on the root itself (git's refusal of a repository it takes for
/// another user's: `… in repository at '<root>'`) names the root as `.`, not by its path on the
/// host.
#[test]
fn a_line_of_git_that_ends_on_the_root_names_it_as_dot() {
    let repo = Repo::new("other-owner", "main");
    let run = repo.spec_compat(&[("GIT_TEST_ASSUME_DIFFERENT_OWNER", "1")]);
    assert_eq!(run.code, 1, "{}", shown(&run));
    let line = last(&run.stderr);
    let root = repo.root.display().to_string();
    let real = repo.root.canonicalize().unwrap().display().to_string();
    assert!(
        line.starts_with("spec-compat: git ")
            && line
                .ends_with(" exited 128: fatal: detected dubious ownership in repository at '.'")
            && !line.contains(&root)
            && !line.contains(&real),
        "{}",
        shown(&run)
    );
}

/// On exit 0 with counts, what is breaking or unknown is not more than what `ess` says it
/// acknowledged; a line of the pass the step cannot read fails it.
#[test]
fn an_exit_0_is_held_to_the_number_it_acknowledged() {
    let repo = Repo::new("held-to-acknowledged", "main");
    let base = repo.short("main");
    repo.compatible_change();
    let pass = |acknowledged: &str| {
        format!("gate --fail-on breaking-or-unknown: passed ({acknowledged} acknowledged)\n")
    };
    let not_shown = |why: &str| {
        format!(
            "spec-compat: ess verify diff exited 0 {why}; ess/ against {base} is not shown compatible"
        )
    };
    let counts = "compatibility: 1 breaking, 1 unknown, 0 compatible";
    let (_, path) = stand_in_ess(&repo, &format!("{counts}\n"), &pass("2"), 0);
    let run = repo.spec_compat(&[("PATH", &path)]);
    assert_eq!(run.code, 0, "{}", shown(&run));
    assert_eq!(
        last(&run.stdout),
        format!("spec-compat: ess/ against {base}: {counts}")
    );
    let (_, path) = stand_in_ess(&repo, &format!("{counts}\n"), &pass("1"), 0);
    failed_with(
        &repo.spec_compat(&[("PATH", &path)]),
        &not_shown("with 1 breaking and 1 unknown, and 1 acknowledged"),
    );
    let (_, path) = stand_in_ess(&repo, "compatibility: 1 breaking\n", &pass("1"), 0);
    failed_with(
        &repo.spec_compat(&[("PATH", &path)]),
        &not_shown(
            "with the line \"compatibility: 1 breaking\", which is not `compatibility: N breaking, N unknown, N compatible`",
        ),
    );
    let (_, path) = stand_in_ess(&repo, &format!("{counts}\n"), "", 0);
    failed_with(
        &repo.spec_compat(&[("PATH", &path)]),
        &not_shown(
            "with 0 lines `gate --fail-on breaking-or-unknown: passed (N acknowledged)`, not 1",
        ),
    );
    let (_, path) = stand_in_ess(&repo, &format!("{counts}\n"), &pass("two"), 0);
    failed_with(
        &repo.spec_compat(&[("PATH", &path)]),
        &not_shown(
            "with the line \"gate --fail-on breaking-or-unknown: passed (two acknowledged)\", which is not `gate --fail-on breaking-or-unknown: passed (N acknowledged)`",
        ),
    );
}
