use std::fs;
use std::path::{Path, PathBuf};

use inbox_xtask::{Difference, GENERATED, differences, drift, generate};

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn copy_tree(from: &Path, to: &Path) {
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

fn pair(name: &str) -> (PathBuf, PathBuf) {
    let root = scratch(name);
    let fresh = root.join("fresh");
    let held = root.join("held");
    for dir in [&fresh, &held] {
        write(&dir.join("Cargo.toml"), "[package]\n");
        write(&dir.join("src/lib.rs"), "pub mod a;\n");
        write(&dir.join("src/a.rs"), "pub struct A;\n");
    }
    (fresh, held)
}

#[test]
fn identical_trees_have_no_difference() {
    let (fresh, held) = pair("identical");
    assert_eq!(
        differences(&fresh, &held).unwrap(),
        Vec::<Difference>::new()
    );
}

#[test]
fn an_edited_file_is_reported() {
    let (fresh, held) = pair("edited");
    write(&held.join("src/a.rs"), "pub struct A(u8);\n");
    assert_eq!(
        differences(&fresh, &held).unwrap(),
        vec![Difference::Edited(PathBuf::from("src/a.rs"))]
    );
}

#[test]
fn a_file_missing_from_the_held_crate_is_reported() {
    let (fresh, held) = pair("missing");
    fs::remove_file(held.join("src/a.rs")).unwrap();
    assert_eq!(
        differences(&fresh, &held).unwrap(),
        vec![Difference::Missing(PathBuf::from("src/a.rs"))]
    );
}

#[test]
fn a_file_no_longer_generated_is_reported() {
    let (fresh, held) = pair("not-generated");
    write(&held.join("src/b.rs"), "pub struct B;\n");
    assert_eq!(
        differences(&fresh, &held).unwrap(),
        vec![Difference::NotGenerated(PathBuf::from("src/b.rs"))]
    );
}

#[test]
fn the_synthesis_state_and_the_held_lock_are_ignored_at_the_top_only() {
    let (fresh, held) = pair("ignored");
    write(
        &fresh.join(".ess-output/state.json"),
        "{\"anchor_id\":\"1\"}",
    );
    write(
        &held.join(".ess-output/state.json"),
        "{\"anchor_id\":\"2\"}",
    );
    write(&held.join("Cargo.lock"), "version = 4\n");
    write(&held.join("src/Cargo.lock"), "version = 4\n");
    assert_eq!(
        differences(&fresh, &held).unwrap(),
        vec![Difference::NotGenerated(PathBuf::from("src/Cargo.lock"))]
    );
}

#[test]
fn each_difference_names_its_kind() {
    assert_eq!(
        Difference::Edited(PathBuf::from("src/a.rs")).to_string(),
        "edited: src/a.rs"
    );
    assert_eq!(
        Difference::Missing(PathBuf::from("src/a.rs")).to_string(),
        "missing: src/a.rs"
    );
    assert_eq!(
        Difference::NotGenerated(PathBuf::from("src/b.rs")).to_string(),
        "not generated: src/b.rs"
    );
}

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn synthesized_root(name: &str) -> PathBuf {
    let root = scratch(name);
    copy_tree(&repository().join("ess"), &root.join("ess"));
    write(&root.join(GENERATED).join("Cargo.lock"), "held lock\n");
    generate(&root).unwrap();
    root
}

#[test]
fn generate_writes_the_crate_keeps_its_lock_and_drops_the_synthesis_state() {
    let root = synthesized_root("generate");
    let crate_dir = root.join(GENERATED);
    assert_eq!(
        fs::read_to_string(crate_dir.join("Cargo.lock")).unwrap(),
        "held lock\n"
    );
    assert!(crate_dir.join("Cargo.toml").is_file());
    assert!(crate_dir.join("PLAN.md").is_file());
    assert!(crate_dir.join("src/lib.rs").is_file());
    assert!(!crate_dir.join(".ess-output").exists());
    let message = drift(&root).unwrap();
    assert!(
        message.starts_with("generated/rust/inbox: no drift"),
        "{message}"
    );
}

#[test]
fn drift_names_generated_rust_and_every_changed_path() {
    let root = synthesized_root("drift-edited");
    let crate_dir = root.join(GENERATED);
    write(&crate_dir.join("src/lib.rs"), "// edited by hand\n");
    fs::remove_file(crate_dir.join("PLAN.md")).unwrap();
    write(&crate_dir.join("src/extra.rs"), "pub struct Extra;\n");
    let error = drift(&root).unwrap_err();
    let first = error.lines().next().unwrap();
    assert!(
        first.starts_with("generated/rust/inbox drifted from a fresh synthesis of ess/"),
        "{error}"
    );
    for line in [
        "  edited: src/lib.rs",
        "  missing: PLAN.md",
        "  not generated: src/extra.rs",
    ] {
        assert!(error.lines().any(|l| l == line), "{line:?} not in {error}");
    }
}

#[test]
fn an_optional_field_added_to_the_specification_is_drift() {
    let root = synthesized_root("drift-specification");
    let domain = root.join("ess/domains/messaging.yaml");
    let text = fs::read_to_string(&domain).unwrap();
    let anchor = "      - {name: adapter, type: inbox.messaging.Adapter}\n";
    assert!(text.contains(anchor));
    let changed = text.replacen(
        anchor,
        &format!("{anchor}      - {{name: label, type: Optional<String>}}\n"),
        1,
    );
    fs::write(&domain, changed).unwrap();
    let error = drift(&root).unwrap_err();
    assert!(error.starts_with("generated/rust/inbox drifted"), "{error}");
    assert!(error.contains("  edited: src/messaging.rs"), "{error}");
}

#[test]
fn a_failed_synthesis_leaves_the_held_crate_alone() {
    let root = synthesized_root("generate-fails");
    let before = fs::read(root.join(GENERATED).join("src/lib.rs")).unwrap();
    write(
        &root.join("ess/domains/messaging.yaml"),
        "not: [an ess document\n",
    );
    assert!(generate(&root).is_err());
    assert_eq!(
        fs::read(root.join(GENERATED).join("src/lib.rs")).unwrap(),
        before
    );
}
