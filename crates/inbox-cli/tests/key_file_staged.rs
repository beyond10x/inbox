//! The staged place of the key file (story:an-unreadable-epochs-file-keeps-its-other-lines):
//! `inbox keyfile add-key` and `inbox keyfile add-delegation` write the key file to
//! `<file>.<pid>.new` and rename it over the file; a link planted at that place is removed, not
//! followed. Every key is made at run time.
//!
//! The staged name holds the process id of the command, which does not exist before it starts.
//! The case starts it as `sh -c 'ln -s other "$1.$$.new" && shift && exec "$@"'`: the shell plants
//! the link under its own id, and `exec` keeps that id for the binary, so the link is at the
//! binary's staged place before the binary runs.

use crate::support;

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::process::{Command, Stdio};

use support::{ALICE, BIN, Dir, Run, key_new, keys_only, run};

/// The bytes of the other file of the directory, the target of the planted link.
const OTHER: &[u8] = b"another file of the directory\n";

/// `args` run as `command(dir)` runs them, by a shell that first plants a link at
/// `<file>.<its pid>.new` pointing at `other` and then becomes the binary.
fn planted(dir: &Path, file: &str, args: &[&str]) -> Run {
    let mut shell = Command::new("sh");
    shell.current_dir(dir).stdin(Stdio::null());
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("INBOX_") {
            shell.env_remove(name);
        }
    }
    let output = shell
        .env("HOME", dir)
        .env("XDG_STATE_HOME", dir.join("state"))
        .args([
            "-c",
            r#"ln -s other "$1.$$.new" && shift && exec "$@""#,
            "sh",
            file,
            BIN,
        ])
        .args(args)
        .output()
        .expect("sh starts");
    Run {
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8(output.stdout).expect("stdout is UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr is UTF-8"),
    }
}

/// A directory holding a copy of `from`'s operator key and, when `key_file` names a mode, of its
/// key file with that mode.
fn copy(from: &Dir, name: &str, key_file: Option<u32>) -> Dir {
    let dir = Dir::new(name);
    fs::copy(from.join("operator.key"), dir.join("operator.key")).expect("the key is copied");
    if let Some(mode) = key_file {
        fs::copy(from.join("keys.json"), dir.join("keys.json")).expect("the key file is copied");
        fs::set_permissions(dir.join("keys.json"), fs::Permissions::from_mode(mode)).unwrap();
    }
    dir
}

/// For each command that writes the key file, over a key file of mode 0640 and over none: with a
/// link planted at the staged place that points at another file of the directory, the command
/// ends as it does without the link, the key file is a regular file with the mode and the bytes
/// it has without the link (0640 kept), the other file is byte for byte unchanged and keeps its
/// mode, and no staged entry is left.
#[test]
fn a_link_at_the_staged_place_of_the_key_file_is_removed_not_followed() {
    let (fixture, _) = keys_only("key-file-staged");
    let carol = key_new(&fixture.path, "carol.key", None);
    let add_key = [
        "keyfile",
        "add-key",
        "--file",
        "keys.json",
        "--operator-secret-key-file",
        "operator.key",
        "--key-id",
        "carol-1",
        "--principal",
        "agent:test/carol",
        "--public-key",
        &carol,
        "--valid-until",
        "2099-01-01T00:00:00Z",
    ];
    let add_delegation = [
        "keyfile",
        "add-delegation",
        "--file",
        "keys.json",
        "--operator-secret-key-file",
        "operator.key",
        "--delegation-id",
        "d-1",
        "--agent",
        ALICE,
        "--on-behalf-of",
        "human:test/owner",
        "--valid-until",
        "2099-01-01T00:00:00Z",
    ];
    for (args, key_file) in [
        (&add_key[..], Some(0o640)),
        (&add_key[..], None),
        (&add_delegation[..], Some(0o640)),
        (&add_delegation[..], None),
    ] {
        let row = format!(
            "{} over {}",
            args[1],
            key_file.map_or_else(|| "no key file".to_owned(), |mode| format!("mode {mode:o}"))
        );
        let without = copy(&fixture, "key-file-staged-without", key_file);
        let unlinked = run(&without.path, args, &[]).ok();
        let with = copy(&fixture, "key-file-staged-with", key_file);
        fs::write(with.join("other"), OTHER).unwrap();
        fs::set_permissions(with.join("other"), fs::Permissions::from_mode(0o600)).unwrap();

        let linked = planted(&with.path, "keys.json", args);

        assert_eq!(linked.status, unlinked.status, "{row}: {linked:#?}");
        assert_eq!(linked.stdout, unlinked.stdout, "{row}");
        assert_eq!(linked.stderr, unlinked.stderr, "{row}");
        let written = fs::symlink_metadata(with.join("keys.json")).unwrap();
        let expected = fs::metadata(without.join("keys.json")).unwrap();
        assert!(written.file_type().is_file(), "{row}: {written:?}");
        assert_eq!(written.mode() & 0o7777, expected.mode() & 0o7777, "{row}");
        if let Some(mode) = key_file {
            assert_eq!(written.mode() & 0o7777, mode, "{row}");
        }
        assert_eq!(
            fs::read(with.join("keys.json")).unwrap(),
            fs::read(without.join("keys.json")).unwrap(),
            "{row}"
        );
        let other = fs::symlink_metadata(with.join("other")).unwrap();
        assert!(other.file_type().is_file(), "{row}: {other:?}");
        assert_eq!(other.mode() & 0o7777, 0o600, "{row}");
        assert_eq!(fs::read(with.join("other")).unwrap(), OTHER, "{row}");
        let staged: Vec<String> = fs::read_dir(&with.path)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".new"))
            .collect();
        assert!(staged.is_empty(), "{row}: {staged:?}");
    }
}
