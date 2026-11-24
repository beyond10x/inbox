//! Adversary cases of story:no-case-of-the-bounded-run-waits-on-a-real-tool (pass 1).
//!
//! `task test` makes the products of the bounded run with one `cargo nextest run -E` over the
//! setup cases, by name. cargo-nextest runs what the expression matches and fails only when it
//! matches nothing: a setup case renamed, or a name mistyped in `Taskfile.yml`, leaves the other
//! setup cases running, exit 0, and the product is built inside the bound by the first case that
//! reads it.

use std::fs;
use std::process::{Command, Stdio};

use crate::tools::repository;

/// The names of the `test(=…)` terms of the setup command of `task test`: the line of
/// `Taskfile.yml` that runs `cargo nextest run -p inbox-xtask` with `-E`.
fn setup_names() -> Vec<String> {
    let taskfile = fs::read_to_string(repository().join("Taskfile.yml")).unwrap();
    let line = taskfile
        .lines()
        .find(|line| line.contains("cargo nextest run -p inbox-xtask") && line.contains(" -E "))
        .expect("Taskfile.yml holds the setup command of task test");
    line.split("test(=")
        .skip(1)
        .map(|part| part.split(')').next().unwrap().to_owned())
        .collect()
}

/// Every setup case `task test` names is a case of this test binary, so the setup command runs
/// each of them and none of the products is left to the bounded run.
#[test]
fn every_setup_case_of_task_test_is_a_case_of_the_binary() {
    let names = setup_names();
    assert_eq!(
        names.len(),
        3,
        "the setup command of task test names {names:?}"
    );
    let listed = Command::new(std::env::current_exe().unwrap())
        .args(["--list", "--format", "terse"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(listed.status.success(), "the test binary lists its cases");
    let listed = String::from_utf8(listed.stdout).unwrap();
    let cases: Vec<&str> = listed
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .collect();
    for name in &names {
        assert!(
            cases.contains(&name.as_str()),
            "task test names the setup case {name}, which is no case of inbox-xtask: \
             cargo-nextest runs the others and exits 0"
        );
    }
}
