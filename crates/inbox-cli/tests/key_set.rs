//! The key set of the cases (`support::keys_only`, `support::World`): an operator key, the keys
//! of alice and bob and a key file holding both, made by the binary itself once per test run, not
//! once per case, and copied into the directory of each case that asks for it.

use crate::support;

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Barrier;

use support::{ALICE, BOB, Dir, key_set_in, keys_only};

/// The files of a key set.
const FILES: [&str; 4] = ["operator.key", "alice.key", "bob.key", "keys.json"];

fn inode(path: &Path) -> u64 {
    fs::metadata(path).expect("the file is there").ino()
}

/// Two directories get the same key set, each as files of its own: a case that changes one of
/// them changes neither another case's nor the set the next case gets.
#[test]
fn two_directories_get_one_key_set_as_copies_of_their_own() {
    let (first, operator) = keys_only("key-set-first");
    let (second, again) = keys_only("key-set-second");
    assert_eq!(
        operator, again,
        "the key set is made once, not once per case"
    );
    for file in FILES {
        let (one, other) = (first.join(file), second.join(file));
        assert_eq!(
            fs::read(&one).expect("the file is there"),
            fs::read(&other).expect("the file is there"),
            "{file}"
        );
        assert_ne!(
            inode(&one),
            inode(&other),
            "{file} is a file of each directory's own"
        );
    }
    for file in ["operator.key", "alice.key", "bob.key"] {
        let mode = fs::metadata(second.join(file))
            .expect("the file is there")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "{file} is a secret key file");
    }

    fs::write(first.join("keys.json"), "changed\n").expect("written");
    fs::remove_file(first.join("alice.key")).expect("removed");
    let (third, _) = keys_only("key-set-third");
    for dir in [&second, &third] {
        assert_ne!(
            fs::read_to_string(dir.join("keys.json")).expect("the key file is there"),
            "changed\n"
        );
        assert!(dir.join("alice.key").is_file());
    }
}

/// Processes that ask at once for a key set that is not there yet get one set, made once: the
/// cache holds one complete entry and nothing partial, and its key file names the keys of alice
/// and bob. That the service takes the set is held by every case of a `World`.
#[test]
fn a_key_set_asked_for_at_once_is_made_once() {
    let cache = Dir::new("key-set-cache");
    let barrier = Barrier::new(4);
    let answers: Vec<(PathBuf, String)> = std::thread::scope(|scope| {
        let asked: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    let entry = key_set_in(&cache.path);
                    let operator = fs::read_to_string(entry.join("operator.pub"))
                        .expect("the operator public key is there");
                    (entry, operator)
                })
            })
            .collect();
        asked
            .into_iter()
            .map(|answer| answer.join().expect("the key set is answered"))
            .collect()
    });
    assert!(
        answers.iter().all(|answer| *answer == answers[0]),
        "{answers:#?}"
    );
    let mut entries: Vec<String> = fs::read_dir(&cache.path)
        .expect("the cache is there")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| !name.ends_with(".lock"))
        .collect();
    entries.sort();
    let entry = answers[0].0.file_name().expect("a name");
    assert_eq!(
        entries,
        [entry.to_string_lossy()],
        "one entry, nothing partial"
    );

    let keys = fs::read_to_string(answers[0].0.join("keys.json")).expect("the key file is there");
    for (key_id, principal) in [("alice-1", ALICE), ("bob-1", BOB)] {
        assert!(keys.contains(key_id) && keys.contains(principal), "{keys}");
    }
}
