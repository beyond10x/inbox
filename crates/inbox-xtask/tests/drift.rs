use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use inbox_xtask::{Difference, GENERATED, differences, drift_with, generate_with, repository_root};

use crate::synthesis::synthesize;

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

fn set_mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

#[test]
fn mode_bits_other_than_the_executable_bit_are_no_drift() {
    let (fresh, held) = pair("mode-umask");
    set_mode(&fresh.join("src/a.rs"), 0o644);
    set_mode(&held.join("src/a.rs"), 0o664);
    assert_eq!(
        differences(&fresh, &held).unwrap(),
        Vec::<Difference>::new()
    );
}

#[test]
fn an_executable_bit_the_held_crate_lacks_is_drift() {
    let (fresh, held) = pair("mode-lost");
    set_mode(&fresh.join("src/a.rs"), 0o755);
    set_mode(&held.join("src/a.rs"), 0o644);
    assert_eq!(
        differences(&fresh, &held).unwrap(),
        vec![Difference::Edited(PathBuf::from("src/a.rs"))]
    );
}

#[test]
fn a_build_directory_below_the_top_of_the_crate_is_compared() {
    let (fresh, held) = pair("nested-target");
    write(&held.join("src/target/x.rs"), "pub struct X;\n");
    assert_eq!(
        differences(&fresh, &held).unwrap(),
        vec![Difference::NotGenerated(PathBuf::from("src/target/x.rs"))]
    );
}

#[test]
fn a_symbolic_link_to_a_directory_is_not_followed() {
    let (fresh, held) = pair("directory-link");
    let outside = held.parent().unwrap().join("outside");
    fs::rename(held.join("src"), &outside).unwrap();
    std::os::unix::fs::symlink(&outside, held.join("src")).unwrap();
    assert_eq!(
        differences(&fresh, &held).unwrap(),
        vec![
            Difference::NotGenerated(PathBuf::from("src")),
            Difference::Missing(PathBuf::from("src/a.rs")),
            Difference::Missing(PathBuf::from("src/lib.rs")),
        ]
    );
}

#[test]
fn a_held_crate_that_is_itself_a_symbolic_link_is_refused() {
    let (fresh, held) = pair("crate-link");
    let link = held.parent().unwrap().join("link");
    std::os::unix::fs::symlink(&held, &link).unwrap();
    let error = differences(&fresh, &link).unwrap_err();
    assert!(
        error.ends_with("is a symbolic link, not a directory"),
        "{error}"
    );
}

fn repository_layout(name: &str) -> PathBuf {
    let root = scratch(name);
    fs::create_dir_all(root.join("ess")).unwrap();
    write(&root.join("Cargo.toml"), "[workspace]\n");
    fs::create_dir_all(root.join("crates/inbox-xtask/src")).unwrap();
    root
}

#[test]
fn a_relative_manifest_directory_is_taken_from_the_current_directory() {
    let root = repository_layout("root-relative");
    assert_eq!(
        repository_root(Some(Path::new("crates/inbox-xtask")), &root).unwrap(),
        root
    );
}

#[test]
fn a_directory_holding_ess_but_a_cargo_toml_directory_is_no_root() {
    let root = scratch("root-manifest-directory");
    fs::create_dir_all(root.join("ess")).unwrap();
    fs::create_dir_all(root.join("Cargo.toml")).unwrap();
    let error = repository_root(Some(&root.join("crates/inbox-xtask")), &root).unwrap_err();
    assert!(error.ends_with("it holds no `Cargo.toml`"), "{error}");
}

#[test]
fn the_root_is_found_from_a_directory_below_it() {
    let root = repository_layout("root-walk");
    let below = root.join("crates/inbox-xtask/src");
    assert_eq!(repository_root(None, &below).unwrap(), root);
}

#[test]
fn the_root_is_two_levels_above_the_runtime_manifest_directory() {
    let root = repository_layout("root-manifest");
    let elsewhere = scratch("root-manifest-elsewhere");
    assert_eq!(
        repository_root(Some(&root.join("crates/inbox-xtask")), &elsewhere).unwrap(),
        root
    );
}

/// Started from `/`, which no repository holds: every scratch directory lies inside this one.
#[test]
fn no_root_found_names_what_is_missing() {
    let error = repository_root(None, Path::new("/")).unwrap_err();
    assert!(
        error.contains("holds both `ess/` and `Cargo.toml`"),
        "{error}"
    );
    let lone = scratch("root-none");
    fs::create_dir_all(lone.join("ess")).unwrap();
    let error = repository_root(Some(&lone.join("crates/inbox-xtask")), &lone).unwrap_err();
    assert!(error.ends_with("it holds no `Cargo.toml`"), "{error}");
    let empty = scratch("root-empty");
    let error = repository_root(Some(&empty.join("crates/inbox-xtask")), &empty).unwrap_err();
    assert!(
        error.ends_with("it holds no `ess/` and no `Cargo.toml`"),
        "{error}"
    );
}

#[test]
fn the_binary_outside_any_repository_exits_nonzero_and_says_why() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_inbox-xtask"))
        .arg("drift")
        .env_remove("CARGO_MANIFEST_DIR")
        .current_dir("/")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(
        stderr.contains("holds both `ess/` and `Cargo.toml`"),
        "{stderr}"
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
    generate_with(&root, synthesize).unwrap();
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
    let message = drift_with(&root, synthesize).unwrap();
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
    let error = drift_with(&root, synthesize).unwrap_err();
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
    let error = drift_with(&root, synthesize).unwrap_err();
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
    assert!(generate_with(&root, synthesize).is_err());
    assert_eq!(
        fs::read(root.join(GENERATED).join("src/lib.rs")).unwrap(),
        before
    );
}

#[test]
fn generate_moves_a_crate_left_aside_back_and_keeps_its_lock() {
    let root = scratch("recover-aside");
    copy_tree(&repository().join("ess"), &root.join("ess"));
    let previous = root.join("target/inbox-xtask/previous");
    copy_tree(&repository().join(GENERATED), &previous);
    write(&previous.join("Cargo.lock"), "held lock\n");
    generate_with(&root, synthesize).unwrap();
    assert_eq!(
        fs::read_to_string(root.join(GENERATED).join("Cargo.lock")).unwrap(),
        "held lock\n"
    );
    assert!(!previous.exists());
    assert!(drift_with(&root, synthesize).is_ok());
}

#[test]
fn a_leftover_aside_beside_a_present_crate_is_removed() {
    let root = synthesized_root("stale-aside");
    let previous = root.join("target/inbox-xtask/previous");
    write(&previous.join("stale.rs"), "pub struct Stale;\n");
    generate_with(&root, synthesize).unwrap();
    assert!(!previous.exists());
    assert_eq!(
        fs::read_to_string(root.join(GENERATED).join("Cargo.lock")).unwrap(),
        "held lock\n"
    );
    assert!(!root.join(GENERATED).join("stale.rs").exists());
}

#[test]
fn drift_on_a_tree_without_the_crate_reports_every_file_missing() {
    let root = scratch("no-crate");
    copy_tree(&repository().join("ess"), &root.join("ess"));
    let error = drift_with(&root, synthesize).unwrap_err();
    assert!(error.starts_with("generated/rust/inbox drifted"), "{error}");
    for line in ["  missing: Cargo.toml", "  missing: src/lib.rs"] {
        assert!(error.lines().any(|l| l == line), "{line:?} not in {error}");
    }
    assert!(
        !error.lines().any(|l| l.starts_with("  edited:")),
        "{error}"
    );
}
