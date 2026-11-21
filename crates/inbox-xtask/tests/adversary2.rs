use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use inbox_xtask::{GENERATED, generate};

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("adversary2")
        .join(name);
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

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

/// `task generate` on a tree that has `ess/` and `generated/` but no `generated/rust/` yet: the
/// first synthesis of the crate, as this unit itself made it, or after `generated/rust` was
/// removed to start over. `replace` creates the parent, but the root check refuses the tree first.
#[test]
fn generate_creates_the_crate_where_generated_rust_does_not_exist_yet() {
    let root = scratch("bootstrap");
    copy_tree(&repository().join("ess"), &root.join("ess"));
    copy_tree(
        &repository().join("generated/openapi"),
        &root.join("generated/openapi"),
    );
    write(&root.join("Cargo.toml"), "[workspace]\n");
    fs::create_dir_all(root.join("crates/inbox-xtask")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_inbox-xtask"))
        .arg("generate")
        .env("CARGO_MANIFEST_DIR", root.join("crates/inbox-xtask"))
        .current_dir(&root)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "generate exited {}; stderr: {stderr}",
        output.status
    );
    assert!(root.join(GENERATED).join("src/lib.rs").is_file());
}

/// A run that failed to move the held crate back (or was killed between the two moves) leaves
/// the crate, with its `Cargo.lock`, only in `target/inbox-xtask/previous`; the error says so.
/// The next `generate` removes that directory before reading it, so the held lock is gone and
/// `cargo metadata` resolves a new one.
#[test]
fn generate_after_a_failed_restore_keeps_the_held_lock() {
    let root = scratch("leftover-previous");
    copy_tree(&repository().join("ess"), &root.join("ess"));
    fs::create_dir_all(root.join("generated/rust")).unwrap();
    let previous = root.join("target/inbox-xtask/previous");
    copy_tree(&repository().join(GENERATED), &previous);
    write(&previous.join("Cargo.lock"), "held lock\n");
    let result = generate(&root);
    let lock = fs::read_to_string(root.join(GENERATED).join("Cargo.lock")).ok();
    assert!(
        lock.as_deref() == Some("held lock\n") || previous.join("Cargo.lock").is_file(),
        "generate returned {result:?}; the held lock is neither in the crate ({lock:?}) nor left in {}",
        previous.display()
    );
}

/// A relative `CARGO_MANIFEST_DIR` two components deep resolves to the empty path, which passes
/// the root check (relative to the current directory) and then fails as if `ess` were missing.
#[test]
fn a_relative_manifest_directory_still_reports_the_drift() {
    let root = scratch("relative-manifest");
    copy_tree(&repository().join("ess"), &root.join("ess"));
    copy_tree(&repository().join(GENERATED), &root.join(GENERATED));
    write(&root.join("Cargo.toml"), "[workspace]\n");
    write(
        &root.join(GENERATED).join("src/lib.rs"),
        "// edited by hand\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_inbox-xtask"))
        .arg("drift")
        .env("CARGO_MANIFEST_DIR", "crates/inbox-xtask")
        .current_dir(&root)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("generated/rust/inbox drifted from a fresh synthesis of ess/"),
        "exit {}; stderr: {stderr}",
        output.status
    );
}
