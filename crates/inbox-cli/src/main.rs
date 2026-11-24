//! `inbox`: the HTTP service of the inbox on a loopback address (`inbox serve`), and the client
//! that talks to it with the caller's own key.
//!
//! Exit statuses: 0 done; 1 the service refused (the model's error name for a declared refusal,
//! the service's own `refused` text otherwise); 2 wrong usage; 3 transport failure; 4 a local
//! file that cannot be read, written or created; 5 (`inbox wake` and `inbox bridge` only) another
//! wake or bridge of the same inbox with the same outbox file runs on this host.

#![forbid(unsafe_code)]

mod bridge;
mod client;
mod clock_file;
mod collected;
mod commands;
mod failure;
mod ids;
mod keys;
mod outbox;
mod output;
#[cfg(test)]
mod reference;
mod serve;
mod wake;

use std::ffi::{OsStr, OsString};
use std::net::SocketAddr;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

use clap::{Args, CommandFactory, Parser, Subcommand};

use crate::client::Service;
use crate::commands::{AnswerInput, Client, PostInput, Reconnected, Typed};
use crate::failure::{Failure, USAGE};
use crate::output::Done;

/// The inbox: one durable, addressable inbox per agent or person.
#[derive(Debug, Parser)]
#[command(name = "inbox", version)]
struct Cli {
    /// Print one JSON document on standard output instead of text. After an answer from a new
    /// start of the service it is printed once the reconnect ended, with the member
    /// `reconnected`; text is printed before the reconnect starts.
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
    /// The certificate `inbox serve` answers with over TLS.
    #[command(subcommand)]
    Tls(TlsCommand),
    /// Open the caller's own inbox, or with --for the inbox of that address. The adapter is
    /// recorded beside the outbox file for that inbox: after a restart of the service the
    /// caller's next command for the same inbox opens it again with it, until `inbox close`.
    Open {
        #[command(flatten)]
        client: ClientArgs,
        /// The wake surface the inbox is read through.
        #[arg(long, default_value = "terminal")]
        adapter: String,
    },
    /// Close the caller's own inbox, or with --for the inbox of that address.
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
    /// `pending`, held open by the service until it has a row whose `expires` has not passed, or
    /// the wait ends; then every row, those past their `expires` included.
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
    /// Wait until the caller's own inbox has a message not yet delivered and not past its
    /// `expires`, for a wake adapter: print those rows of `pending` (with --json one document of
    /// them), then mark each delivered, and exit 0, or with the status of the first mark that
    /// failed. A message already delivered and not yet read neither ends the wait nor is printed:
    /// `pending` lists it. The wait is asked again when the service ends it without such a row
    /// (after a pause that grows to 30 s when that answer came within a second), after a restart
    /// of the service (the reconnect of every client command opens the inbox again), and, after a
    /// pause that grows to 30 s, while the service cannot be reached or answers 408 or a status
    /// from 500 on (also to the open after a restart). An inbox that is not open at the service
    /// ends it with 1 when this host has no record of an `inbox open` of it (also after `inbox
    /// close`), when the service refuses to open it again, or when it is still not open right
    /// after an open; an answer that is neither a success nor a refusal ends it with 3. A
    /// standard output that is closed or /dev/null ends it with 4 before any request; one it
    /// cannot write, with 4 and no mark. A second wake of the same inbox with the same outbox file
    /// on this host exits 5 and sends nothing. SIGINT and SIGTERM end it at once with nothing
    /// printed.
    Wake {
        #[command(flatten)]
        client: ClientArgs,
        /// The terminal adapter: after the marks wait again instead of exiting, and show each
        /// message as it arrives (--json: one document per line) until SIGINT or SIGTERM ends it;
        /// a mark that fails is named on standard error and it goes on. Without --json, on a
        /// terminal, the rows of each arrival are followed by one bell. An inbox this host opened
        /// with another adapter ends it with 1, at the start before any request, and while it
        /// waits with the next arrival, which is not shown: run `inbox open --adapter terminal`.
        #[arg(long)]
        follow: bool,
    },
    /// The chat bridge, for the inbox of a person: post each message of the caller's own inbox
    /// into the chat channel of the binding through `connectors`, one post per message, then mark
    /// it delivered, until SIGINT or SIGTERM. It opens the inbox with the adapter slack and takes
    /// the lock of `inbox wake`; an inbox this host opened with another adapter ends it with 1. A
    /// post that fails is tried again after a pause, and the message stays undelivered until it
    /// succeeds. While one of its posts is open it reads the recent posts of the channel through
    /// `connectors` at an interval, and a reaction of the allowed chat user on a post marks its
    /// message read (a note is handled and removed); a reaction of any other chat user does
    /// nothing. A read of the channel and a post never run at once. A message is posted once per
    /// message id: one that comes back after a restart of the service, or that a new bridge finds
    /// undelivered, is marked delivered (and read, when it was read) without a second post. At
    /// the end of a message nobody answered it writes one line into the thread of its post: the
    /// option silence took, or "expired, not answered"; it never answers in the person's place.
    /// The post of a decision lists its options by letter. A reply of the allowed chat user in the
    /// thread of the post of a request or a decision is sent as the answer, signed with the
    /// bridge's key (on behalf of the person of --for, under --authority): the text as the body
    /// of a request; for a decision the letter of an option, alone or followed by a space, a
    /// colon or a full stop and a body. A message not read yet is marked read first.
    /// Every other reply of the allowed chat user (no option named, to a note, over 65536 bytes,
    /// no text, refused by the service, after the answer) gets one line in the thread and sends
    /// nothing; a reply of any other chat user, and a line of the bridge's own, does nothing.
    /// A released `connectors` refuses the posts until the operator gave it a standing approval
    /// of the post operation into the channel: the bridge passes no approval proof. With a key of
    /// its own it serves the person's inbox with --for <address> and --authority <delegation id>,
    /// which go together; a request then answered 401 (the key entry or the delegation is missing
    /// from the service's key file or lapsed) ends it with 1 and one line that names both ids.
    Bridge {
        #[command(flatten)]
        client: ClientArgs,
        /// With --for: the id of the delegation of the person of --for that lets the bridge's key
        /// act for that inbox, as the service's key file holds it. Named in the line of a refused
        /// key; not checked by the client.
        #[arg(long, value_name = "DELEGATION_ID")]
        authority: Option<String>,
        /// The chat channel the messages are posted into: its id, such as C0000000001.
        #[arg(long)]
        chat_channel: Option<String>,
        /// The one chat user allowed to act on the posts: its id, such as U0000000001. Any
        /// reaction of this user on a post marks the message read; a reply of this user in the
        /// thread of a post answers it.
        #[arg(long)]
        allowed_chat_user: Option<String>,
        /// The target `connectors invoke` is called with: a connector, or a connector and its
        /// instance.
        #[arg(long, value_name = "CONNECTOR[/INSTANCE]")]
        chat_connector: Option<String>,
        /// How often the bridge reads the recent posts of the chat channel for the reactions on
        /// its open posts, and the threads of the posts of open messages for replies, in seconds;
        /// at least 1. A poll that fails is tried again after a pause instead.
        #[arg(long, value_name = "SECONDS", default_value_t = 10,
              value_parser = clap::value_parser!(u64).range(1..))]
        chat_poll_interval: u64,
        /// How long the record of the chat posts keeps the entry of a message once the message
        /// is gone from the service, in seconds after its `expires`. A message whose id comes
        /// back after that is posted again.
        #[arg(long, value_name = "SECONDS", default_value_t = 86_400)]
        chat_post_retention: u64,
        /// For tests only, in a debug build only, hidden from `--help` and the reference: a file
        /// that holds one RFC 3339 instant, which the bridge's clock reads each time it asks for
        /// the time (`src/clock_file.rs`). It still signs with the system clock. A release build
        /// has no such flag.
        #[cfg(debug_assertions)]
        #[arg(long, value_name = "PATH", hide = true)]
        clock_file: Option<PathBuf>,
    },
    /// Mark a message read: a note is handled and removed, a request or decision waits for its
    /// answer. A message past its `expires` is not marked: refused with `Expired`, exit status 1;
    /// the expiry task removes it or decides it on silence.
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
    /// arrive at the service's current start. Every client command does this by itself after an
    /// answer from a new start of the service. An envelope whose post the current start answered
    /// (`posted` or `Duplicate`) is left alone; a note its recipient read before a restart is
    /// delivered once more by the first repost after it.
    Repost {
        #[command(flatten)]
        client: ClientArgs,
    },
}

/// Where a client command finds the service, the caller's key and the outbox file.
#[derive(Debug, Args)]
struct ClientArgs {
    /// The service: http://<host>:<port>, or https://<host>:<port> with --tls-pin.
    #[arg(long, env = "INBOX_URL")]
    url: Option<String>,
    /// The pin of the service's TLS certificate, as `inbox tls new` printed it: with an https://
    /// URL the client accepts that certificate only, and sends nothing to a service with another.
    #[arg(long, env = "INBOX_TLS_PIN")]
    tls_pin: Option<String>,
    /// The file of the caller's secret key (`inbox key new --key-id … --principal …`); it names
    /// the key id and the principal, which is also the address of the caller's inbox.
    #[arg(long, env = "INBOX_SECRET_KEY_FILE")]
    secret_key_file: Option<PathBuf>,
    /// The outbox file; default $XDG_STATE_HOME/inbox/outbox.jsonl, else
    /// $HOME/.local/state/inbox/outbox.jsonl.
    #[arg(long, env = "INBOX_OUTBOX")]
    outbox: Option<PathBuf>,
    /// Act for the inbox at this address, the inbox of the principal with the same text: every
    /// request carries it in the header Inbox-On-Behalf-Of, signed with the caller's key, and the
    /// service answers the caller as the owner of that inbox while the key file holds a live
    /// delegation of the caller for that principal; without one every request is refused as one
    /// signed with a key the service does not hold. It is then the caller's inbox in the command:
    /// the default of --inbox, and of --on-behalf-of of `answer`; the inbox `open`, `close`,
    /// `wake` and `bridge` open, close and serve; and the inbox whose record this host keeps
    /// beside the outbox file (the lock of the wake, the record of `inbox open`, which the
    /// reconnect opens again, and the record of the chat posts). Sent as given; an empty value,
    /// one with white space at either end and one with a control character are wrong usage.
    /// `bridge` takes it with --authority only.
    #[arg(long = "for", value_name = "ADDRESS")]
    acting_for: Option<String>,
}

impl ClientArgs {
    fn client(&self) -> Result<Client, Failure> {
        if let Some(address) = &self.acting_for {
            acting_for_address(address)?;
        }
        Ok(Client::new(
            Service::from_url(self.url.as_deref(), self.tls_pin.as_deref())?,
            keys::caller(self.secret_key_file.as_deref())?,
            outbox::Outbox::locate(self.outbox.as_deref())?,
        )
        .acting_for(self.acting_for.clone()))
    }

    /// Runs the client command `command` (`typed`), and hands back the client for the reconnect
    /// its answer calls for ([`commands::reconnect`]), which `main` runs after it printed what it
    /// can print before it.
    fn run(&self, typed: Typed, command: impl FnOnce(&Client) -> Result<Done, Failure>) -> Ran {
        let client = match self.client() {
            Ok(client) => client,
            Err(failure) => return (Err(failure), None),
        };
        let result = command(&client);
        (result.map(Some), Some((client, typed)))
    }
}

/// Refuses a value of `--for` the service never honours, as wrong usage before any request: an
/// empty one, one with white space at either end (the service's reader trims every white space,
/// `char::is_whitespace`, so it would read another text than the signed one), and one that holds
/// a control character (`char::is_control`; a line break would also end the header line on the
/// wire). The value is named in its escaped form, on one line.
fn acting_for_address(address: &str) -> Result<(), Failure> {
    let padded = address.starts_with(char::is_whitespace) || address.ends_with(char::is_whitespace);
    if address.is_empty() || padded || address.contains(char::is_control) {
        return Err(Failure::Usage(format!(
            "--for {address:?} is no address: empty, white space at either end, or a control \
             character"
        )));
    }
    Ok(())
}

/// What a command did, and for a client command the client and the command it ran, for the
/// reconnect after it.
type Ran = (Result<Option<Done>, Failure>, Option<(Client, Typed)>);

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
    /// The file `inbox tls new` wrote (mode 0600): the service answers over TLS with its
    /// certificate, and the first line printed is an https:// URL. Plain HTTP without it.
    #[arg(long)]
    tls_file: Option<PathBuf>,
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
    /// elapsed time. Without it, the system clock. The deadline of a post, an answer, a refusal,
    /// a read mark and a delivery mark, the rows a wait holds as due, the request window and the
    /// expiry task read this clock.
    #[arg(long, value_name = "INSTANT", hide = true)]
    clock_start: Option<String>,
    /// For tests only, in a debug build only, hidden from `--help` and the reference: a file that
    /// holds one RFC 3339 instant, which the service's clock reads each time it asks for the
    /// time (`src/clock_file.rs`). A release build has no such flag.
    #[cfg(debug_assertions)]
    #[arg(long, value_name = "PATH", hide = true, conflicts_with = "clock_start")]
    clock_file: Option<PathBuf>,
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

#[derive(Debug, Subcommand)]
enum TlsCommand {
    /// Make a key and a self-signed certificate for `inbox serve --tls-file`: both go to --out
    /// (mode 0600, never overwritten); prints the pin the clients are given.
    New {
        /// The file to create.
        #[arg(long)]
        out: PathBuf,
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
    /// The principal whose word this is; the caller by default, and under --for the address of
    /// --for.
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

/// Runs `command`; a client command also runs the reconnect after it.
fn run(command: Command, json: bool) -> Ran {
    let (typed, client) = match &command {
        Command::Open { client, .. } => (Typed::Open, client),
        Command::Close { client } => (Typed::Close, client),
        Command::Repost { client } => (Typed::Repost, client),
        Command::Post(PostArgs { client, .. })
        | Command::Answer(AnswerArgs { client, .. })
        | Command::Pending { client, .. }
        | Command::Wait { client, .. }
        | Command::Read { client, .. }
        | Command::Refuse { client, .. }
        | Command::Outbox { client }
        | Command::Collect { client, .. }
        | Command::Thread { client, .. }
        | Command::Replies { client, .. } => (Typed::Other, client),
        Command::Serve(_) | Command::Key(_) | Command::Keyfile(_) | Command::Tls(_) => {
            return (local(command, json), None);
        }
        Command::Wake { .. } | Command::Bridge { .. } => {
            unreachable!("main runs inbox wake and inbox bridge itself")
        }
    };
    // The body is read before the client is made, as before the reconnect.
    let body = match &command {
        Command::Post(args) => Some(args.body.read()),
        Command::Answer(args) => Some(args.body.read()),
        _ => None,
    };
    let body = match body.transpose() {
        Ok(body) => body.unwrap_or_default(),
        Err(failure) => return (Err(failure), None),
    };
    client.run(typed, |client| client_command(&command, body, client))
}

/// A client command against `client`; `body` is the body of `post` and `answer`, read already.
fn client_command(command: &Command, body: String, client: &Client) -> Result<Done, Failure> {
    match command {
        Command::Open { adapter, .. } => commands::open(client, adapter),
        Command::Close { .. } => commands::close(client),
        Command::Post(args) => commands::post(
            client,
            &PostInput {
                to: args.to.clone(),
                kind: args.kind.clone(),
                thread: args.thread.clone(),
                dedup_key: args.dedup_key.clone(),
                on_behalf_of: args.on_behalf_of.clone(),
                authority: args.authority.clone(),
                reply_shape: args.reply_shape.clone(),
                options: args.options.clone(),
                recommended_option: args.recommended_option.clone(),
                default_on_silence: args.default_on_silence.clone(),
                request_ref: args.request_ref.clone(),
                expires: args.expires.clone(),
                body,
            },
        ),
        Command::Pending { inbox, .. } => commands::pending(client, inbox.as_deref(), None),
        Command::Wait { inbox, seconds, .. } => {
            commands::pending(client, inbox.as_deref(), Some(*seconds))
        }
        Command::Read { message_id, .. } => commands::read(client, message_id),
        Command::Answer(args) => commands::answer(
            client,
            &AnswerInput {
                in_reply_to: args.message_id.clone(),
                option: args.option.clone(),
                on_behalf_of: args.on_behalf_of.clone(),
                authority: args.authority.clone(),
                expires: args.expires.clone(),
                body,
            },
        ),
        Command::Refuse {
            message_id, reason, ..
        } => commands::refuse(client, message_id, reason),
        Command::Outbox { .. } => commands::outbox(client),
        Command::Collect { message_id, .. } => commands::collect(client, message_id),
        Command::Thread { thread, .. } => commands::thread(client, "thread", thread),
        Command::Replies { thread, .. } => commands::thread(client, "replies", thread),
        Command::Repost { .. } => commands::repost(client),
        Command::Serve(_)
        | Command::Key(_)
        | Command::Keyfile(_)
        | Command::Tls(_)
        | Command::Wake { .. }
        | Command::Bridge { .. } => {
            unreachable!("run sends only a client command here")
        }
    }
}

/// A command that is no client of the service: `serve`, `key`, `keyfile`, `tls`.
fn local(command: Command, json: bool) -> Result<Option<Done>, Failure> {
    let done = match command {
        Command::Serve(args) => {
            let settings = serve::Settings {
                listen: args.listen,
                key_file: &args.key_file,
                operator_public_key: &args.operator_public_key,
                tls_file: args.tls_file.as_deref(),
                max_connections: args.max_connections,
                request_timeout: args.request_timeout,
                max_request_bytes: args.max_request_bytes,
                max_messages: args.max_messages,
                max_wait: args.max_wait,
                sweep_interval: args.sweep_interval,
                answer_retention: args.answer_retention,
                clock_start: args.clock_start.as_deref(),
                #[cfg(debug_assertions)]
                clock_file: args.clock_file.as_deref(),
                #[cfg(not(debug_assertions))]
                clock_file: None,
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
        Command::Tls(TlsCommand::New { out }) => keys::tls_new(&out)?,
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
        Command::Open { .. }
        | Command::Close { .. }
        | Command::Post(_)
        | Command::Pending { .. }
        | Command::Wait { .. }
        | Command::Read { .. }
        | Command::Answer(_)
        | Command::Refuse { .. }
        | Command::Outbox { .. }
        | Command::Collect { .. }
        | Command::Thread { .. }
        | Command::Replies { .. }
        | Command::Repost { .. }
        | Command::Wake { .. }
        | Command::Bridge { .. } => unreachable!("run sends a client command to client_command"),
    };
    Ok(Some(done))
}

/// Whether `command` itself (not a subcommand, not a parent) defines the option `name`
/// (`--out`, no value) with a file name as its value (a `PathBuf`): such a value is any bytes,
/// and clap takes a name that is not UTF-8 there.
fn takes_file_name(command: &clap::Command, name: &[u8]) -> bool {
    command.get_arguments().any(|argument| {
        argument.get_value_parser().type_id() == std::any::TypeId::of::<PathBuf>()
            && argument
                .get_long()
                .is_some_and(|long| name.strip_prefix(b"--") == Some(long.as_bytes()))
    })
}

/// What a read of the command line along clap's rules found: the first argument that is not
/// UTF-8 and that no file name explains (`named`), and the places of the arguments that are
/// file names (`file_names`). A place is an index into the arguments; the program name is 0.
#[derive(Debug, Default)]
struct Scan {
    named: Option<usize>,
    file_names: Vec<usize>,
}

/// Reads `arguments` as clap does, to tell a file name that is not UTF-8 (a path option has
/// taken one since 0.1.0) from an argument a command reads as text:
/// - after a bare `--` nothing is an option;
/// - the value of an option that takes a file name is the next argument unless it starts with
///   `-`, or the text after `=`;
/// - such an option counts only under the subcommand that defines it, the subcommands named so
///   far on the line.
fn scan(arguments: &[OsString]) -> Scan {
    let root = Cli::command();
    let mut command = &root;
    let mut found = Scan::default();
    let mut after_double_dash = false;
    let mut value_next = false;
    for (at, argument) in arguments.iter().enumerate().skip(1) {
        let bytes = argument.as_bytes();
        if after_double_dash {
            if argument.to_str().is_none() {
                found.named = Some(at);
                return found;
            }
            continue;
        }
        if std::mem::take(&mut value_next) && !bytes.starts_with(b"-") {
            found.file_names.push(at);
            continue;
        }
        if bytes == b"--" {
            after_double_dash = true;
            continue;
        }
        if bytes.starts_with(b"--") {
            let name = bytes.split(|byte| *byte == b'=').next().unwrap_or(bytes);
            if takes_file_name(command, name) {
                if name.len() < bytes.len() {
                    found.file_names.push(at);
                } else {
                    value_next = true;
                }
                continue;
            }
        } else if !bytes.starts_with(b"-")
            && let Some(sub) = argument
                .to_str()
                .and_then(|name| command.find_subcommand(name))
        {
            command = sub;
            continue;
        }
        if argument.to_str().is_none() {
            found.named = Some(at);
            return found;
        }
    }
    found
}

/// The place of the argument to name for the line clap refused, and the argument. clap's error
/// does not say which argument it refused (`InvalidUtf8` for a value, an unknown argument or
/// subcommand for a stray one), so [`scan`] decides: the first argument that is not UTF-8 and
/// no file name explains. When it names none and clap's own kind is `InvalidUtf8`, the first
/// argument that is not UTF-8 and that the scan did not judge a file name. A line refused for
/// another reason with a file name that is not UTF-8 on it blames no argument. `None` when
/// there is none (the text came from an environment variable, or the cause is another).
fn refused_argument(
    arguments: &[OsString],
    kind: clap::error::ErrorKind,
) -> Option<(usize, &OsStr)> {
    let found = scan(arguments);
    let at = found.named.or_else(|| {
        (kind == clap::error::ErrorKind::InvalidUtf8)
            .then(|| {
                arguments.iter().enumerate().skip(1).find(|(at, argument)| {
                    argument.to_str().is_none() && !found.file_names.contains(at)
                })
            })
            .flatten()
            .map(|(at, _)| at)
    })?;
    Some((at, arguments[at].as_os_str()))
}

/// The usage failure for `argument`, the `at`th: its bytes in ASCII escapes (`\xE9` for each
/// byte that is not printable ASCII, a byte of a letter included; `\n`, `\r`, `\t`, `\"` and
/// `\\`), so no byte that is not printable ASCII reaches the terminal.
fn not_utf_8(at: usize, argument: &OsStr) -> Failure {
    use std::fmt::Write as _;
    let mut shown = String::new();
    for &byte in argument.as_bytes() {
        match byte {
            b'"' => shown.push_str("\\\""),
            b'\\' => shown.push_str("\\\\"),
            b'\n' => shown.push_str("\\n"),
            b'\r' => shown.push_str("\\r"),
            b'\t' => shown.push_str("\\t"),
            0x20..=0x7e => shown.push(char::from(byte)),
            _ => {
                let _ = write!(shown, "\\x{byte:02X}");
            }
        }
    }
    Failure::Usage(format!("argument {at} is not UTF-8: \"{shown}\""))
}

fn main() {
    let arguments: Vec<OsString> = std::env::args_os().collect();
    let cli = match Cli::try_parse_from(&arguments) {
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
            let wants_json = arguments.iter().any(|arg| arg == "--json");
            if let Some((at, argument)) = refused_argument(&arguments, error.kind()) {
                let failure = not_utf_8(at, argument);
                if wants_json {
                    output::print_document(&failure.document());
                }
                output::warn(&format!("inbox: {}", failure.message()));
                std::process::exit(failure.status());
            }
            if wants_json {
                output::print_document(&Failure::Usage(error.kind().to_string()).document());
            }
            let _ = error.print();
            std::process::exit(USAGE);
        }
    };
    let json = cli.json;
    // `inbox wake` writes its own output: the rows before the marks, and the reconnects as they
    // run.
    if let Command::Wake { client, follow } = &cli.command {
        let status = match client.client() {
            Ok(client) => wake::run(&client, json, *follow),
            Err(failure) => wake::report(&failure, json),
        };
        std::process::exit(status);
    }
    // So does `inbox bridge`, which runs the rounds of the wake.
    if let Command::Bridge {
        client,
        authority,
        chat_channel,
        allowed_chat_user,
        chat_connector,
        chat_poll_interval,
        chat_post_retention,
        #[cfg(debug_assertions)]
        clock_file,
    } = &cli.command
    {
        let given = bridge::Given {
            chat_channel: chat_channel.as_deref(),
            allowed_chat_user: allowed_chat_user.as_deref(),
            chat_connector: chat_connector.as_deref(),
            chat_poll_interval: *chat_poll_interval,
            chat_post_retention: *chat_post_retention,
            #[cfg(debug_assertions)]
            clock_file: clock_file.as_deref(),
            #[cfg(not(debug_assertions))]
            clock_file: None,
            authority: authority.as_deref(),
        };
        // A key of its own acts for the person's inbox only under a delegation: the two flags go
        // together, and either alone is wrong usage before anything is read or sent.
        let status = if client.acting_for.is_some() == authority.is_some() {
            match client.client() {
                Ok(client) => bridge::run(client, json, given),
                Err(failure) => bridge::report(&failure, json),
            }
        } else {
            bridge::report(
                &Failure::Usage(
                    "--for <address> and --authority <delegation id> go together: with a key of \
                     its own the bridge acts for the inbox of --for under the delegation of \
                     --authority, and with a key of the person it takes neither"
                        .to_owned(),
                ),
                json,
            )
        };
        std::process::exit(status);
    }
    let (result, after) = run(cli.command, json);
    // Text: the command's own lines are written before the reconnect starts. `--json`: the one
    // document waits for the reconnect, whose member `reconnected` it carries.
    if !json {
        match &result {
            Ok(Some(done)) => output::print_text(&done.text),
            Ok(None) => {}
            Err(failure) => {
                if let Some(text) = failure.text() {
                    output::print_text(text);
                }
                output::warn(&format!("inbox: {}", failure.message()));
            }
        }
    }
    let reconnected: Option<Reconnected> =
        after.and_then(|(client, typed)| commands::reconnect(&client, typed));
    if json {
        let shown = |document: &str| match &reconnected {
            Some(reconnected) => {
                output::with_member(document, "reconnected", &reconnected.document).into_owned()
            }
            None => document.to_owned(),
        };
        match &result {
            Ok(Some(done)) => output::print_document(&shown(&done.document)),
            Ok(None) => {}
            Err(failure) => {
                output::print_document(&shown(&failure.document()));
                output::warn(&format!("inbox: {}", failure.message()));
            }
        }
    }
    if let Some(reconnected) = &reconnected {
        output::warn(&reconnected.line);
    }
    if let Err(failure) = &result {
        std::process::exit(failure.status());
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
                "tls new",
                "open",
                "close",
                "post",
                "pending",
                "wait",
                "wake",
                "bridge",
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
            "\"code\": 5",
        ] {
            assert!(text.contains(expected), "the document lacks {expected}");
        }
    }
}
