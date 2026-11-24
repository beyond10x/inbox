use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use inbox_xtask::Answer;
use inbox_xtask::spec_compat::{self, Line, Said};

#[derive(Parser)]
#[command(about = "Repository tasks of the inbox")]
struct Cli {
    #[command(subcommand)]
    task: Task,
}

#[derive(Subcommand)]
enum Task {
    /// Rewrite generated/rust/inbox from a fresh synthesis of ess/.
    Generate,
    /// Fail when generated/rust/inbox differs from a fresh synthesis of ess/.
    Drift,
    /// Check the scenario files of ess/scenarios/, then rewrite ess/ess-inputs.yaml from them.
    Scenarios {
        /// Write nothing; fail with one line per path when the manifest does not list each
        /// scenario file once.
        #[arg(long)]
        check: bool,
    },
    /// Write website/src/generated/animation.json from the compiled ess/.
    Animation {
        /// Write nothing; fail with `edited:` or `missing:` when the file differs.
        #[arg(long)]
        check: bool,
        /// Read the compiled IR from this file instead of running `ess specify compile`.
        #[arg(long, value_name = "FILE")]
        ir: Option<PathBuf>,
    },
    /// Fail when an end-user page names a wave, a stage, a story, an epic or a count of scenarios.
    DocsWords {
        /// Files, or directories whose *.md and *.mdx files are read.
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
    /// Fail when ess/ holds a change that is breaking or of unknown compatibility against the
    /// ess/ of the base: the commit INBOX_SPEC_BASE names, else the merge base of HEAD with
    /// origin/main, else with main.
    SpecCompat,
}

/// The tree the binary is run in: the runtime `CARGO_MANIFEST_DIR` (`cargo run` sets it), else
/// the current directory. Never the tree the binary was compiled in.
fn root(current: &Path) -> Result<PathBuf, String> {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from);
    inbox_xtask::repository_root(manifest_dir.as_deref(), current)
}

/// Run one task. A path argument is taken from the current directory.
fn run(task: Task) -> Result<Said, String> {
    let current =
        std::env::current_dir().map_err(|e| format!("read the current directory: {e}"))?;
    let said = |line: String| Answer {
        stdout: vec![line],
        ..Answer::default()
    };
    let answer = match task {
        Task::Generate => inbox_xtask::generate(&root(&current)?)
            .map(|()| said(format!("{} rewritten from ess/", inbox_xtask::GENERATED))),
        Task::Drift => inbox_xtask::drift(&root(&current)?).map(said),
        Task::Scenarios { check } => inbox_xtask::scenarios::scenarios(&root(&current)?, check),
        Task::Animation { check, ir } => {
            let ir = ir.map(|file| current.join(file));
            inbox_xtask::animation::animation(&root(&current)?, ir.as_deref(), check)
        }
        Task::DocsWords { paths } => inbox_xtask::docs_words::docs_words(&paths),
        Task::SpecCompat => {
            let named = std::env::var_os(spec_compat::BASE_VARIABLE)
                .map(|value| value.to_string_lossy().into_owned());
            return Ok(spec_compat::spec_compat(&root(&current)?, named.as_deref()));
        }
    };
    answer.map(Said::from)
}

fn main() -> ExitCode {
    match run(Cli::parse().task) {
        Ok(said) => {
            for line in &said.lines {
                match line {
                    Line::Out(line) => println!("{line}"),
                    Line::Err(line) => eprintln!("{line}"),
                }
            }
            ExitCode::from(said.code)
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
