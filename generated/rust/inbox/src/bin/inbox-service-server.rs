// generated from inbox v1
// contract digest a07be6c10394398db00443c7f14d6b2b93014d8a5f9ab6f7516db37c386cf373
// do not edit: regenerate with `ess synthesize --layout crate`

//! Ephemeral generated server. Durable storage and production authentication remain ports.
use clap::Parser;

#[derive(Parser)]
struct Options {
#[arg(long, default_value = "127.0.0.1:8080")]
listen: String,
#[arg(long, default_value = "none", value_parser = ["none", "actor-header"])]
callers: String,
#[arg(long = "static")]
static_root: Option<std::path::PathBuf>,
}

fn main() -> std::process::ExitCode {
let options = Options::parse();
let _ = options;
eprintln!("unmet startup obligations:\ncaller attribute: inbox\ncaller attribute: principal\ncommand behaviour: inbox.messaging.Expire\ncommand behaviour: inbox.messaging.PostMessage\nexternal branch answer: inbox.messaging.Answer/authority-invalid\nexternal branch answer: inbox.messaging.Answer/shape-mismatch\nexternal branch answer: inbox.messaging.Answer/signature-invalid");
std::process::ExitCode::FAILURE
}
