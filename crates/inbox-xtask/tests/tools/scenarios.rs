//! `inbox-xtask scenarios`: the checks of the authored scenario files and the manifest that lists
//! them. Each planted defect is one the retired generator refused or reported.

use std::fs;
use std::path::{Path, PathBuf};

use super::{Run, replace, repository, tree, xtask};

const SUMMARY: &str = "42 scenarios, 0 path(s) differ from ess/ess-inputs.yaml";

fn ess_tree(name: &str) -> PathBuf {
    tree(name, &["ess"])
}

fn scenario(root: &Path, name: &str) -> PathBuf {
    root.join("ess/scenarios").join(format!("{name}.yaml"))
}

fn manifest(root: &Path) -> PathBuf {
    root.join("ess/ess-inputs.yaml")
}

fn check(root: &Path) -> Run {
    xtask(root, &["scenarios", "--check"])
}

/// The run failed its checks: exit 1, `line` on standard error, nothing on standard output.
fn refused(run: &Run, line: &str) {
    assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
    assert!(
        run.stderr.lines().any(|l| l == line),
        "{line:?} not in:\n{}",
        run.stderr
    );
    assert!(
        run.stderr
            .lines()
            .last()
            .unwrap()
            .ends_with("check(s) failed; nothing written"),
        "{}",
        run.stderr
    );
    assert_eq!(run.stdout, "");
}

/// Insert `line` as the first entry of the `covers:` list of a scenario file.
fn claim(path: &Path, line: &str) {
    replace(path, "covers:\n", &format!("covers:\n- {line}\n"));
}

#[test]
fn the_committed_scenarios_pass_and_no_path_differs() {
    let root = ess_tree("scenarios-clean");
    let run = check(&root);
    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(run.stdout_lines(), [SUMMARY]);
}

#[test]
fn a_covered_id_with_two_claimants_is_refused() {
    let root = ess_tree("scenarios-two-claimants");
    claim(
        &scenario(&root, "repeated-dedup-key"),
        "inbox.messaging.PostMessage/outcome/duplicate",
    );
    refused(
        &check(&root),
        "scenarios: an id is claimed twice: inbox.messaging.PostMessage/outcome/duplicate (repeated-dedup-key, repost-after-restart-arrives-once)",
    );
}

#[test]
fn a_scenario_file_the_manifest_does_not_list_is_named() {
    let root = ess_tree("scenarios-unlisted");
    replace(&manifest(&root), "- scenarios/note-expires.yaml\n", "");
    fs::write(root.join("ess/scenarios/notes.txt"), "a stray file\n").unwrap();
    let run = check(&root);
    assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        run.stdout_lines(),
        [
            "not in the manifest: ess/scenarios/note-expires.yaml",
            "not in the manifest: ess/scenarios/notes.txt",
            "42 scenarios, 2 path(s) differ from ess/ess-inputs.yaml",
        ]
    );
}

#[test]
fn a_manifest_line_without_its_file_is_missing() {
    let root = ess_tree("scenarios-missing");
    fs::remove_file(scenario(&root, "note-expires")).unwrap();
    let run = check(&root);
    assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        run.stdout_lines(),
        [
            "missing: ess/scenarios/note-expires.yaml",
            "41 scenarios, 1 path(s) differ from ess/ess-inputs.yaml",
        ]
    );
}

#[test]
fn a_manifest_out_of_its_form_is_edited() {
    for (name, from, to) in [
        (
            "scenarios-edited-requires",
            "requires: ess 0.97.3\n",
            "requires: ess 0.97.1\n",
        ),
        (
            "scenarios-edited-twice",
            "- scenarios/note-expires.yaml\n",
            "- scenarios/note-expires.yaml\n- scenarios/note-expires.yaml\n",
        ),
        (
            "scenarios-edited-order",
            "- scenarios/note-expires.yaml\n- scenarios/note-read-is-gone.yaml\n",
            "- scenarios/note-read-is-gone.yaml\n- scenarios/note-expires.yaml\n",
        ),
        (
            "scenarios-edited-not-a-scenario",
            "- scenarios/note-expires.yaml\n",
            "- scenarios/note-expires.yaml\n- domains/messaging.yaml\n",
        ),
    ] {
        let root = ess_tree(name);
        replace(&manifest(&root), from, to);
        let run = check(&root);
        assert_eq!(run.code, 1, "{name}: {}{}", run.stdout, run.stderr);
        assert_eq!(
            run.stdout_lines(),
            [
                "edited: ess/ess-inputs.yaml",
                "42 scenarios, 1 path(s) differ from ess/ess-inputs.yaml",
            ],
            "{name}"
        );
    }
}

#[test]
fn an_absent_manifest_is_missing() {
    let root = ess_tree("scenarios-no-manifest");
    fs::remove_file(manifest(&root)).unwrap();
    let run = check(&root);
    assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        run.stdout_lines(),
        [
            "missing: ess/ess-inputs.yaml",
            "42 scenarios, 1 path(s) differ from ess/ess-inputs.yaml",
        ]
    );
}

#[test]
fn scenarios_rewrites_the_manifest_from_the_files() {
    let root = ess_tree("scenarios-write");
    replace(&manifest(&root), "- scenarios/note-expires.yaml\n", "");
    let run = xtask(&root, &["scenarios"]);
    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(run.stdout_lines(), ["42 scenarios"]);
    assert_eq!(
        fs::read(manifest(&root)).unwrap(),
        fs::read(repository().join("ess/ess-inputs.yaml")).unwrap()
    );
    assert_eq!(check(&root).stdout_lines(), [SUMMARY]);
}

#[test]
fn a_failed_check_writes_no_manifest() {
    let root = ess_tree("scenarios-write-refused");
    replace(&manifest(&root), "- scenarios/note-expires.yaml\n", "");
    let before = fs::read(manifest(&root)).unwrap();
    claim(
        &scenario(&root, "repeated-dedup-key"),
        "inbox.messaging.PostMessage/outcome/duplicate",
    );
    let run = xtask(&root, &["scenarios"]);
    refused(
        &run,
        "scenarios: an id is claimed twice: inbox.messaging.PostMessage/outcome/duplicate (repeated-dedup-key, repost-after-restart-arrives-once)",
    );
    assert_eq!(fs::read(manifest(&root)).unwrap(), before);
}

#[test]
fn a_file_that_is_no_scenario_file_writes_no_manifest() {
    let root = ess_tree("scenarios-write-stray");
    fs::write(root.join("ess/scenarios/notes.txt"), "a stray file\n").unwrap();
    replace(&manifest(&root), "- scenarios/note-expires.yaml\n", "");
    let before = fs::read(manifest(&root)).unwrap();
    let run = xtask(&root, &["scenarios"]);
    assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        run.stderr.lines().collect::<Vec<_>>(),
        [
            "scenarios: ess/scenarios/notes.txt: no scenario file (a name of a-z, 0-9 and -, ending in .yaml); nothing written"
        ]
    );
    assert_eq!(fs::read(manifest(&root)).unwrap(), before);
}

#[test]
fn a_claim_on_a_view_field_no_row_holds_is_refused() {
    let anchor = "    body: Ship release 4.2 today?\n    decision: *id001\n";
    for (name, to) in [
        ("scenarios-row-absent", "    decision: *id001\n"),
        (
            "scenarios-row-null",
            "    body: null\n    decision: *id001\n",
        ),
    ] {
        let root = ess_tree(name);
        replace(
            &scenario(&root, "read-decision-waits-for-its-answer"),
            anchor,
            to,
        );
        refused(
            &check(&root),
            "scenarios: read-decision-waits-for-its-answer claims inbox.messaging.Body/invariant/at/inbox.messaging.Unanswered/body and reads no value at `body` in a row of Unanswered",
        );
    }
}

#[test]
fn a_claim_through_a_list_needs_a_value_in_every_element() {
    let root = ess_tree("scenarios-row-list");
    let file = scenario(&root, "read-decision-waits-for-its-answer");
    replace(
        &file,
        "      - letter: B\n        text: Hold",
        "      - text: Hold",
    );
    refused(
        &check(&root),
        "scenarios: read-decision-waits-for-its-answer claims inbox.messaging.OptionLetter/invariant-wrapped/at/inbox.messaging.Unanswered/decision/5fc40af03c03b2190846fd86e4dc7b3694f364228b490834f133211ec5ee2e25 and reads no value at `decision.options.letter` in a row of Unanswered",
    );
}

#[test]
fn an_invariant_id_the_check_cannot_read_is_refused() {
    let zeros = "0".repeat(64);
    for (name, id, verdict) in [
        (
            "scenarios-invariant-shape",
            "inbox.messaging.Body/invariant/at/inbox.messaging.Unanswered".to_string(),
            "an invariant id of a shape this check does not read",
        ),
        (
            "scenarios-invariant-wrapped",
            format!(
                "inbox.messaging.Body/invariant-wrapped/at/inbox.messaging.Unanswered/body/{zeros}"
            ),
            "a wrapped path this check does not know",
        ),
        (
            "scenarios-invariant-digest",
            format!("inbox.messaging.Body/invariant/at/inbox.messaging.Unanswered/body/{zeros}"),
            "an invariant id that is not wrapped carries a digest",
        ),
    ] {
        let root = ess_tree(name);
        claim(&scenario(&root, "read-decision-waits-for-its-answer"), &id);
        refused(
            &check(&root),
            &format!("scenarios: read-decision-waits-for-its-answer: {id}: {verdict}"),
        );
    }
}

#[test]
fn a_summary_over_200_characters_is_refused() {
    let root = ess_tree("scenarios-summary");
    let file = scenario(&root, "note-takes-no-answer");
    let text = fs::read_to_string(&file).unwrap();
    let start = text.find("\nsummary: ").unwrap() + 1;
    let end = start + text[start..].find("\ntimeline:").unwrap() + 1;
    let summary = "ä".repeat(201);
    fs::write(
        &file,
        format!("{}summary: {summary}\n{}", &text[..start], &text[end..]),
    )
    .unwrap();
    refused(
        &check(&root),
        "scenarios: ess/scenarios/note-takes-no-answer.yaml: the summary has 201 characters, more than 200",
    );
}

#[test]
fn a_message_id_that_is_no_ulid_is_refused() {
    for (name, id) in [
        ("scenarios-id-short", "01JGT0AN00000000000000001"),
        ("scenarios-id-letter", "01JGT0AN00000000000000000I"),
    ] {
        let root = ess_tree(name);
        replace(
            &scenario(&root, "note-takes-no-answer"),
            "message_id: 01JGT0AN000000000000000001\n",
            &format!("message_id: {id}\n"),
        );
        refused(
            &check(&root),
            &format!(
                "scenarios: ess/scenarios/note-takes-no-answer.yaml: `message_id` is `{id}`, not a message id (26 characters of 0-9 and A-Z without I, L, O, U)"
            ),
        );
    }
}

#[test]
fn a_scenario_named_unlike_its_file_is_refused() {
    let root = ess_tree("scenarios-name");
    replace(
        &scenario(&root, "note-expires"),
        "scenario: note-expires\n",
        "scenario: note-expired\n",
    );
    refused(
        &check(&root),
        "scenarios: ess/scenarios/note-expires.yaml: `scenario:` is `note-expired`, not the file's name `note-expires`",
    );
}
