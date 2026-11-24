//! Adversary cases of `inbox-xtask spec-compat`. Each case makes a git repository of its own in a
//! temporary directory from a copy of `tests/fixtures/spec-compat/ess/`, with a made-up identity,
//! without the git configuration and the hooks of the host, and without `INBOX_SPEC_BASE` unless it
//! sets it.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

use crate::tools::{Run, copy_tree, replace, scratch, spec_compat_fixture};

const ACKNOWLEDGEMENTS: &str = ".config/ess-diff-acknowledgements.json";
const DOMAIN: &str = "ess/domains/box.yaml";
const OPEN_SUMMARY: &str = "summary: Open a box with a label.";
const CLOSE_OUTCOMES: &str = "    outcomes:\n      - {name: unknown-box, unknown_instance: true, error: shelf.box.UnknownBox}";
const BREAKING: &str = "command/shelf.box.CloseBox/input-added/reason";

const GIT_ENV: [(&str, &str); 6] = [
    ("GIT_CONFIG_GLOBAL", "/dev/null"),
    ("GIT_CONFIG_NOSYSTEM", "1"),
    ("GIT_AUTHOR_NAME", "inbox-test"),
    ("GIT_AUTHOR_EMAIL", "inbox-test@example.invalid"),
    ("GIT_COMMITTER_NAME", "inbox-test"),
    ("GIT_COMMITTER_EMAIL", "inbox-test@example.invalid"),
];

const DROPPED: [&str; 7] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_CEILING_DIRECTORIES",
    "CARGO_MANIFEST_DIR",
    "INBOX_SPEC_BASE",
];

struct Repo {
    root: PathBuf,
}

impl Repo {
    /// A root (`ess/` beside a `Cargo.toml`); a git repository with one commit on `main` when
    /// `git` is true.
    fn new(name: &str, git: bool) -> Repo {
        let root = scratch(&format!("adv-spec-compat-{name}")).join("root");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("Cargo.toml"), "[workspace]\n").unwrap();
        copy_tree(&spec_compat_fixture(), &root.join("ess"));
        let repo = Repo { root };
        if git {
            repo.git(&["init", "-q", "-b", "main"]);
            repo.commit("the specification");
        }
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

    fn short(&self, revision: &str) -> String {
        self.git(&["rev-parse", revision])[..12].to_string()
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    fn breaking_change(&self) {
        replace(
            &self.path(DOMAIN),
            CLOSE_OUTCOMES,
            &format!("      - {{name: reason, type: String}}\n{CLOSE_OUTCOMES}"),
        );
    }

    fn compatible_change(&self) {
        replace(
            &self.path(DOMAIN),
            OPEN_SUMMARY,
            "summary: Open the inbox at an address, or open it again; its adapter is the one given last.",
        );
    }

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

    /// A stand-in `ess` that prints `stdout` and `stderr` and exits `code`; the `PATH` to find it.
    fn stand_in_ess(&self, stdout: &str, stderr: &str, code: i32) -> String {
        let dir = self.root.parent().unwrap().join("bin");
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        fs::create_dir_all(&dir).unwrap();
        let script = dir.join("ess");
        fs::write(
            &script,
            format!("#!/bin/sh\nprintf '%s' '{stdout}'\nprintf '%s' '{stderr}' >&2\nexit {code}\n"),
        )
        .unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        format!("{}:{}", dir.display(), std::env::var("PATH").unwrap())
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

fn digest(run: &Run, label: &str) -> String {
    run.stdout
        .lines()
        .find_map(|line| line.trim().strip_prefix(label))
        .map(|rest| rest.trim().to_string())
        .unwrap_or_else(|| panic!("no {label} digest:\n{}", shown(run)))
}

/// A breaking edit of `ess/` that git is told not to look at (assume-unchanged, skip-worktree)
/// is still the `ess/` every other step of the gate reads: it is compared and fails.
#[test]
fn a_breaking_edit_git_is_told_not_to_see_fails() {
    let mut runs = Vec::new();
    for flag in ["--assume-unchanged", "--skip-worktree"] {
        let repo = Repo::new(&format!("hidden{flag}"), true);
        repo.git(&["checkout", "-q", "-b", "work"]);
        repo.breaking_change();
        repo.git(&["update-index", flag, DOMAIN]);
        runs.push((flag, repo.spec_compat(&[])));
    }
    let line = format!("fails --fail-on breaking-or-unknown: {BREAKING}");
    for (flag, run) in &runs {
        assert!(
            run.code == 1 && run.stderr.lines().any(|l| l == line),
            "{flag}: a breaking edit of ess/ passed:\n{}",
            shown(run)
        );
    }
}

/// With both `origin/main` and `main`, the merge base with `origin/main` is the base, also when
/// the local `main` is behind it.
#[test]
fn origin_main_wins_over_a_stale_main() {
    let repo = Repo::new("origin-wins", true);
    let stale = repo.git(&["rev-parse", "main"]);
    repo.compatible_change();
    repo.commit("one summary sentence, on origin/main");
    let tip = repo.git(&["rev-parse", "HEAD"]);
    repo.git(&["update-ref", "refs/remotes/origin/main", &tip]);
    repo.git(&["checkout", "-q", "-b", "work"]);
    repo.git(&["branch", "-q", "-f", "main", &stale]);
    let run = repo.spec_compat(&[]);
    assert_eq!(run.code, 0, "{}", shown(&run));
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        [format!(
            "spec-compat: ess/ equals the one of {}; nothing compared",
            &tip[..12]
        )],
        "{}",
        shown(&run)
    );
}

/// An acknowledgements file written for this comparison (its 2 digests) that names a change id
/// this comparison does not have fails; the step's line does not say the file is for another
/// comparison.
#[test]
fn an_acknowledgement_of_an_absent_change_is_not_called_another_comparison() {
    let repo = Repo::new("ack-absent-id", true);
    repo.git(&["checkout", "-q", "-b", "work"]);
    repo.breaking_change();
    repo.commit("an input CloseInbox requires");
    let first = repo.spec_compat(&[]);
    assert_eq!(first.code, 1, "{}", shown(&first));
    fs::create_dir_all(repo.path(".config")).unwrap();
    fs::write(
        repo.path(ACKNOWLEDGEMENTS),
        format!(
            "{{\"format\": \"ess-diff-acknowledgements/1\", \"before\": \"{}\", \"after\": \"{}\", \"acknowledged\": [\"{BREAKING}-typo\"]}}\n",
            digest(&first, "before"),
            digest(&first, "after")
        ),
    )
    .unwrap();
    let run = repo.spec_compat(&[]);
    assert_eq!(run.code, 1, "{}", shown(&run));
    assert!(
        run.stderr.lines().any(|line| line.starts_with(&format!(
            "refused: {ACKNOWLEDGEMENTS}: [undeclared_reference]"
        ))),
        "{}",
        shown(&run)
    );
    assert!(
        !last(&run.stderr).contains("is not for this comparison"),
        "the file is for this comparison; the step says it is not:\n{}",
        shown(&run)
    );
}

/// The line "… is not for this comparison …" comes only on an exit 1 of `ess` that refused the
/// acknowledgements file the step passed; any other exit is named as it is.
#[test]
fn the_refusal_line_is_added_only_on_a_refusal() {
    let repo = Repo::new("refusal-only", true);
    let base = repo.short("main");
    repo.compatible_change();
    let refused = format!("refused: {ACKNOWLEDGEMENTS}: [conflicting_declaration] x\n");
    let not_compared = |code: i32| {
        format!("spec-compat: ess verify diff exited {code}; ess/ against {base} was not compared")
    };
    // No acknowledgements file: a refused line of ess is no refusal of a file the step passed.
    let path = repo.stand_in_ess("", &refused, 1);
    let run = repo.spec_compat(&[("PATH", &path)]);
    assert_eq!(last(&run.stderr), not_compared(1), "{}", shown(&run));
    fs::create_dir_all(repo.path(".config")).unwrap();
    fs::write(repo.path(ACKNOWLEDGEMENTS), "{}\n").unwrap();
    // The file passed and ess exits 1 for another cause.
    let path = repo.stand_in_ess("", "error: reading ess/system.yaml: denied\n", 1);
    let run = repo.spec_compat(&[("PATH", &path)]);
    assert_eq!(last(&run.stderr), not_compared(1), "{}", shown(&run));
    // The file passed, a refused line, and another exit.
    let path = repo.stand_in_ess("", &refused, 3);
    let run = repo.spec_compat(&[("PATH", &path)]);
    assert_eq!(last(&run.stderr), not_compared(3), "{}", shown(&run));
}

/// An `ess` that exits 0 and prints a verdict with a breaking change, while no change is
/// acknowledged, is no pass.
#[test]
fn an_exit_0_with_a_breaking_count_is_no_pass() {
    let repo = Repo::new("exit-0-breaking", true);
    repo.compatible_change();
    let path = repo.stand_in_ess(
        "compatibility: 1 breaking, 0 unknown, 0 compatible\n",
        "gate --fail-on breaking-or-unknown: passed (0 acknowledged)\n",
        0,
    );
    let run = repo.spec_compat(&[("PATH", &path)]);
    assert_eq!(
        run.code,
        1,
        "1 breaking, 0 acknowledged passed:\n{}",
        shown(&run)
    );
}

/// A root that is no git repository fails with a line that names that cause, not a missing
/// `origin/main` or a revision that names no commit.
#[test]
fn a_root_that_is_no_git_repository_is_named() {
    let repo = Repo::new("no-repository", false);
    let ceiling = repo.root.parent().unwrap().display().to_string();
    for env in [
        vec![("GIT_CEILING_DIRECTORIES", ceiling.as_str())],
        vec![
            ("GIT_CEILING_DIRECTORIES", ceiling.as_str()),
            ("INBOX_SPEC_BASE", "HEAD^1"),
        ],
    ] {
        let run = repo.spec_compat(&env);
        assert_eq!(run.code, 1, "{}", shown(&run));
        let line = last(&run.stderr);
        assert!(
            !line.contains("neither origin/main nor main names a commit")
                && !line.contains("names no commit"),
            "the cause is no git repository; the step names another:\n{}",
            shown(&run)
        );
    }
}

/// A cause line of the step holds no path of the host.
#[test]
fn a_cause_line_holds_no_path_of_the_host() {
    let repo = Repo::new("host-path", true);
    repo.compatible_change();
    fs::create_dir_all(repo.path(".out/spec-compat")).unwrap();
    fs::write(
        repo.path(".out/spec-compat/base"),
        "a file, not a directory\n",
    )
    .unwrap();
    let run = repo.spec_compat(&[]);
    assert_eq!(run.code, 1, "{}", shown(&run));
    let root = repo.root.display().to_string();
    assert!(
        !last(&run.stderr).contains(&root),
        "a path of the host:\n{}",
        shown(&run)
    );
}

// Pass 2.

const NOT_YOURS: &str = "      - name: not-yours\n        when: label != caller.shelf\n        error: shelf.box.NotYours\n";

/// A breaking edit that a clean filter of the repository's own configuration takes out of git's
/// view is still the `ess/` every other step of the gate reads: the step does not call it equal.
#[test]
fn a_breaking_edit_a_clean_filter_hides_fails() {
    let repo = Repo::new("clean-filter", true);
    repo.git(&["checkout", "-q", "-b", "work"]);
    repo.git(&[
        "config",
        "filter.hide.clean",
        "grep -v 'name: reason, type: String'",
    ]);
    fs::create_dir_all(repo.path(".git/info")).unwrap();
    fs::write(repo.path(".git/info/attributes"), "ess/** filter=hide\n").unwrap();
    repo.breaking_change();
    let run = repo.spec_compat(&[]);
    assert!(
        run.code == 1 && !run.stdout.contains("nothing compared"),
        "a breaking edit of ess/ that a clean filter hides passed:\n{}",
        shown(&run)
    );
}

/// An `origin/main` whose object is missing is no `origin/main` that is absent: the step does not
/// fall back to a stale `main` and pass a change that is breaking against `origin/main`.
#[test]
fn an_origin_main_whose_object_is_missing_is_no_fallback_to_main() {
    let repo = Repo::new("origin-missing", true);
    replace(&repo.path(DOMAIN), NOT_YOURS, "");
    repo.commit("a stale main, without the outcome not-yours");
    let stale = repo.git(&["rev-parse", "HEAD"]);
    fs::copy(
        spec_compat_fixture().join("domains/box.yaml"),
        repo.path(DOMAIN),
    )
    .unwrap();
    repo.commit("origin/main, with the outcome not-yours");
    let tip = repo.git(&["rev-parse", "HEAD"]);
    repo.git(&["update-ref", "refs/remotes/origin/main", &tip]);
    let reference = repo.path(".git/refs/remotes/origin/main");
    assert!(reference.is_file(), "the case needs loose refs");
    repo.git(&["checkout", "-q", "-b", "work"]);
    repo.git(&["branch", "-q", "-f", "main", &stale]);
    replace(&repo.path(DOMAIN), NOT_YOURS, "");
    let control = repo.spec_compat(&[]);
    assert_eq!(control.code, 1, "control:\n{}", shown(&control));
    fs::write(&reference, "0123456789abcdef0123456789abcdef01234567\n").unwrap();
    let run = repo.spec_compat(&[]);
    assert_eq!(
        run.code,
        1,
        "origin/main names a missing object; the step compared with the stale main:\n{}",
        shown(&run)
    );
}

/// An `ess` that exits 0 with a second `compatibility:` line that counts a breaking change, while
/// nothing is acknowledged, is no pass.
#[test]
fn a_second_compatibility_line_is_no_pass() {
    let repo = Repo::new("second-verdict", true);
    repo.compatible_change();
    let path = repo.stand_in_ess(
        "compatibility: 0 breaking, 0 unknown, 1 compatible\ncompatibility: 1 breaking, 0 unknown, 0 compatible\n",
        "gate --fail-on breaking-or-unknown: passed (0 acknowledged)\n",
        0,
    );
    let run = repo.spec_compat(&[("PATH", &path)]);
    assert_eq!(
        run.code,
        1,
        "a second verdict with 1 breaking passed:\n{}",
        shown(&run)
    );
}

/// Two lines of the pass are no pass, also when the first acknowledges enough.
#[test]
fn two_lines_of_the_pass_are_no_pass() {
    let repo = Repo::new("two-passes", true);
    repo.compatible_change();
    let path = repo.stand_in_ess(
        "compatibility: 1 breaking, 0 unknown, 0 compatible\n",
        "gate --fail-on breaking-or-unknown: passed (2 acknowledged)\ngate --fail-on breaking-or-unknown: passed (0 acknowledged)\n",
        0,
    );
    let run = repo.spec_compat(&[("PATH", &path)]);
    assert_eq!(run.code, 1, "{}", shown(&run));
    assert!(
        last(&run.stderr).contains("with 2 lines `gate --fail-on breaking-or-unknown: passed"),
        "{}",
        shown(&run)
    );
}

/// A root reached through a symbolic link (`CARGO_MANIFEST_DIR`, as `cargo run` sets it): a line
/// of git that names the real path of the root (here its `.git`) names it relative to the root.
#[test]
fn a_root_through_a_symbolic_link_is_named_relative() {
    let repo = Repo::new("linked-root", true);
    fs::remove_dir_all(repo.path(".git")).unwrap();
    fs::write(
        repo.path(".git"),
        format!("gitdir: {}\n", repo.path("nowhere").display()),
    )
    .unwrap();
    let link = repo.root.parent().unwrap().join("link");
    std::os::unix::fs::symlink(&repo.root, &link).unwrap();
    let manifest = link.join("crates/inbox-xtask").display().to_string();
    let run = repo.spec_compat(&[("CARGO_MANIFEST_DIR", manifest.as_str())]);
    assert_eq!(run.code, 1, "{}", shown(&run));
    let line = last(&run.stderr);
    let real = repo.root.display().to_string();
    let linked = link.display().to_string();
    assert!(
        line.starts_with("spec-compat: git rev-parse ")
            && line.contains(" exited 128: fatal: ")
            && !line.contains(&real)
            && !line.contains(&linked),
        "{}",
        shown(&run)
    );
}
