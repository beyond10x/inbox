//! `tools/test-step.sh`, the test step of `task check`: it runs a command under a bound on wall
//! time, fails when the command runs over it, and prints the ten slowest tests of the JUnit report
//! the command wrote, pass or fail. Each case runs it on a stand-in command that writes a report.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::{Run, repository, scratch, shell};

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

/// The step as `task test` runs it (this repository's `.config/nextest.toml`, real
/// cargo-nextest, test binaries built before the step), over its bound while a test ignores
/// SIGTERM and another test's child does:
/// when the step has ended, no process of the run is still running, and the 10 slowest tests
/// are printed. Adversary case, 2026-11-22: the step SIGKILLs cargo-nextest 10 s after SIGTERM,
/// the same 10 s as nextest's grace period before it SIGKILLs a test, so nextest dies first and
/// the test runs on as an orphan of the user's init, outside the build pool, and no JUnit report
/// is written; a test's child that ignores SIGTERM outlives its test once the test has ended.
#[test]
fn a_run_over_the_bound_leaves_no_test_process_running() {
    let dir = scratch("test-step-stubborn");
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"stubborn\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/lib.rs"), "").unwrap();
    fs::create_dir_all(dir.join("tests")).unwrap();
    fs::write(dir.join("tests/stubborn.rs"), STUBBORN).unwrap();
    fs::create_dir_all(dir.join(".config")).unwrap();
    fs::copy(
        repository().join(".config/nextest.toml"),
        dir.join(".config/nextest.toml"),
    )
    .unwrap();
    let target = dir.join("target");
    let report = target.join("nextest/default/junit.xml");
    // In the fixture, with none of the variables the outer cargo-nextest gives this test.
    let fixture_command = |program: &str, args: &[&str]| {
        let mut command = std::process::Command::new(program);
        command
            .args(args)
            .current_dir(&dir)
            .env("CARGO_TARGET_DIR", &target)
            .stdin(std::process::Stdio::null());
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().contains("NEXTEST") {
                command.env_remove(name);
            }
        }
        command
    };
    // Built and listed before the step, so its bound of 2 s holds no start of `cargo` and no wait
    // on the lock of the package cache: on a loaded host that start took up to 3.7 s and the bound
    // ended the run before either test ran.
    let cargo_json = |name: &str, args: &[&str]| {
        let built = fixture_command("cargo", args).output().unwrap();
        assert!(
            built.status.success(),
            "{}",
            String::from_utf8_lossy(&built.stderr)
        );
        let path = dir.join(name);
        fs::write(&path, &built.stdout).unwrap();
        path.to_str().unwrap().to_owned()
    };
    let binaries = cargo_json(
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
    );
    let metadata = cargo_json(
        "metadata.json",
        &["metadata", "--offline", "--format-version", "1"],
    );

    let output = fixture_command(
        "sh",
        &[
            script().to_str().unwrap(),
            "2",
            report.to_str().unwrap(),
            "cargo-nextest",
            "nextest",
            "run",
            "--binaries-metadata",
            &binaries,
            "--cargo-metadata",
            &metadata,
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
