// generated from inbox v1
// contract digest 9447d0d3fd8d202100ab11dab84fc004c77f28cae8e3e28a0e0222ae7f780f19
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
eprintln!("unmet startup obligations:\ncaller attribute: inbox\ncaller attribute: principal\ncommand behaviour: inbox.messaging.Expire\ncommand behaviour: inbox.messaging.PostMessage\nexternal branch answer: inbox.messaging.Answer/authority-invalid\nexternal branch answer: inbox.messaging.Answer/deadline-passed\nexternal branch answer: inbox.messaging.Answer/expired-on-arrival\nexternal branch answer: inbox.messaging.Answer/shape-mismatch\nexternal branch answer: inbox.messaging.Answer/signature-invalid\nexternal branch answer: inbox.messaging.Refuse/deadline-passed");
std::process::ExitCode::FAILURE
}
