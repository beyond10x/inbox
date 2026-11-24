//! `tools/test-step.sh`, the test step of `task check`: it runs a command under a bound on wall
//! time, fails when the command runs over it, and prints the ten slowest tests of the JUnit report
//! the command wrote, pass or fail. Each case runs it on a stand-in command that writes a report.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use super::{Run, repository, scratch, shell};
use crate::synthesis::damaged_entry;

fn script() -> PathBuf {
    repository().join("tools/test-step.sh")
}

/// A report of 12 tests in 2 binaries, in no order of time, written as nextest writes one.
const REPORT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="nextest-run" tests="12" failures="1" errors="0" uncounted="0" timestamp="2026-11-22T12:00:00.000+01:00" time="9.500">
    <testsuite name="inbox-cli::main" tests="6" disabled="0" errors="0" failures="1">
        <testcase name="cases::a" classname="inbox-cli::main" timestamp="2026-11-22T12:00:00.000+01:00" time="0.250">
        </testcase>
        <testcase name="cases::b" classname="inbox-cli::main" timestamp="2026-11-22T12:00:00.000+01:00" time="7.125">
            <failure type="test failure">thread 'cases::b' panicked</failure>
        </testcase>
        <testcase name="cases::c" classname="inbox-cli::main" timestamp="2026-11-22T12:00:00.000+01:00" time="0.010">
        </testcase>
        <testcase name="cases::d&lt;e&gt;" classname="inbox-cli::main" timestamp="2026-11-22T12:00:00.000+01:00" time="3.500">
        </testcase>
        <testcase name="epoch::f" classname="inbox-cli::main" timestamp="2026-11-22T12:00:00.000+01:00" time="1.000">
        </testcase>
        <testcase name="epoch::g" classname="inbox-cli::main" timestamp="2026-11-22T12:00:00.000+01:00" time="12.000">
        </testcase>
    </testsuite>
    <testsuite name="inbox-core::main" tests="6" disabled="0" errors="0" failures="0">
        <testcase name="section::h" classname="inbox-core::main" timestamp="2026-11-22T12:00:00.000+01:00" time="0.500"/>
        <testcase name="section::i" classname="inbox-core::main" timestamp="2026-11-22T12:00:00.000+01:00" time="0.005"/>
        <testcase name="section::j" classname="inbox-core::main" timestamp="2026-11-22T12:00:00.000+01:00" time="2.000"/>
        <testcase name="section::k" classname="inbox-core::main" timestamp="2026-11-22T12:00:00.000+01:00" time="0.750"/>
        <testcase name="section::l" classname="inbox-core::main" timestamp="2026-11-22T12:00:00.000+01:00" time="60.002"/>
        <testcase name="section::m" classname="inbox-core::main" timestamp="2026-11-22T12:00:00.000+01:00" time="0.125"/>
    </testsuite>
</testsuites>
"#;

/// The lines the script prints for [`REPORT`]: the ten slowest, slowest first; `cases::c` and
/// `section::i` are the 2 fastest and left out.
const SLOWEST: [&str; 11] = [
    "test-step: the 10 slowest tests",
    "    60.002 s  inbox-core::main section::l",
    "    12.000 s  inbox-cli::main epoch::g",
    "     7.125 s  inbox-cli::main cases::b",
    "     3.500 s  inbox-cli::main cases::d<e>",
    "     2.000 s  inbox-core::main section::j",
    "     1.000 s  inbox-cli::main epoch::f",
    "     0.750 s  inbox-core::main section::k",
    "     0.500 s  inbox-core::main section::h",
    "     0.250 s  inbox-cli::main cases::a",
    "     0.125 s  inbox-core::main section::m",
];

/// A directory with the report as `fixture.xml`; the stand-in command copies it to `junit.xml`.
fn fixture(name: &str) -> PathBuf {
    let dir = scratch(name);
    fs::write(dir.join("fixture.xml"), REPORT).unwrap();
    dir
}

fn step(dir: &Path, seconds: &str, command: &str) -> (Run, Duration) {
    let start = Instant::now();
    let run = shell(dir, &script(), &[seconds, "junit.xml", "sh", "-c", command]);
    (run, start.elapsed())
}

fn slowest(run: &Run) -> Vec<&str> {
    run.stdout_lines()
        .into_iter()
        .skip_while(|line| !line.starts_with("test-step: the 10 slowest"))
        .take(SLOWEST.len())
        .collect()
}

#[test]
fn a_passing_run_prints_its_ten_slowest_tests_slowest_first() {
    let dir = fixture("test-step-pass");
    let (run, _) = step(&dir, "30", "cp fixture.xml junit.xml");
    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(slowest(&run), SLOWEST, "{}", run.stdout);
    assert_eq!(run.stderr, "");
}

#[test]
fn a_failing_run_keeps_its_exit_status_and_prints_its_ten_slowest_tests() {
    let dir = fixture("test-step-fail");
    let (run, _) = step(&dir, "30", "cp fixture.xml junit.xml; exit 100");
    assert_eq!(run.code, 100, "{}{}", run.stdout, run.stderr);
    assert_eq!(slowest(&run), SLOWEST, "{}", run.stdout);
}

#[test]
fn a_run_over_the_bound_is_ended_and_fails_and_says_so() {
    let dir = fixture("test-step-over");
    let (run, took) = step(&dir, "2", "cp fixture.xml junit.xml; sleep 60");
    assert!(took < Duration::from_secs(20), "the step took {took:?}");
    assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
    assert_eq!(slowest(&run), SLOWEST, "{}", run.stdout);
    assert!(
        run.stderr
            .contains("test-step: the run took longer than its bound of 2 s and was ended"),
        "{}",
        run.stderr
    );
}

#[test]
fn a_stale_report_is_removed_before_the_run() {
    let dir = fixture("test-step-stale");
    fs::copy(dir.join("fixture.xml"), dir.join("junit.xml")).unwrap();
    let (run, _) = step(&dir, "30", "true");
    assert_eq!(run.code, 1, "{}{}", run.stdout, run.stderr);
    assert!(slowest(&run).is_empty(), "{}", run.stdout);
    assert!(
        run.stderr
            .contains("test-step: the run wrote no JUnit report at junit.xml"),
        "{}",
        run.stderr
    );
}

#[test]
fn a_bound_that_is_no_number_of_seconds_is_a_usage_error() {
    let dir = scratch("test-step-usage");
    for arguments in [
        &[][..],
        &["120", "junit.xml"][..],
        &["2m", "junit.xml", "true"][..],
        &["0", "junit.xml", "true"][..],
    ] {
        let run = shell(&dir, &script(), arguments);
        assert_eq!(run.code, 2, "{arguments:?}: {}{}", run.stdout, run.stderr);
        assert_eq!(run.stdout, "");
        assert!(
            run.stderr.starts_with("usage: test-step.sh"),
            "{}",
            run.stderr
        );
    }
}

/// A package of 2 tests, each in a process group of its own under cargo-nextest: one ignores
/// SIGTERM, one waits on a child that ignores SIGTERM. Each writes a counter 5 times a second for
/// 30 s at most, and its process id, so a process that outlives the step is seen and ended.
const STUBBORN: &str = r#"
unsafe extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
}

fn beat(name: &str) {
    std::fs::write(format!("{name}.pid"), std::process::id().to_string()).unwrap();
    for i in 0..150 {
        std::fs::write(name, i.to_string()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

#[test]
fn ignores_term() {
    unsafe {
        signal(15, 1);
    }
    beat("test-alive");
}

#[test]
fn child_ignores_term() {
    std::process::Command::new("sh")
        .args(["-c", "echo $$ > child-alive.pid; trap '' TERM; i=0; \
            while [ $i -lt 150 ]; do echo $i > child-alive; i=$((i+1)); sleep 0.2; done"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
}
"#;

/// The files of the fixture package, by their path in it: a manifest, an empty library, the
/// tests of [`STUBBORN`] and a copy of this repository's `.config/nextest.toml`.
fn stubborn_package() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        (
            "Cargo.toml",
            b"[package]\nname = \"stubborn\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n"
                .to_vec(),
        ),
        ("src/lib.rs", Vec::new()),
        ("tests/stubborn.rs", STUBBORN.as_bytes().to_vec()),
        (
            ".config/nextest.toml",
            fs::read(repository().join(".config/nextest.toml")).unwrap(),
        ),
    ]
}

fn write_package(dir: &Path) {
    for (path, bytes) in stubborn_package() {
        let file = dir.join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, bytes).unwrap();
    }
}

/// `program` with `args` in `dir`, with none of the variables the outer cargo-nextest gives this
/// test.
fn fixture_command(dir: &Path, program: &str, args: &[&str]) -> Command {
    let mut command = Command::new(program);
    command.args(args).current_dir(dir).stdin(Stdio::null());
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().contains("NEXTEST") {
            command.env_remove(name);
        }
    }
    command
}

/// What builds and lists the fixture: `binaries.json` and `metadata.json` of an entry.
const LISTS: [(&str, &[&str]); 2] = [
    (
        "binaries.json",
        &[
            "nextest",
            "list",
            "--offline",
            "--list-type",
            "binaries-only",
            "--message-format",
            "json",
        ],
    ),
    (
        "metadata.json",
        &["metadata", "--offline", "--format-version", "1"],
    ),
];

/// What the build of the fixture reads besides the package and the commands of [`LISTS`].
struct KeyInputs {
    /// This repository's `rust-toolchain.toml`.
    toolchain: Vec<u8>,
    /// What `cargo-nextest nextest --version` prints.
    nextest: Vec<u8>,
    /// This repository's `.cargo/config.toml`, which cargo reads for the package under
    /// `target/tmp/`; `None` when there is none.
    cargo_config: Option<Vec<u8>>,
    /// `RUSTFLAGS` as this process sees it, which the build inherits; `None` when it is unset.
    rustflags: Option<OsString>,
    /// `CARGO_ENCODED_RUSTFLAGS` the same way.
    encoded_rustflags: Option<OsString>,
}

impl KeyInputs {
    fn of_this_run() -> Self {
        let version = Command::new("cargo-nextest")
            .args(["nextest", "--version"])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(version.status.success(), "cargo-nextest --version failed");
        let config = repository().join(".cargo/config.toml");
        Self {
            toolchain: fs::read(repository().join("rust-toolchain.toml")).unwrap(),
            nextest: version.stdout,
            cargo_config: config.is_file().then(|| fs::read(&config).unwrap()),
            rustflags: std::env::var_os("RUSTFLAGS"),
            encoded_rustflags: std::env::var_os("CARGO_ENCODED_RUSTFLAGS"),
        }
    }
}

/// The SHA-256, in hex, of what the built fixture is made from: the files of the package, the
/// commands of [`LISTS`] and the [`KeyInputs`] of this run.
fn stubborn_key() -> String {
    stubborn_key_of(&KeyInputs::of_this_run())
}

/// The key of a fixture built from `inputs`. A member that is absent (`None`) goes in as a
/// marker of its own, so an empty file or an empty variable is another key than none.
fn stubborn_key_of(inputs: &KeyInputs) -> String {
    let mut digest = Sha256::new();
    let mut field = |bytes: &[u8]| {
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
    };
    for (path, bytes) in stubborn_package() {
        field(path.as_bytes());
        field(&bytes);
    }
    for (name, args) in LISTS {
        field(name.as_bytes());
        field(args.join("\0").as_bytes());
    }
    field(&inputs.toolchain);
    field(&inputs.nextest);
    let rustflags =
        |value: &Option<OsString>| value.as_ref().map(|v| v.as_encoded_bytes().to_vec());
    for member in [
        inputs.cargo_config.clone(),
        rustflags(&inputs.rustflags),
        rustflags(&inputs.encoded_rustflags),
    ] {
        match member {
            Some(bytes) => {
                field(b"set");
                field(&bytes);
            }
            None => field(b"absent"),
        }
    }
    digest
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The inputs of a key, made up, with none of the 3 optional members.
fn made_up_inputs() -> KeyInputs {
    KeyInputs {
        toolchain: b"[toolchain]\nchannel = \"1.0.0\"\n".to_vec(),
        nextest: b"cargo-nextest 0.0.0\n".to_vec(),
        cargo_config: None,
        rustflags: None,
        encoded_rustflags: None,
    }
}

/// The keys of `inputs` with one member set by `set` to each of `values`, in order; the first
/// is `inputs` as given.
fn keys_with(set: impl Fn(&mut KeyInputs, Option<&str>), values: &[Option<&str>]) -> Vec<String> {
    values
        .iter()
        .map(|value| {
            let mut inputs = made_up_inputs();
            set(&mut inputs, *value);
            stubborn_key_of(&inputs)
        })
        .collect()
}

/// Every key of `keys` differs from the others, and the key of the same inputs is the same.
fn all_differ(member: &str, keys: &[String], again: &str) {
    for (i, key) in keys.iter().enumerate() {
        for other in &keys[i + 1..] {
            assert_ne!(
                key, other,
                "the key of the fixture is the same for 2 values of {member}"
            );
        }
    }
    assert_eq!(keys[0], again, "the key of the same inputs changed");
}

#[test]
fn the_key_of_the_fixture_holds_the_cargo_configuration_of_the_repository() {
    let set = |inputs: &mut KeyInputs, value: Option<&str>| {
        inputs.cargo_config = value.map(|text| text.as_bytes().to_vec());
    };
    let keys = keys_with(
        set,
        &[None, Some(""), Some("[profile.dev]\ndebug = false\n")],
    );
    all_differ(
        ".cargo/config.toml",
        &keys,
        &stubborn_key_of(&made_up_inputs()),
    );
}

#[test]
fn the_key_of_the_fixture_holds_rustflags() {
    let set = |inputs: &mut KeyInputs, value: Option<&str>| {
        inputs.rustflags = value.map(OsString::from);
    };
    let keys = keys_with(set, &[None, Some(""), Some("-C opt-level=1")]);
    all_differ("RUSTFLAGS", &keys, &stubborn_key_of(&made_up_inputs()));
}

#[test]
fn the_key_of_the_fixture_holds_cargo_encoded_rustflags() {
    let set = |inputs: &mut KeyInputs, value: Option<&str>| {
        inputs.encoded_rustflags = value.map(OsString::from);
    };
    let mut keys = keys_with(set, &[None, Some(""), Some("-C\x1fopt-level=1")]);
    // The same value in `RUSTFLAGS` is another key: the 2 variables are 2 members.
    let mut inputs = made_up_inputs();
    inputs.rustflags = Some(OsString::from("-C\x1fopt-level=1"));
    keys.push(stubborn_key_of(&inputs));
    all_differ(
        "CARGO_ENCODED_RUSTFLAGS",
        &keys,
        &stubborn_key_of(&made_up_inputs()),
    );
}

/// The fixture package built once per test run, as the synthesis of `ess/` is
/// (`tests/synthesis.rs`): the entry `<CARGO_TARGET_TMPDIR>/test-step-stubborn/<key>/` holds the
/// package, its `target/` with the test binaries, and `binaries.json` and `metadata.json` that
/// cargo-nextest runs them from. The first process that finds no entry takes an exclusive lock
/// on `<key>.lock`, builds into `<key>.partial` (a leftover of a killed run is removed first) and
/// renames it to `<key>`; the others wait on the lock and read it. The lists name the paths of
/// `<key>.partial`: a run remaps them (`--workspace-remap`, `--target-dir-remap`). `task test`
/// makes the entry before its bounded run (`the_fixture_of_the_stubborn_run_is_built`). An entry
/// that lost a file is not built anew; its readers say so with [`damage`].
fn built_stubborn() -> PathBuf {
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR")).join("test-step-stubborn");
    let key = stubborn_key();
    let done = cache.join(&key);
    if done.is_dir() {
        return done;
    }
    fs::create_dir_all(&cache).unwrap();
    let lock = fs::File::create(cache.join(format!("{key}.lock"))).unwrap();
    lock.lock().unwrap();
    if !done.is_dir() {
        let partial = cache.join(format!("{key}.partial"));
        if partial.symlink_metadata().is_ok() {
            fs::remove_dir_all(&partial).unwrap();
        }
        write_package(&partial);
        for (name, args) in LISTS {
            let built = fixture_command(&partial, "cargo", args)
                .env("CARGO_TARGET_DIR", partial.join("target"))
                .output()
                .unwrap();
            assert!(
                built.status.success(),
                "{}",
                String::from_utf8_lossy(&built.stderr)
            );
            fs::write(partial.join(name), &built.stdout).unwrap();
        }
        fs::rename(&partial, &done).unwrap();
    }
    done
}

/// The setup step of `task test` runs this case before the bounded run, so the fixture of
/// `a_run_over_the_bound_leaves_no_test_process_running` is built before it starts.
#[test]
fn the_fixture_of_the_stubborn_run_is_built() {
    let built = built_stubborn();
    if let Some(line) = damage(&built) {
        panic!("{line}");
    }
    for (path, bytes) in stubborn_package() {
        assert_eq!(fs::read(built.join(path)).unwrap(), bytes, "{path}");
    }
    let binaries = fs::read_to_string(built.join("binaries.json")).unwrap();
    assert!(binaries.contains("\"stubborn\""), "{binaries}");
    let metadata = fs::read_to_string(built.join("metadata.json")).unwrap();
    assert!(metadata.contains("\"workspace_root\""), "{metadata}");
}

/// The line a case fails with when the built fixture `entry` lacks a file a run reads: a file of
/// the package, a list of [`LISTS`] or the test binary. `None` when it holds them all.
fn damage(entry: &Path) -> Option<String> {
    let missing = stubborn_package()
        .into_iter()
        .map(|(path, _)| path)
        .chain(LISTS.map(|(name, _)| name))
        .find(|path| !entry.join(path).is_file());
    if let Some(path) = missing {
        return Some(damaged_entry(entry, path));
    }
    let binary = fs::read_dir(entry.join("target/debug/deps")).is_ok_and(|deps| {
        deps.filter_map(Result::ok).any(|file| {
            let name = file.file_name();
            let name = name.to_string_lossy();
            name.starts_with("stubborn-") && !name.contains('.')
        })
    });
    (!binary).then(|| {
        format!(
            "no test binary of the fixture was built: {}",
            damaged_entry(entry, "target/debug/deps/stubborn-<hash>")
        )
    })
}

/// A built fixture with every file but the test binary, then without `binaries.json`: each time
/// the line names the entry by its path in the tree and how to have it built anew.
#[test]
fn a_damaged_fixture_is_named_with_the_way_to_build_it_anew() {
    let entry = Path::new(env!("CARGO_TARGET_TMPDIR")).join("test-step-damaged/entry");
    if entry.exists() {
        fs::remove_dir_all(&entry).unwrap();
    }
    write_package(&entry);
    for (name, _) in LISTS {
        fs::write(entry.join(name), "{}").unwrap();
    }
    fs::create_dir_all(entry.join("target/debug/deps")).unwrap();
    let tree = repository().display().to_string();
    let sentence = "removing that directory makes the next run build it anew";
    let line = damage(&entry).expect("an entry without its test binary is damaged");
    assert!(
        line.contains("no test binary of the fixture was built")
            && line.contains("target/tmp/test-step-damaged/entry")
            && line.contains(sentence)
            && !line.contains(&tree),
        "{line}"
    );
    fs::write(
        entry.join("target/debug/deps/stubborn-0123456789abcdef"),
        "",
    )
    .unwrap();
    assert_eq!(damage(&entry), None);
    fs::remove_file(entry.join("binaries.json")).unwrap();
    let line = damage(&entry).expect("an entry without binaries.json is damaged");
    assert!(
        line.contains("binaries.json")
            && line.contains("target/tmp/test-step-damaged/entry")
            && line.contains(sentence)
            && !line.contains(&tree),
        "{line}"
    );
}

/// The step as `task test` runs it (this repository's `.config/nextest.toml`, real
/// cargo-nextest, test binaries built before the step), over its bound while a test ignores
/// SIGTERM and another test's child does:
/// when the step has ended, no process of the run is still running, and the 10 slowest tests
/// are printed. Adversary case, 2026-11-22: the step SIGKILLs cargo-nextest 10 s after SIGTERM,
/// the same 10 s as nextest's grace period before it SIGKILLs a test, so nextest dies first and
/// the test runs on as an orphan of the user's init, outside the build pool, and no JUnit report
/// is written; a test's child that ignores SIGTERM outlives its test once the test has ended.
///
/// The test binaries are the ones [`built_stubborn`] made before the run; the step runs them with
/// a copy of the package of its own as the workspace, so the tests write their beats and nextest
/// its report there and never into the entry.
#[test]
fn a_run_over_the_bound_leaves_no_test_process_running() {
    let built = built_stubborn();
    // Said here, where nextest would only say it failed to run a file it was given.
    if let Some(line) = damage(&built) {
        panic!("{line}");
    }
    let dir = scratch("test-step-stubborn");
    write_package(&dir);
    let report = dir.join("target/nextest/default/junit.xml");
    // Built and listed before the step, so its bound of 2 s holds no start of `cargo` and no wait
    // on the lock of the package cache: on a loaded host that start took up to 3.7 s and the bound
    // ended the run before either test ran.
    let binaries = built.join("binaries.json");
    let metadata = built.join("metadata.json");
    let target = built.join("target");

    let output = fixture_command(
        &dir,
        "sh",
        &[
            script().to_str().unwrap(),
            "2",
            report.to_str().unwrap(),
            "cargo-nextest",
            "nextest",
            "run",
            "--binaries-metadata",
            binaries.to_str().unwrap(),
            "--cargo-metadata",
            metadata.to_str().unwrap(),
            "--workspace-remap",
            dir.to_str().unwrap(),
            "--target-dir-remap",
            target.to_str().unwrap(),
        ],
    )
    .output()
    .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let beats =
        || ["test-alive", "child-alive"].map(|name| fs::read_to_string(dir.join(name)).ok());
    let before = beats();
    std::thread::sleep(Duration::from_millis(1500));
    let after = beats();
    let survivors: Vec<&str> = ["test-alive", "child-alive"]
        .into_iter()
        .zip(before.iter().zip(after.iter()))
        .filter(|(_, (before, after))| before != after)
        .map(|(name, _)| name)
        .collect();
    for name in ["test-alive", "child-alive"] {
        if let Ok(pid) = fs::read_to_string(dir.join(format!("{name}.pid"))) {
            let _ = std::process::Command::new("kill")
                .args(["-9", pid.trim()])
                .stderr(std::process::Stdio::null())
                .status();
        }
    }

    assert_eq!(output.status.code(), Some(1), "{stdout}{stderr}");
    assert!(
        before.iter().all(Option::is_some),
        "both tests ran: {before:?}\n{stdout}{stderr}"
    );
    assert!(
        survivors.is_empty(),
        "still running after the step ended: {survivors:?}\n{stdout}{stderr}"
    );
    assert!(
        stdout.contains("test-step: the 10 slowest tests"),
        "{stdout}{stderr}"
    );
}
