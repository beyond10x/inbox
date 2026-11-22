//! `inbox`: the HTTP service of the inbox on a loopback address (`inbox serve`), and the client
//! that talks to it with the caller's own key.
//!
//! Exit statuses: 0 done; 1 the service refused (the model's error name for a declared refusal,
//! the service's own `refused` text otherwise); 2 wrong usage; 3 transport failure; 4 a local
//! file that cannot be read, written or created.

#![forbid(unsafe_code)]

mod client;
mod commands;
mod failure;
mod ids;
mod keys;
mod outbox;
mod output;
#[cfg(test)]
mod reference;
mod serve;

use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::client::Service;
use crate::commands::{AnswerInput, Client, PostInput};
use crate::failure::{Failure, USAGE};
use crate::output::Done;

/// The inbox: one durable, addressable inbox per agent or person.
#[derive(Debug, Parser)]
#[command(name = "inbox", version)]
struct Cli {
    /// Print one JSON document on standard output instead of text.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Serve the inbox on a loopback address until SIGINT or SIGTERM. The backend is
    /// INBOX_BACKEND: `memory` (the default) is the only one.
    Serve(ServeArgs),
    /// Key pairs.
    #[command(subcommand)]
    Key(KeyCommand),
    /// The key file the service reads: entries signed with the operator key.
    #[command(subcommand)]
    Keyfile(KeyfileCommand),
    /// Open the caller's own inbox (again after every restart of the service).
    Open {
        #[command(flatten)]
        client: ClientArgs,
        /// The wake surface the inbox is read through.
        #[arg(long, default_value = "terminal")]
        adapter: String,
    },
    /// Close the caller's own inbox.
    Close {
        #[command(flatten)]
        client: ClientArgs,
    },
    /// Post a note, a request or a decision, signed with the caller's key. Prints the message id.
    Post(PostArgs),
    /// What an inbox holds that its owner has not read.
    Pending {
        #[command(flatten)]
        client: ClientArgs,
        /// The inbox; the caller's own by default.
        #[arg(long)]
        inbox: Option<String>,
    },
    /// `pending`, held open by the service until it has a row or the wait ends.
    Wait {
        #[command(flatten)]
        client: ClientArgs,
        /// The inbox; the caller's own by default.
        #[arg(long)]
        inbox: Option<String>,
        /// The longest wait in seconds; the service cuts it to its own --max-wait.
        #[arg(long, default_value_t = 60)]
        seconds: u64,
    },
    /// Mark a message read: a note is handled and removed, a request or decision waits for its
    /// answer.
    Read {
        #[command(flatten)]
        client: ClientArgs,
        /// The message id.
        message_id: String,
    },
    /// Answer a request or a decision with a reply envelope signed with the caller's key.
    Answer(AnswerArgs),
    /// Refuse a message and say why.
    Refuse {
        #[command(flatten)]
        client: ClientArgs,
        /// The message id.
        message_id: String,
        /// Why.
        #[arg(long)]
        reason: String,
    },
    /// Every live message the caller sent, in whatever state.
    Outbox {
        #[command(flatten)]
        client: ClientArgs,
    },
    /// Take the answer or the refusal of a message the caller sent; it leaves the outbox file.
    Collect {
        #[command(flatten)]
        client: ClientArgs,
        /// The message id.
        message_id: String,
    },
    /// Every live message of one thread.
    Thread {
        #[command(flatten)]
        client: ClientArgs,
        /// The thread id.
        thread: String,
    },
    /// The answered messages of one thread.
    Replies {
        #[command(flatten)]
        client: ClientArgs,
        /// The thread id.
        thread: String,
    },
    /// Post again, unchanged, every envelope the outbox file keeps for the caller that did not
    /// arrive at the service's current start: for after a restart of the service. An envelope
    /// whose post the current start answered (`posted` or `Duplicate`) is left alone; a note its
    /// recipient read before a restart is delivered once more by the first repost after it.
    Repost {
        #[command(flatten)]
        client: ClientArgs,
    },
}

/// Where a client command finds the service, the caller's key and the outbox file.
#[derive(Debug, Args)]
struct ClientArgs {
    /// The service: http://<host>:<port>.
    #[arg(long, env = "INBOX_URL")]
    url: Option<String>,
    /// The file of the caller's secret key (`inbox key new --key-id … --principal …`); it names
    /// the key id and the principal, which is also the address of the caller's inbox.
    #[arg(long, env = "INBOX_SECRET_KEY_FILE")]
    secret_key_file: Option<PathBuf>,
    /// The outbox file; default $XDG_STATE_HOME/inbox/outbox.jsonl, else
    /// $HOME/.local/state/inbox/outbox.jsonl.
    #[arg(long, env = "INBOX_OUTBOX")]
    outbox: Option<PathBuf>,
}

impl ClientArgs {
    fn client(&self) -> Result<Client, Failure> {
        Ok(Client {
            service: Service::from_url(self.url.as_deref())?,
            caller: keys::caller(self.secret_key_file.as_deref())?,
            outbox: outbox::Outbox::locate(self.outbox.as_deref())?,
        })
    }
}

#[derive(Debug, Args)]
struct ServeArgs {
    /// The address to bind: a loopback address only. Port 0 takes an ephemeral port; the first
    /// line printed is the URL served.
    #[arg(long)]
    listen: SocketAddr,
    /// The key file: who each request's key id is.
    #[arg(long)]
    key_file: PathBuf,
    /// The operator public key (44 characters of base64) that signed the key file.
    #[arg(long, env = "INBOX_OPERATOR_PUBLIC_KEY")]
    operator_public_key: String,
    /// The most connections served at once.
    #[arg(long, default_value_t = inbox_server::Config::MAX_CONNECTIONS)]
    max_connections: usize,
    /// Seconds a request may take to arrive in full.
    #[arg(long, default_value_t = inbox_server::Config::REQUEST_TIMEOUT.as_secs())]
    request_timeout: u64,
    /// The longest body read, in bytes.
    #[arg(long, default_value_t = inbox::server::http::MAX_BODY)]
    max_request_bytes: usize,
    /// The message count at which a post is answered 503.
    #[arg(long, default_value_t = inbox_server::Config::MAX_MESSAGES)]
    max_messages: usize,
    /// The longest wait on `pending`, in seconds.
    #[arg(long, default_value_t = inbox_server::Config::MAX_WAIT.as_secs())]
    max_wait: u64,
    /// Seconds between two sweeps of the expiry task.
    #[arg(long, default_value_t = inbox_server::Config::SWEEP_INTERVAL.as_secs())]
    sweep_interval: u64,
    /// Seconds an answer taken on silence waits for its sender.
    #[arg(long, default_value_t = inbox_server::Config::ANSWER_RETENTION.as_secs())]
    answer_retention: u64,
    /// For the conformance run only, hidden from `--help` and the reference: the instant (RFC
    /// 3339) the service's clock reads at the start, from which it moves on by the system clock's
    /// elapsed time. Without it, the system clock. The deadline of a post, an answer and a
    /// refusal, the request window and the expiry task read this clock.
    #[arg(long, value_name = "INSTANT", hide = true)]
    clock_start: Option<String>,
}

#[derive(Debug, Subcommand)]
enum KeyCommand {
    /// Make a key pair: the secret goes to --out (mode 0600, never overwritten); prints the
    /// public key.
    New {
        /// The secret key file to create.
        #[arg(long)]
        out: PathBuf,
        /// The key id of this key's entry in the key file (a client key).
        #[arg(long, requires = "principal")]
        key_id: Option<String>,
        /// The principal of this key's entry in the key file (a client key).
        #[arg(long, requires = "key_id")]
        principal: Option<String>,
    },
}

#[derive(Debug, Args)]
struct KeyfileArgs {
    /// The key file; created when absent.
    #[arg(long)]
    file: PathBuf,
    /// The operator's secret key file.
    #[arg(long)]
    operator_secret_key_file: PathBuf,
    /// The end of the entry's validity (RFC 3339); 30 days from now by default.
    #[arg(long)]
    valid_until: Option<String>,
}

#[derive(Debug, Subcommand)]
enum KeyfileCommand {
    /// Append a key entry: this public key signs for this principal.
    AddKey {
        #[command(flatten)]
        target: KeyfileArgs,
        /// The key id; an id is never reused.
        #[arg(long)]
        key_id: String,
        /// The principal the key signs for.
        #[arg(long)]
        principal: String,
        /// The public key (44 characters of base64), as `inbox key new` printed it.
        #[arg(long)]
        public_key: String,
    },
    /// Append a delegation entry: the agent may speak for another principal.
    AddDelegation {
        #[command(flatten)]
        target: KeyfileArgs,
        /// The delegation id; an id is never reused.
        #[arg(long)]
        delegation_id: String,
        /// The principal that speaks.
        #[arg(long)]
        agent: String,
        /// The principal it speaks for.
        #[arg(long)]
        on_behalf_of: String,
    },
}

/// Where an envelope's body comes from.
#[derive(Debug, Args)]
#[group(required = true, multiple = false)]
struct BodyArgs {
    /// The body.
    #[arg(long)]
    body: Option<String>,
    /// A file holding the body; `-` for standard input.
    #[arg(long)]
    body_file: Option<PathBuf>,
}

impl BodyArgs {
    fn read(&self) -> Result<String, Failure> {
        commands::body(self.body.as_deref(), self.body_file.as_deref())
    }
}

#[derive(Debug, Args)]
struct PostArgs {
    #[command(flatten)]
    client: ClientArgs,
    /// The inbox posted to.
    #[arg(long)]
    to: String,
    /// note, request or decision.
    #[arg(long, default_value = "note")]
    kind: String,
    #[command(flatten)]
    body: BodyArgs,
    /// The thread; the message id by default.
    #[arg(long)]
    thread: Option<String>,
    /// The sender's key of this logical message; the message id by default.
    #[arg(long)]
    dedup_key: Option<String>,
    /// The principal whose word this is; the caller by default.
    #[arg(long)]
    on_behalf_of: Option<String>,
    /// The delegation that lets the caller speak for --on-behalf-of.
    #[arg(long)]
    authority: Option<String>,
    /// What a reply must look like: free, lines=<n> or schema=<reference>.
    #[arg(long)]
    reply_shape: Option<String>,
    /// An option of a decision, <letter>=<text>; repeated.
    #[arg(long = "option", value_parser = option_pair)]
    options: Vec<(String, String)>,
    /// The option the sender recommends.
    #[arg(long)]
    recommended_option: Option<String>,
    /// The option silence takes at `expires`.
    #[arg(long)]
    default_on_silence: Option<String>,
    /// The id of the decision request where another system records it.
    #[arg(long)]
    request_ref: Option<String>,
    /// When the message is removed (RFC 3339); created plus 24 hours by default.
    #[arg(long)]
    expires: Option<String>,
}

fn option_pair(text: &str) -> Result<(String, String), String> {
    text.split_once('=')
        .map(|(letter, text)| (letter.to_owned(), text.to_owned()))
        .ok_or_else(|| format!("`{text}` is not <letter>=<text>"))
}

#[derive(Debug, Args)]
struct AnswerArgs {
    #[command(flatten)]
    client: ClientArgs,
    /// The message answered.
    message_id: String,
    #[command(flatten)]
    body: BodyArgs,
    /// The option of a decision chosen.
    #[arg(long)]
    option: Option<String>,
    /// The principal whose word this is; the caller by default.
    #[arg(long)]
    on_behalf_of: Option<String>,
    /// The delegation that lets the caller speak for --on-behalf-of.
    #[arg(long)]
    authority: Option<String>,
    /// How long the answer waits for the sender to collect it (RFC 3339); created plus 24 hours
    /// by default.
    #[arg(long)]
    expires: Option<String>,
}

fn run(command: Command, json: bool) -> Result<Option<Done>, Failure> {
    let done = match command {
        Command::Serve(args) => {
            let settings = serve::Settings {
                listen: args.listen,
                key_file: &args.key_file,
                operator_public_key: &args.operator_public_key,
                max_connections: args.max_connections,
                request_timeout: args.request_timeout,
                max_request_bytes: args.max_request_bytes,
                max_messages: args.max_messages,
                max_wait: args.max_wait,
                sweep_interval: args.sweep_interval,
                answer_retention: args.answer_retention,
                clock_start: args.clock_start.as_deref(),
            };
            let backend = std::env::var("INBOX_BACKEND").ok();
            serve::serve(&settings, backend.as_deref(), json)?;
            return Ok(None);
        }
        Command::Key(KeyCommand::New {
            out,
            key_id,
            principal,
        }) => keys::new(&out, key_id.as_deref().zip(principal.as_deref()))?,
        Command::Keyfile(KeyfileCommand::AddKey {
            target,
            key_id,
            principal,
            public_key,
        }) => keys::add_key(
            &keys::KeyFileTarget {
                file: &target.file,
                operator: &target.operator_secret_key_file,
            },
            &key_id,
            &principal,
            &public_key,
            &keys::valid_until(target.valid_until.as_deref())?,
        )?,
        Command::Keyfile(KeyfileCommand::AddDelegation {
            target,
            delegation_id,
            agent,
            on_behalf_of,
        }) => keys::add_delegation(
            &keys::KeyFileTarget {
                file: &target.file,
                operator: &target.operator_secret_key_file,
            },
            &delegation_id,
            &agent,
            &on_behalf_of,
            &keys::valid_until(target.valid_until.as_deref())?,
        )?,
        Command::Open { client, adapter } => commands::open(&client.client()?, &adapter)?,
        Command::Close { client } => commands::close(&client.client()?)?,
        Command::Post(args) => {
            let body = args.body.read()?;
            commands::post(
                &args.client.client()?,
                &PostInput {
                    to: args.to,
                    kind: args.kind,
                    thread: args.thread,
                    dedup_key: args.dedup_key,
                    on_behalf_of: args.on_behalf_of,
                    authority: args.authority,
                    reply_shape: args.reply_shape,
                    options: args.options,
                    recommended_option: args.recommended_option,
                    default_on_silence: args.default_on_silence,
                    request_ref: args.request_ref,
                    expires: args.expires,
                    body,
                },
            )?
        }
        Command::Pending { client, inbox } => {
            commands::pending(&client.client()?, inbox.as_deref(), None)?
        }
        Command::Wait {
            client,
            inbox,
            seconds,
        } => commands::pending(&client.client()?, inbox.as_deref(), Some(seconds))?,
        Command::Read { client, message_id } => commands::read(&client.client()?, &message_id)?,
        Command::Answer(args) => {
            let body = args.body.read()?;
            commands::answer(
                &args.client.client()?,
                &AnswerInput {
                    in_reply_to: args.message_id,
                    option: args.option,
                    on_behalf_of: args.on_behalf_of,
                    authority: args.authority,
                    expires: args.expires,
                    body,
                },
            )?
        }
        Command::Refuse {
            client,
            message_id,
            reason,
        } => commands::refuse(&client.client()?, &message_id, &reason)?,
        Command::Outbox { client } => commands::outbox(&client.client()?)?,
        Command::Collect { client, message_id } => {
            commands::collect(&client.client()?, &message_id)?
        }
        Command::Thread { client, thread } => {
            commands::thread(&client.client()?, "thread", &thread)?
        }
        Command::Replies { client, thread } => {
            commands::thread(&client.client()?, "replies", &thread)?
        }
        Command::Repost { client } => commands::repost(&client.client()?)?,
    };
    Ok(Some(done))
}

/// Writes one line to standard output. A failed write (a reader that is gone) is not a panic: the
/// exit status still says what happened.
fn print(line: &str) {
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{line}");
    let _ = stdout.flush();
}

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            use clap::error::ErrorKind;
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp
                    | ErrorKind::DisplayVersion
                    | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
            ) {
                error.exit();
            }
            if std::env::args().any(|arg| arg == "--json") {
                print(&Failure::Usage(error.kind().to_string()).document());
            }
            let _ = error.print();
            std::process::exit(USAGE);
        }
    };
    let json = cli.json;
    match run(cli.command, json) {
        Ok(Some(done)) => print(if json { &done.document } else { &done.text }),
        Ok(None) => {}
        Err(failure) => {
            if json {
                print(&failure.document());
            } else if let Some(text) = failure.text() {
                print(text);
            }
            output::warn(&format!("inbox: {}", failure.message()));
            std::process::exit(failure.status());
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    #[test]
    fn the_command_line_is_well_formed() {
        Cli::command().debug_assert();
    }

    /// `generated/cli/inbox.json` is what `reference` writes from the clap definition.
    /// `INBOX_WRITE_CLI_REFERENCE=1` (`task cli-reference`) writes the file instead.
    #[test]
    fn the_cli_reference_document_is_current() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../generated/cli/inbox.json");
        let fresh = crate::reference::pretty(&crate::reference::reference(&Cli::command()));
        if std::env::var_os("INBOX_WRITE_CLI_REFERENCE").is_some() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &fresh).unwrap();
            return;
        }
        let committed = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            committed == fresh,
            "{}: generated/cli/inbox.json; run `task cli-reference` and commit the file",
            if committed.is_empty() {
                "missing"
            } else {
                "edited"
            }
        );
    }

    #[test]
    fn the_cli_reference_lists_every_command_with_its_flags() {
        use inbox::server::json::Value;
        let document = crate::reference::reference(&Cli::command());
        let Some(Value::Array(commands)) = document.member("commands") else {
            panic!("no commands");
        };
        let paths: Vec<String> = commands
            .iter()
            .map(|command| match command.member("path") {
                Some(Value::Array(tokens)) => tokens
                    .iter()
                    .map(|token| match token {
                        Value::Text(token) => token.clone(),
                        _ => panic!("a path token that is not text"),
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
                _ => panic!("a command without a path"),
            })
            .collect();
        assert_eq!(
            paths,
            [
                "serve",
                "key new",
                "keyfile add-key",
                "keyfile add-delegation",
                "open",
                "close",
                "post",
                "pending",
                "wait",
                "read",
                "answer",
                "refuse",
                "outbox",
                "collect",
                "thread",
                "replies",
                "repost"
            ]
        );
        let text = crate::reference::pretty(&document);
        for expected in [
            "\"env\": \"INBOX_URL\"",
            "\"default\": \"terminal\"",
            "\"repeatable\": true",
            "\"kind\": \"positional\"",
            "\"kind\": \"switch\"",
            "\"name\": \"BodyArgs\"",
            "\"code\": 4",
        ] {
            assert!(text.contains(expected), "the document lacks {expected}");
        }
    }
}
