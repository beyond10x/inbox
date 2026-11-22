//! `tools/gate-paths.sh`, the path decision of the CI job: its own 12 cases, and the answers of
//! the retired `tools/gate_paths.py` on the inputs it was run on before it was removed.

use std::fs;
use std::path::PathBuf;

use super::{replace, repository, scratch, shell};

fn script() -> PathBuf {
    repository().join("tools/gate-paths.sh")
}

#[test]
fn the_cases_of_the_script_answer_as_stated() {
    let dir = scratch("gate-paths-cases");
    let run = shell(&dir, &script(), &["--test"]);
    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        run.stdout_lines(),
        ["gate-paths: 12 case(s), each answers as stated"]
    );
    assert_eq!(run.stderr, "");
}

#[test]
fn a_broken_rule_fails_the_case_that_holds_it() {
    let dir = scratch("gate-paths-broken");
    let copy = dir.join("gate-paths.sh");
    fs::copy(script(), &copy).unwrap();
    replace(
        &copy,
        "website/* | generated/* | README.md) return 1 ;;",
        "website/* | generated/*) return 1 ;;",
    );
    let run = shell(&dir, &copy, &["--test"]);
    assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
    assert_eq!(run.stdout, "");
    assert_eq!(
        run.stderr.lines().collect::<Vec<_>>(),
        [
            "gate-paths: README.md, which an acceptance case reads: README.md answers skip, expected run"
        ]
    );
}

/// Each input with the answer `tools/gate_paths.py` printed for it (exit 0).
const ANSWERS: [(&str, &[u8], &str); 21] = [
    (
        "skip",
        b".engineering/planning/story/x.md\ndocs/plan/stage-2.md\n",
        "skip",
    ),
    ("run", b"crates/inbox-core/src/lib.rs\n", "run"),
    ("empty", b"", "run"),
    ("quoted", b"\"docs/pl\\303\\244ne.md\"\n", "run"),
    ("absolute", b"/abs/x.md\n", "run"),
    ("dotdot", b"docs/../x.md\n", "run"),
    ("blank-lines", b"docs/a.md\n\n\ndocs/b.md\n", "skip"),
    ("no-newline", b"docs/a.md", "skip"),
    ("not-utf8", b"docs/\xff.md\n", "run"),
    ("readme", b"README.md\n", "run"),
    ("nested-readme", b"docs/README.md\n", "skip"),
    ("upper", b"docs/x.MD\n", "run"),
    ("crlf", b"docs/a.md\r\n", "skip"),
    ("lone-cr", b"docs/a.md\rcrates/x.rs\n", "run"),
    ("engineering-bare", b".engineering\n", "run"),
    ("website", b"website/x.md\n", "run"),
    ("dotmd", b".md\n", "skip"),
    ("space-line", b"docs/a.md\n \n", "run"),
    ("dotdot-space", b"docs/.. /a.md\n", "skip"),
    ("dotdot-in-name", b"docs/x..md\n", "skip"),
    ("utf8-unquoted", "docs/pläne.md\n".as_bytes(), "skip"),
];

#[test]
fn each_input_answers_as_the_retired_script_did() {
    let dir = scratch("gate-paths-answers");
    for (name, content, expected) in ANSWERS {
        let file = dir.join(name);
        fs::write(&file, content).unwrap();
        let run = shell(&dir, &script(), &[file.to_str().unwrap()]);
        assert_eq!(run.code, 0, "{name}: {}{}", run.stdout, run.stderr);
        assert_eq!(run.stdout_lines(), [expected], "{name}");
    }
}

#[test]
fn an_unreadable_list_answers_run() {
    let dir = scratch("gate-paths-unreadable");
    fs::create_dir_all(dir.join("a-directory")).unwrap();
    for argument in ["does-not-exist", "a-directory"] {
        let run = shell(&dir, &script(), &[argument]);
        assert_eq!(run.code, 0, "{argument}: {}{}", run.stdout, run.stderr);
        assert_eq!(run.stdout_lines(), ["run"], "{argument}");
    }
}

#[test]
fn a_wrong_number_of_arguments_is_a_usage_error() {
    let dir = scratch("gate-paths-usage");
    for arguments in [&[][..], &["a", "b"][..]] {
        let run = shell(&dir, &script(), arguments);
        assert_eq!(run.code, 2, "{arguments:?}: {}{}", run.stdout, run.stderr);
        assert_eq!(run.stdout, "");
        assert!(
            run.stderr.starts_with("usage: gate-paths.sh"),
            "{}",
            run.stderr
        );
    }
}
