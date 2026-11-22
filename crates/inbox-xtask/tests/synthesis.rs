//! The synthesis of the unchanged `ess/`, made once per test run and read by every case that
//! needs it.
//!
//! The cases of `generate` and `drift` need what `ess generate synthesize` writes from a copy of
//! the repository's `ess/`, and one synthesis takes seconds. cargo-nextest runs each case in a
//! process of its own, so the product is a cache entry on disk,
//! `<CARGO_TARGET_TMPDIR>/synthesis/<key>/`, made by the first case that asks for it:
//!
//! - `ess/`: the specification it was made from, byte for byte;
//! - `crate/`: what the real `ess` wrote from it, with the arguments of `inbox_xtask::SYNTHESIS`;
//! - `bin/ess`: a POSIX shell script in place of `ess`, first on `PATH` (`path()`) for a case
//!   that starts the `inbox-xtask` binary. Called with exactly those arguments in a directory
//!   whose `ess/` has the bytes of the entry's `ess/` (`diff -r`), it copies `crate/` to the
//!   output directory; any other call runs the real `ess`.
//!
//! A case that calls `generate_with` or `drift_with` passes `synthesize`, which answers the
//! same way without starting a process: `crate/` when the root's `ess/` has the files of the
//! entry's `ess/` (path, executable bit, bytes), else the real `ess`. Either way a case that
//! changes the specification before it synthesizes runs its own synthesis.
//!
//! The key is the SHA-256 of all the synthesis reads: the script's text, the arguments, the real
//! `ess` (its `--version`, path, length and modification time) and every file of `ess/` (path,
//! executable bit, bytes). Another specification or another `ess` release is another entry. A
//! process works the key out once (`OnceLock`); the product itself is only the entry on disk.
//!
//! Several processes ask at once. An entry directory only ever appears complete: the first
//! process that finds none takes an exclusive lock on `<key>.lock`, writes `<key>.partial` (a
//! leftover of a killed run is removed first) and renames it to `<key>`; the others wait on the
//! lock, find the entry, and read it. `task test` runs `the_entry_of_the_repository_is_made`
//! alone before its bounded run, so no case of the run waits for the synthesis.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Barrier, OnceLock};
use std::thread;

use inbox_xtask::{SPECIFICATION, SYNTHESIS, synthesize_with};
use sha2::{Digest, Sha256};

/// `bin/ess` of an entry; `@REAL@` is the real `ess`, `@COUNT@` the number of arguments and
/// `@ARGUMENTS@` all but the last (the output directory), each after one space.
const SCRIPT: &str = r#"#!/bin/sh
# The synthesis of one specification, answered from the cache entry this script lies in; any
# other call runs the real ess. Written by the tests of inbox-xtask (tests/synthesis.rs).
entry=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P) || exit 2
real='@REAL@'
if [ $# -eq @COUNT@ ]; then
    n=0
    asked=
    for argument in "$@"; do
        n=$((n + 1))
        if [ "$n" -lt $# ]; then
            asked="$asked $argument"
        else
            out=$argument
        fi
    done
    if [ "$asked" = '@ARGUMENTS@' ] && diff -r @SPECIFICATION@ "$entry/@SPECIFICATION@" > /dev/null 2>&1; then
        mkdir -p -- "$out" && cp -R -- "$entry/crate/." "$out/"
        exit $?
    fi
fi
exec "$real" "$@"
"#;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

/// The entry of the repository's `ess/` and the `ess` on `PATH`, made on first use, and that
/// `ess`.
fn repository_entry() -> &'static (PathBuf, PathBuf) {
    static ENTRY: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();
    ENTRY.get_or_init(|| {
        let cache = Path::new(env!("CARGO_TARGET_TMPDIR")).join("synthesis");
        let real = fs::canonicalize(on_path("ess")).unwrap();
        let entry = entry(&cache, &repository().join(SPECIFICATION), &real);
        (entry, real)
    })
}

/// The synthesis to pass to `generate_with` and `drift_with`: `crate/` of the entry of the
/// repository's `ess/` when `root/ess` has its files, else the real `ess`.
pub fn synthesize(root: &Path, out: &Path) -> Result<(), String> {
    let (entry, real) = repository_entry();
    answer(entry, real, root, out)
}

/// `PATH` with `bin/` of the entry of the repository's `ess/` first, for a case that starts the
/// `inbox-xtask` binary.
pub fn path() -> OsString {
    let first = repository_entry().0.join("bin");
    let rest = env::var_os("PATH").unwrap_or_default();
    env::join_paths(std::iter::once(first).chain(env::split_paths(&rest))).unwrap()
}

/// What `synthesize` does, for the entry `entry` made with the program `real`.
fn answer(entry: &Path, real: &Path, root: &Path, out: &Path) -> Result<(), String> {
    let spec = root.join(SPECIFICATION);
    if !spec.is_dir() || files(&spec) != files(&entry.join(SPECIFICATION)) {
        return synthesize_with(root, real, out);
    }
    if out.exists() {
        fs::remove_dir_all(out).map_err(|e| format!("remove {}: {e}", out.display()))?;
    }
    copy_tree(&entry.join("crate"), out);
    Ok(())
}

fn on_path(program: &str) -> PathBuf {
    let path = env::var_os("PATH").unwrap_or_default();
    env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|file| {
            file.metadata()
                .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        })
        .unwrap_or_else(|| panic!("no `{program}` on PATH"))
}

/// The entry for `spec` (a directory named `ess`) and the program `real` under `cache`, made
/// when it is not there. Only the holder of the lock writes; a directory named by the key is
/// complete, because it appears by one rename.
fn entry(cache: &Path, spec: &Path, real: &Path) -> PathBuf {
    let key = key(spec, real);
    let done = cache.join(&key);
    if done.is_dir() {
        return done;
    }
    fs::create_dir_all(cache).unwrap();
    let lock = fs::File::create(cache.join(format!("{key}.lock"))).unwrap();
    lock.lock().unwrap();
    if !done.is_dir() {
        let partial = cache.join(format!("{key}.partial"));
        build(&partial, spec, real);
        fs::rename(&partial, &done).unwrap();
    }
    done
}

/// Write an entry into `partial`: the copy of `spec`, the synthesis the real `ess` makes of it,
/// and the script.
fn build(partial: &Path, spec: &Path, real: &Path) {
    if partial.symlink_metadata().is_ok() {
        fs::remove_dir_all(partial).unwrap();
    }
    copy_tree(spec, &partial.join(SPECIFICATION));
    synthesize_with(partial, real, &partial.join("crate"))
        .unwrap_or_else(|e| panic!("synthesis of {} for the cache: {e}", spec.display()));
    let script = partial.join("bin/ess");
    fs::create_dir_all(script.parent().unwrap()).unwrap();
    fs::write(&script, script_text(real)).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
}

fn script_text(real: &Path) -> String {
    let real = real.to_str().expect("the path of ess is UTF-8");
    assert!(
        !real.contains('\''),
        "the path of ess holds a quote: {real}"
    );
    let arguments = SYNTHESIS;
    assert!(
        arguments
            .iter()
            .all(|a| !a.contains([' ', '\'', '"', '\\', '$', '`'])),
        "an argument of the synthesis needs quoting: {arguments:?}"
    );
    SCRIPT
        .replace("@REAL@", real)
        .replace("@COUNT@", &(SYNTHESIS.len() + 1).to_string())
        .replace(
            "@ARGUMENTS@",
            &arguments
                .iter()
                .map(|a| format!(" {a}"))
                .collect::<String>(),
        )
        .replace("@SPECIFICATION@", SPECIFICATION)
}

/// The SHA-256, in hex, of all the synthesis of `spec` by `real` reads.
fn key(spec: &Path, real: &Path) -> String {
    let mut digest = Sha256::new();
    let mut field = |bytes: &[u8]| {
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
    };
    field(SCRIPT.as_bytes());
    field(SYNTHESIS.join("\0").as_bytes());
    let real = fs::canonicalize(real).unwrap();
    let meta = fs::metadata(&real).unwrap();
    let modified = meta
        .modified()
        .unwrap()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    field(real.as_os_str().as_encoded_bytes());
    field(&meta.len().to_le_bytes());
    field(&modified.as_nanos().to_le_bytes());
    let version = Command::new(&real).arg("--version").output().unwrap();
    assert!(
        version.status.success(),
        "{} --version failed",
        real.display()
    );
    field(&version.stdout);
    for (relative, executable, bytes) in files(spec) {
        field(relative.as_os_str().as_encoded_bytes());
        field(&[u8::from(executable)]);
        field(&bytes);
    }
    digest
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Every file below `dir`, sorted by its path relative to `dir`: the path, whether an executable
/// bit is set, the bytes.
fn files(dir: &Path) -> Vec<(PathBuf, bool, Vec<u8>)> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        for item in fs::read_dir(&current).unwrap() {
            let path = item.unwrap().path();
            let meta = fs::metadata(&path).unwrap();
            if meta.is_dir() {
                pending.push(path);
            } else {
                let relative = path.strip_prefix(dir).unwrap().to_path_buf();
                found.push((
                    relative,
                    meta.permissions().mode() & 0o111 != 0,
                    fs::read(&path).unwrap(),
                ));
            }
        }
    }
    found.sort();
    found
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

/// A program in place of the real `ess`, in `@DIR@`: `--version` prints the text of
/// `version`; a synthesis appends a line to `calls`, waits the seconds `pause` names (none when
/// it is absent), and writes `Cargo.toml` and `src/lib.rs` (the bytes of `ess/spec.yaml`) to the
/// output directory.
const STUB: &str = r#"#!/bin/sh
d='@DIR@'
if [ "$1" = --version ]; then
    cat "$d/version"
    exit 0
fi
echo call >> "$d/calls"
if [ -f "$d/pause" ]; then
    sleep "$(cat "$d/pause")"
fi
for argument in "$@"; do
    out=$argument
done
mkdir -p "$out/src"
printf '[package]\n' > "$out/Cargo.toml"
cat ess/spec.yaml > "$out/src/lib.rs"
"#;

fn stub(dir: &Path) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("version"), "stub 1\n").unwrap();
    let program = dir.join("ess");
    fs::write(&program, STUB.replace("@DIR@", dir.to_str().unwrap())).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    program
}

fn calls(stub: &Path) -> usize {
    fs::read_to_string(stub.parent().unwrap().join("calls"))
        .map(|text| text.lines().count())
        .unwrap_or(0)
}

struct Setup {
    cache: PathBuf,
    spec: PathBuf,
    real: PathBuf,
    dir: PathBuf,
}

fn setup(name: &str) -> Setup {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("synthesis-cases")
        .join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    let spec = dir.join("spec/ess");
    fs::create_dir_all(&spec).unwrap();
    fs::write(spec.join("spec.yaml"), "spec: one\n").unwrap();
    Setup {
        cache: dir.join("cache"),
        real: stub(&dir.join("stub")),
        spec,
        dir,
    }
}

/// The setup step of `task test` runs this case alone before the bounded run, so the entry of
/// the repository's `ess/` is there before the cases that read it start.
#[test]
fn the_entry_of_the_repository_is_made() {
    let (entry, real) = repository_entry();
    assert_eq!(
        files(&entry.join(SPECIFICATION)),
        files(&repository().join(SPECIFICATION))
    );
    assert!(entry.join("crate/Cargo.toml").is_file());
    assert!(entry.join("crate/src/lib.rs").is_file());
    let script = fs::read_to_string(entry.join("bin/ess")).unwrap();
    assert!(
        script.contains(&format!("real='{}'", real.display())),
        "{script}"
    );
}

/// A repository root of its own holding a copy of `spec` as `ess/`.
fn root_with(spec: &Path, at: &Path) -> PathBuf {
    copy_tree(spec, &at.join(SPECIFICATION));
    at.to_path_buf()
}

#[test]
fn the_unchanged_specification_is_synthesized_once_for_all_who_ask_at_once() {
    let s = Arc::new(setup("once"));
    fs::write(s.dir.join("stub/pause"), "0.3\n").unwrap();
    let gate = Arc::new(Barrier::new(8));
    let readers: Vec<_> = (0..8)
        .map(|i| {
            let s = Arc::clone(&s);
            let gate = Arc::clone(&gate);
            thread::spawn(move || {
                gate.wait();
                let done = entry(&s.cache, &s.spec, &s.real);
                let root = root_with(&s.spec, &s.dir.join(format!("root-{i}")));
                let out = root.join("out");
                if i % 2 == 0 {
                    answer(&done, &s.real, &root, &out).unwrap();
                } else {
                    synthesize_with(&root, &done.join("bin/ess"), &out).unwrap();
                }
                (
                    fs::read_to_string(out.join("Cargo.toml")).unwrap(),
                    fs::read_to_string(out.join("src/lib.rs")).unwrap(),
                )
            })
        })
        .collect();
    for reader in readers {
        let (manifest, lib) = reader.join().unwrap();
        assert_eq!(manifest, "[package]\n");
        assert_eq!(lib, "spec: one\n");
    }
    assert_eq!(calls(&s.real), 1, "the real ess ran more than once");
}

type Synthesis<'a> = dyn Fn(&Path, &Path) -> Result<(), String> + 'a;

#[test]
fn a_changed_specification_runs_its_own_synthesis() {
    let s = setup("changed");
    let done = entry(&s.cache, &s.spec, &s.real);
    assert_eq!(calls(&s.real), 1);
    let by_script = |root: &Path, out: &Path| synthesize_with(root, &done.join("bin/ess"), out);
    let in_process = |root: &Path, out: &Path| answer(&done, &s.real, root, out);
    let ways: [(&str, &Synthesis); 2] = [("script", &by_script), ("in-process", &in_process)];
    for (way, synthesize) in ways {
        let root = root_with(&s.spec, &s.dir.join(way));
        let out = root.join("out");
        let before = calls(&s.real);
        synthesize(&root, &out).unwrap();
        assert_eq!(calls(&s.real), before, "{way}: the unchanged one ran ess");
        fs::write(root.join("ess/spec.yaml"), "spec: two\n").unwrap();
        synthesize(&root, &out).unwrap();
        assert_eq!(
            calls(&s.real),
            before + 1,
            "{way}: the cache answered a changed specification"
        );
        assert_eq!(
            fs::read_to_string(out.join("src/lib.rs")).unwrap(),
            "spec: two\n"
        );
        fs::write(root.join("ess/added.yaml"), "added: yes\n").unwrap();
        fs::write(root.join("ess/spec.yaml"), "spec: one\n").unwrap();
        synthesize(&root, &out).unwrap();
        assert_eq!(
            calls(&s.real),
            before + 2,
            "{way}: the cache answered an added file"
        );
    }
    let version = Command::new(done.join("bin/ess"))
        .arg("--version")
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(version.stdout).unwrap(), "stub 1\n");
}

#[test]
fn another_specification_or_another_ess_release_is_another_entry() {
    let s = setup("key");
    let first = entry(&s.cache, &s.spec, &s.real);
    assert_eq!(entry(&s.cache, &s.spec, &s.real), first);
    assert_eq!(calls(&s.real), 1);
    fs::write(s.dir.join("stub/version"), "stub 2\n").unwrap();
    let released = entry(&s.cache, &s.spec, &s.real);
    assert_ne!(released, first);
    fs::write(s.spec.join("spec.yaml"), "spec: three\n").unwrap();
    let changed = entry(&s.cache, &s.spec, &s.real);
    assert_ne!(changed, released);
    assert_eq!(calls(&s.real), 3);
}

#[test]
fn a_leftover_of_a_killed_build_is_replaced() {
    let s = setup("leftover");
    let key = key(&s.spec, &s.real);
    let leftover = s.cache.join(format!("{key}.partial"));
    fs::create_dir_all(leftover.join("crate")).unwrap();
    fs::write(leftover.join("crate/torn.rs"), "torn\n").unwrap();
    let done = entry(&s.cache, &s.spec, &s.real);
    assert!(!done.join("crate/torn.rs").exists());
    assert!(done.join("crate/src/lib.rs").is_file());
    assert!(!leftover.exists());
}
