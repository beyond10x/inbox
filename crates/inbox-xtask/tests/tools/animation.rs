//! `inbox-xtask animation`: the data of the landing page's figure, written from the compiled `ess/`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{Run, repository, tree, xtask};

const FILE: &str = "website/src/generated/animation.json";

fn animation_tree(name: &str) -> PathBuf {
    tree(name, &["ess", FILE])
}

/// The IR `ess specify compile` makes of the tree's `ess/`, as text.
fn compiled(root: &Path) -> String {
    let output = Command::new("ess")
        .args(["specify", "compile", "--path", "ess", "--format", "json"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// Writing from `ir` fails with a line that starts with `starts`, and the file stays as it was.
fn refused(root: &Path, ir: &str, starts: &str) -> Run {
    let path = root.join("ir.json");
    fs::write(&path, ir).unwrap();
    let before = fs::read(root.join(FILE)).unwrap();
    let run = xtask(root, &["animation", "--ir", "ir.json"]);
    assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
    assert!(run.stderr.starts_with(starts), "{}", run.stderr);
    assert_eq!(fs::read(root.join(FILE)).unwrap(), before);
    run
}

#[test]
fn the_writer_reproduces_the_committed_file_from_the_compiled_ess() {
    let root = animation_tree("animation-clean");
    let run = xtask(&root, &["animation", "--check"]);
    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(run.stdout, "");
    fs::remove_file(root.join(FILE)).unwrap();
    let run = xtask(&root, &["animation"]);
    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        fs::read(root.join(FILE)).unwrap(),
        fs::read(repository().join(FILE)).unwrap()
    );
}

#[test]
fn an_edited_file_is_reported() {
    let root = animation_tree("animation-edited");
    let path = root.join(FILE);
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        text.replacen("\"frame_ms\": 2600", "\"frame_ms\": 2601", 1),
    )
    .unwrap();
    let run = xtask(&root, &["animation", "--check"]);
    assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
    assert_eq!(run.stdout_lines(), [format!("edited: {FILE}")]);
}

#[test]
fn an_absent_file_is_missing() {
    let root = animation_tree("animation-missing");
    fs::remove_file(root.join(FILE)).unwrap();
    let run = xtask(&root, &["animation", "--check"]);
    assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
    assert_eq!(run.stdout_lines(), [format!("missing: {FILE}")]);
    assert!(!root.join(FILE).exists());
}

#[test]
fn a_view_with_no_place_in_the_story_writes_nothing() {
    let root = animation_tree("animation-view");
    let mut ir: serde_json::Value = serde_json::from_str(&compiled(&root)).unwrap();
    let views = ir["views"].as_object_mut().unwrap();
    let mut nowhere = views.values().next().unwrap().clone();
    nowhere["source"] = serde_json::Value::from("inbox.messaging.Nothing");
    views.insert("inbox.messaging.Nowhere".to_string(), nowhere);
    let run = refused(
        &root,
        &ir.to_string(),
        "animation: views with no place in the story: Nowhere",
    );
    assert_eq!(run.stdout, "");
}

#[test]
fn a_frame_over_the_word_limit_writes_nothing() {
    let root = animation_tree("animation-words");
    let ir = compiled(&root);
    assert!(ir.contains("\"inbox.messaging.NotDue\""));
    let ir = ir.replace(
        "\"inbox.messaging.NotDue\"",
        "\"inbox.messaging.Not due at all yet, nor soon\"",
    );
    let run = refused(&root, &ir, "animation: frame ");
    assert!(
        run.stderr.contains("words, more than 12: ["),
        "{}",
        run.stderr
    );
}
