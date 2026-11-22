use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;

use inbox_xtask::{Difference, GENERATED, differences};

use crate::synthesis::path;

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("adversary")
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

/// A copy of `ess/` and of the checked-in crate, with `src/lib.rs` of the crate edited by hand.
fn drifted_copy(name: &str) -> PathBuf {
    let root = scratch(name);
    copy_tree(&repository().join("ess"), &root.join("ess"));
    copy_tree(&repository().join(GENERATED), &root.join(GENERATED));
    write(&root.join("Cargo.toml"), "[workspace]\n");
    fs::create_dir_all(root.join("crates/inbox-xtask")).unwrap();
    write(
        &root.join(GENERATED).join("src/lib.rs"),
        "// edited by hand\n",
    );
    root
}

/// The acceptance rests on the exit status of `inbox-xtask drift`: `task check` fails only
/// because the binary exits non-zero. No case in tests/drift.rs runs the binary.
#[test]
fn the_drift_binary_exits_nonzero_on_drift() {
    let root = drifted_copy("binary-exit");
    let output = Command::new(env!("CARGO_BIN_EXE_inbox-xtask"))
        .arg("drift")
        .env("PATH", path())
        .env("CARGO_MANIFEST_DIR", root.join("crates/inbox-xtask"))
        .current_dir(&root)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "drift exited {} on a drifted crate; stderr: {stderr}",
        output.status
    );
    assert!(
        stderr.starts_with("generated/rust/inbox drifted from a fresh synthesis of ess/"),
        "{stderr}"
    );
    assert!(
        stderr.lines().any(|l| l == "  edited: src/lib.rs"),
        "{stderr}"
    );
}

/// Run outside `cargo run`, the binary falls back to the manifest directory it was compiled in,
/// so it checks that tree and not the one it is run from.
#[test]
fn the_binary_run_outside_cargo_checks_the_tree_it_is_run_from() {
    let root = drifted_copy("binary-outside-cargo");
    let output = Command::new(env!("CARGO_BIN_EXE_inbox-xtask"))
        .arg("drift")
        .env("PATH", path())
        .env_remove("CARGO_MANIFEST_DIR")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "drift run from a drifted tree exited {}; stdout: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout)
    );
}

/// git records the executable bit; the comparison reads bytes only.
#[test]
fn a_changed_mode_is_drift() {
    let (fresh, held) = pair("mode");
    let file = held.join("src/a.rs");
    let mut permissions = fs::metadata(&file).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&file, permissions).unwrap();
    assert_ne!(
        differences(&fresh, &held).unwrap(),
        Vec::<Difference>::new()
    );
}

/// git records a symbolic link as a link; the comparison follows it and reads the bytes behind it.
#[test]
fn a_symbolic_link_in_place_of_a_generated_file_is_drift() {
    let (fresh, held) = pair("symlink");
    let outside = held.parent().unwrap().join("outside.rs");
    write(&outside, "pub struct A;\n");
    fs::remove_file(held.join("src/a.rs")).unwrap();
    symlink(&outside, held.join("src/a.rs")).unwrap();
    assert_ne!(
        differences(&fresh, &held).unwrap(),
        Vec::<Difference>::new()
    );
}

/// `cargo build` or an editor's `cargo check` inside the crate (it declares its own `[workspace]`)
/// writes `target/` there; git does not track it, `.gitignore` ignores only `/target`.
#[test]
fn a_build_directory_inside_the_held_crate_is_not_drift() {
    let (fresh, held) = pair("build-directory");
    write(&held.join("target/debug/.fingerprint/x"), "x\n");
    assert_eq!(
        differences(&fresh, &held).unwrap(),
        Vec::<Difference>::new()
    );
}
