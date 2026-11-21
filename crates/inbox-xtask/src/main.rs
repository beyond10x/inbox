use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

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
}

/// The tree the binary is run in: the runtime `CARGO_MANIFEST_DIR` (`cargo run` sets it), else
/// the current directory. Never the tree the binary was compiled in.
fn run(task: Task) -> Result<String, String> {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from);
    let current =
        std::env::current_dir().map_err(|e| format!("read the current directory: {e}"))?;
    let root = inbox_xtask::repository_root(manifest_dir.as_deref(), &current)?;
    match task {
        Task::Generate => inbox_xtask::generate(&root)
            .map(|()| format!("{} rewritten from ess/", inbox_xtask::GENERATED)),
        Task::Drift => inbox_xtask::drift(&root),
    }
}

fn main() -> ExitCode {
    match run(Cli::parse().task) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
