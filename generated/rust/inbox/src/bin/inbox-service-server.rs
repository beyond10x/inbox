// generated from inbox v1
// contract digest e34e82a0a81f7c01ba3d91fa287c7a723f89e6d1fbdd928b462676dc857fd14a
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
eprintln!("unmet startup obligations:\ncaller attribute: inbox\ncaller attribute: principal\ncommand behaviour: inbox.messaging.Collect\ncommand behaviour: inbox.messaging.Expire\ncommand behaviour: inbox.messaging.PostMessage\nexternal branch answer: inbox.messaging.Answer/authority-invalid\nexternal branch answer: inbox.messaging.Answer/deadline-passed\nexternal branch answer: inbox.messaging.Answer/expired-on-arrival\nexternal branch answer: inbox.messaging.Answer/shape-mismatch\nexternal branch answer: inbox.messaging.Answer/signature-invalid\nexternal branch answer: inbox.messaging.MarkDelivered/deadline-passed\nexternal branch answer: inbox.messaging.MarkRead/deadline-passed\nexternal branch answer: inbox.messaging.Refuse/deadline-passed");
std::process::ExitCode::FAILURE
}
