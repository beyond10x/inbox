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

/// The repository root: two levels above this crate's manifest. `cargo run` sets
/// `CARGO_MANIFEST_DIR` for the tree it runs in, so a binary built in another tree is not misled.
fn repository_root() -> PathBuf {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    manifest_dir
        .ancestors()
        .nth(2)
        .expect("crates/inbox-xtask sits two levels below the repository root")
        .to_path_buf()
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let root = repository_root();
    let result = match cli.task {
        Task::Generate => inbox_xtask::generate(&root)
            .map(|()| format!("{} rewritten from ess/", inbox_xtask::GENERATED)),
        Task::Drift => inbox_xtask::drift(&root),
    };
    match result {
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
