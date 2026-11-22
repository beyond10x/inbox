// generated from inbox v1
// contract digest f807145c2ee167ef46048e8e80ca37f3559adea0def6d63fa0d8b3b81c9a7a9d
// do not edit: regenerate with `ess synthesize --layout crate`

//! The `inbox-service` component of `inbox` v1, on the wire.
//!
//! The specification says this component's callers are not deployed with it, so its surface
//! exists on a wire. Which wire is derived rather than chosen: the one contract this model
//! projects for a command surface is the `OpenAPI` document, and an `OpenAPI` document is an
//! HTTP contract. The document is beside this file, served verbatim at `/openapi.json`.

use crate::server::{entry, http, json, wire};

/// The contract this surface answers, byte for byte as `generated/` commits it.
///
/// Embedded rather than rebuilt at run time: a server that regenerated its own contract could
/// publish one the repository never reviewed.
pub const OPENAPI: &str = include_str!("inbox-service.openapi.json");

/// The prose the same model produced, byte for byte as the documentation projection wrote it.
pub const DOCS: &str = include_str!("inbox-service.docs.md");

/// Every route this surface answers, in path order.
///
/// The same set the `OpenAPI` document declares, plus the two documents about the surface
/// itself, which no specification construct names and nothing can therefore derive. A path
/// absent from this table is answered with `404`, including one the document declares and this
/// table forgot — which is the failure a table computed twice would hide.
pub const ROUTES: &[(&str, &str)] = &[
    ("GET", "/docs"),
    ("POST", "/messaging/commands/answer"),
    ("POST", "/messaging/commands/close-inbox"),
    ("POST", "/messaging/commands/collect"),
    ("POST", "/messaging/commands/expire"),
    ("POST", "/messaging/commands/mark-delivered"),
    ("POST", "/messaging/commands/open-inbox"),
    ("POST", "/messaging/commands/post"),
    ("POST", "/messaging/commands/read"),
    ("POST", "/messaging/commands/refuse"),
    ("GET", "/messaging/views/due"),
    ("GET", "/messaging/views/inboxes"),
    ("GET", "/messaging/views/messages"),
    ("GET", "/messaging/views/outbox"),
    ("GET", "/messaging/views/pending"),
    ("GET", "/messaging/views/replies"),
    ("GET", "/messaging/views/settled"),
    ("GET", "/messaging/views/thread"),
    ("GET", "/messaging/views/unanswered"),
    ("GET", "/openapi.json"),
];

/// What this process says about itself as it starts, before it answers anything.
///
/// Three lines of JSON on standard output, in this order, every member of them derived from the
/// specification — except `runtime`, which is appended by the emitted code below and holds what
/// is true of *this process*: the language it was synthesised into, and the address it bound.
/// Everything outside `runtime` is the same in every language this plan is emitted into, and
/// `cargo xtask synth --check` starts both and compares them.
pub const STARTUP: &[&str] = &[
    "{\"log\":\"ess/1\",\"event\":\"system.starting\",\"system\":\"inbox\",\"version\":\"v1\",\"model_digest\":\"cc0910790779acb333573cd431a18b30d04fff8553f0fd953aa279794599091d\",\"contract_digest\":\"f807145c2ee167ef46048e8e80ca37f3559adea0def6d63fa0d8b3b81c9a7a9d\",\"components\":[\"inbox-service\"],\"capabilities\":{\"generated\":87,\"obligations\":3,\"refused\":0}",
    "{\"log\":\"ess/1\",\"event\":\"surface.serving\",\"component\":\"inbox-service\",\"reached_by\":\"network\",\"transport\":\"http/1.1\",\"routes\":20,\"paths\":[{\"method\":\"GET\",\"path\":\"/docs\",\"serves\":\"documentation\",\"name\":\"docs\"},{\"method\":\"POST\",\"path\":\"/messaging/commands/answer\",\"serves\":\"command\",\"name\":\"inbox.messaging.Answer\"},{\"method\":\"POST\",\"path\":\"/messaging/commands/close-inbox\",\"serves\":\"command\",\"name\":\"inbox.messaging.CloseInbox\"},{\"method\":\"POST\",\"path\":\"/messaging/commands/collect\",\"serves\":\"command\",\"name\":\"inbox.messaging.Collect\"},{\"method\":\"POST\",\"path\":\"/messaging/commands/expire\",\"serves\":\"command\",\"name\":\"inbox.messaging.Expire\"},{\"method\":\"POST\",\"path\":\"/messaging/commands/mark-delivered\",\"serves\":\"command\",\"name\":\"inbox.messaging.MarkDelivered\"},{\"method\":\"POST\",\"path\":\"/messaging/commands/open-inbox\",\"serves\":\"command\",\"name\":\"inbox.messaging.OpenInbox\"},{\"method\":\"POST\",\"path\":\"/messaging/commands/post\",\"serves\":\"command\",\"name\":\"inbox.messaging.PostMessage\"},{\"method\":\"POST\",\"path\":\"/messaging/commands/read\",\"serves\":\"command\",\"name\":\"inbox.messaging.MarkRead\"},{\"method\":\"POST\",\"path\":\"/messaging/commands/refuse\",\"serves\":\"command\",\"name\":\"inbox.messaging.Refuse\"},{\"method\":\"GET\",\"path\":\"/messaging/views/due\",\"serves\":\"view\",\"name\":\"inbox.messaging.Due\"},{\"method\":\"GET\",\"path\":\"/messaging/views/inboxes\",\"serves\":\"view\",\"name\":\"inbox.messaging.Inboxes\"},{\"method\":\"GET\",\"path\":\"/messaging/views/messages\",\"serves\":\"view\",\"name\":\"inbox.messaging.Messages\"},{\"method\":\"GET\",\"path\":\"/messaging/views/outbox\",\"serves\":\"view\",\"name\":\"inbox.messaging.Outbox\"},{\"method\":\"GET\",\"path\":\"/messaging/views/pending\",\"serves\":\"view\",\"name\":\"inbox.messaging.Pending\"},{\"method\":\"GET\",\"path\":\"/messaging/views/replies\",\"serves\":\"view\",\"name\":\"inbox.messaging.Replies\"},{\"method\":\"GET\",\"path\":\"/messaging/views/settled\",\"serves\":\"view\",\"name\":\"inbox.messaging.Settled\"},{\"method\":\"GET\",\"path\":\"/messaging/views/thread\",\"serves\":\"view\",\"name\":\"inbox.messaging.Thread\"},{\"method\":\"GET\",\"path\":\"/messaging/views/unanswered\",\"serves\":\"view\",\"name\":\"inbox.messaging.Unanswered\"},{\"method\":\"GET\",\"path\":\"/openapi.json\",\"serves\":\"contract\",\"name\":\"openapi\"}]",
    "{\"log\":\"ess/1\",\"event\":\"system.ready\",\"system\":\"inbox\",\"surfaces\":1",
];

/// Writes the startup record, with this process's own facts closing each line.
fn announce(address: &std::net::SocketAddr) {
    for facts in STARTUP {
        let mut line = String::from(*facts);
        line.push_str(",\"runtime\":{\"address\":");
        json::push_text(&mut line, &address.to_string());
        line.push_str(",\"language\":\"rust\",\"port\":");
        json::push_integer(&mut line, i64::from(address.port()));
        line.push_str("}}");
        println!("{line}");
    }
}

/// Serves `inbox-service` at `address`, and does not return while it can answer.
///
/// `address` may name port `0`, which binds an ephemeral port; the startup record says which one
/// was taken, because a caller that cannot learn the port cannot make a request.
///
/// It chooses no realization. Every command reaches the port, and a port over unimplemented
/// obligations answers the typed refusal this surface reports as `501` — the honest empty
/// state rather than a server that pretends.
///
/// `authenticate` is the realization's: it says who each request was sent by, or `None`, and
/// [`dispatch`] checks that caller's grant before the command runs. Nothing here reads an
/// actor from the request itself.
///
/// One connection at a time: each is dropped after [`http::READ_TIMEOUT`] without a byte, or
/// [`http::WRITE_TIMEOUT`] of a stalled write, and whatever fails on one connection — a caller
/// that hung up before reading its answer, a reset, a failed accept — ends that connection only.
///
/// # Errors
///
/// What binding the address refuses: the address is taken, or the port is privileged.
pub fn serve<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, address: &str, authenticate: impl Fn(&http::Request) -> Option<crate::actor::Caller>) -> std::io::Result<()>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{ serve_with_static(system, address, authenticate, None) }

/// Serves the declared surface, with files only for paths outside its route table.
///
/// # Errors
/// Returns a listener or static-root error before announcing readiness.
pub fn serve_with_static<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, address: &str, authenticate: impl Fn(&http::Request) -> Option<crate::actor::Caller>, static_root: Option<&std::path::Path>) -> std::io::Result<()>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let static_root = static_root.map(std::fs::canonicalize).transpose()?;
    if static_root.as_ref().is_some_and(|root| !root.is_dir()) { return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "static root is not a directory")); }
    let listener = std::net::TcpListener::bind(address)?;
    announce(&listener.local_addr()?);
    for connection in listener.incoming() {
        // What fails on one connection ends that connection, never this loop: a caller that
        // gave up before it was accepted, or before it read its answer, is that caller's affair.
        let Ok(connection) = connection else {
            // An accept the listener itself failed (no descriptor left) would fail again at
            // once: pause before the next.
            std::thread::sleep(std::time::Duration::from_millis(10));
            continue;
        };
        let _ = connection.set_read_timeout(Some(http::READ_TIMEOUT));
        let _ = connection.set_write_timeout(Some(http::WRITE_TIMEOUT));
        let mut reader = std::io::BufReader::new(connection);
        let (answer, refused) = match http::read(&mut reader) {
            Ok(request) => {
                if !ROUTES.iter().any(|(_, path)| *path == request.path) {
                    if let Some(root) = &static_root {
                        let _ = crate::server::static_assets::answer(reader.get_mut(), root, &request);
                        continue;
                    }
                }
                (dispatch(system, authenticate(&request).as_ref(), &request), false)
            },
            Err(refusal) => (refusal, true),
        };
        let mut stream = reader.into_inner();
        if http::write(&mut stream, &answer).is_ok() && refused {
            http::linger(&mut stream);
        }
    }
    Ok(())
}

/// Answers one request.
///
/// A path this table does not hold is a `404` naming where the whole table is published; a
/// path it holds under a different method is a `405` naming the one it answers. Neither is a
/// status the contract declares, and neither should be: both are facts about a transport rather
/// than about any command.
///
/// Public so a caller can hand it a request it built itself: [`serve`] is this function behind a
/// socket, and nothing else.
///
/// `caller` is who the realization authenticated the request as, or `None`. Every command
/// checks its grant before it runs, and answers the standard refusal when the caller is none
/// or is an actor the specification does not grant the command.
/// A view some actor's grant names checks the reader's grant the same way before it is read.
pub fn dispatch<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, caller: Option<&crate::actor::Caller>, request: &http::Request) -> http::Response
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    dispatch_with_events(system, caller, request).0
}

/// Answers one request as [`dispatch`] does, and hands back every event published while it
/// ran, in log order: what the command published, what the bindings that reacted to it
/// published in turn, and what a held-back delivery retried during the call published. An
/// event already on the system's log when the call started is not the call's and is never
/// in the list. The list is empty where no command ran — a refusal before the port, a view,
/// a document or a path no route matches — and holds what was published where delivering
/// it failed after the command took effect. An event a component's port published through a
/// direct call is in that component's outbox until a pump collects it, so one no pump has
/// collected yet is listed as this call's: call `pump` after calling a port directly.
pub fn dispatch_with_events<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, caller: Option<&crate::actor::Caller>, request: &http::Request) -> (http::Response, Vec<crate::system::SystemEvent>)
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let mut taken = Vec::new();
    let answer = route_request(system, caller, request, &mut taken);
    (answer, taken)
}

/// The route match behind [`dispatch_with_events`]: each command pushes what it took off the
/// log onto `taken`.
fn route_request<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, caller: Option<&crate::actor::Caller>, request: &http::Request, taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    match request.path.as_str() {
        "/docs" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            http::Response::new(200, http::MARKDOWN, DOCS)
        }
        "/messaging/commands/answer" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "inbox.messaging.Answer") {
                return not_granted(actor);
            }
            serve_inbox_messaging_answer(system, &request.body, taken)
        }
        "/messaging/commands/close-inbox" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "inbox.messaging.CloseInbox") {
                return not_granted(actor);
            }
            serve_inbox_messaging_close_inbox(system, &request.body, taken)
        }
        "/messaging/commands/collect" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "inbox.messaging.Collect") {
                return not_granted(actor);
            }
            serve_inbox_messaging_collect(system, &request.body, taken)
        }
        "/messaging/commands/expire" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "inbox.messaging.Expire") {
                return not_granted(actor);
            }
            serve_inbox_messaging_expire(system, &request.body, taken)
        }
        "/messaging/commands/mark-delivered" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "inbox.messaging.MarkDelivered") {
                return not_granted(actor);
            }
            serve_inbox_messaging_mark_delivered(system, &request.body, taken)
        }
        "/messaging/commands/open-inbox" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "inbox.messaging.OpenInbox") {
                return not_granted(actor);
            }
            serve_inbox_messaging_open_inbox(system, &request.body, taken)
        }
        "/messaging/commands/post" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "inbox.messaging.PostMessage") {
                return not_granted(actor);
            }
            serve_inbox_messaging_post_message(system, &request.body, taken)
        }
        "/messaging/commands/read" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "inbox.messaging.MarkRead") {
                return not_granted(actor);
            }
            serve_inbox_messaging_mark_read(system, &request.body, taken)
        }
        "/messaging/commands/refuse" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "inbox.messaging.Refuse") {
                return not_granted(actor);
            }
            serve_inbox_messaging_refuse(system, &request.body, taken)
        }
        "/messaging/views/due" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            if let Err(actor) = admit_read(caller, "inbox.messaging.Due") {
                return not_granted(actor);
            }
            serve_inbox_messaging_due(system, &request.query)
        }
        "/messaging/views/inboxes" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            if let Err(actor) = admit_read(caller, "inbox.messaging.Inboxes") {
                return not_granted(actor);
            }
            http::answer(run_inbox_messaging_inboxes(system))
        }
        "/messaging/views/messages" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            if let Err(actor) = admit_read(caller, "inbox.messaging.Messages") {
                return not_granted(actor);
            }
            http::answer(run_inbox_messaging_messages(system))
        }
        "/messaging/views/outbox" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            if let Err(actor) = admit_read(caller, "inbox.messaging.Outbox") {
                return not_granted(actor);
            }
            serve_inbox_messaging_outbox(system, &request.query)
        }
        "/messaging/views/pending" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            if let Err(actor) = admit_read(caller, "inbox.messaging.Pending") {
                return not_granted(actor);
            }
            serve_inbox_messaging_pending(system, &request.query)
        }
        "/messaging/views/replies" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            if let Err(actor) = admit_read(caller, "inbox.messaging.Replies") {
                return not_granted(actor);
            }
            serve_inbox_messaging_replies(system, &request.query)
        }
        "/messaging/views/settled" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            if let Err(actor) = admit_read(caller, "inbox.messaging.Settled") {
                return not_granted(actor);
            }
            serve_inbox_messaging_settled(system, &request.query)
        }
        "/messaging/views/thread" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            if let Err(actor) = admit_read(caller, "inbox.messaging.Thread") {
                return not_granted(actor);
            }
            serve_inbox_messaging_thread(system, &request.query)
        }
        "/messaging/views/unanswered" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            if let Err(actor) = admit_read(caller, "inbox.messaging.Unanswered") {
                return not_granted(actor);
            }
            serve_inbox_messaging_unanswered(system, &request.query)
        }
        "/openapi.json" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            http::Response::new(200, http::JSON, OPENAPI)
        }
        other => http::Response::refusal(
            404,
            &format!("`{other}` is not a path this surface declares; `GET /openapi.json` publishes every one that is"),
        ),
    }
}

/// Runs one command or view of `inbox-service` by its qualified name, with no transport.
///
/// The same decoding, refusals and rendering the HTTP routes use — each route and this function call
/// one `run_*` function — so a conformance runner or an in-process caller drives the system
/// without a socket and without a dispatch table of its own. `Ok` is the declared outcome, as the
/// route's body renders it; a view without parameters ignores `input`, as its `GET` route ignores a body, and a view with
/// parameters reads them from `input`, an object keyed by their wire names, as its route reads
/// them from the query string.
///
/// # Errors
///
/// [`entry::Refused::Unknown`] naming `name` when this surface declares no command or view
/// by it; [`entry::Refused::Input`] when `input` is not the command's declared input (the
/// route's `400`); [`entry::Refused::Unmet`] when the port reports an unmet obligation, and
/// [`entry::Refused::Undelivered`] when the command took effect and delivering what it published
/// failed (the route's `501`, with `committed` `false` and `true`).
/// [`entry::Refused::NotGranted`] when `caller` — who the realization authenticated the call
/// as — is none, or is an actor the specification does not grant the command; checked before
/// the command runs (the route's `403`).
/// A view some actor's grant names answers the same refusal, checked before it is read.
///
/// `input` is held to what the route's reader admits: nested no deeper, every number spelled
/// as JSON spells one, or [`entry::Refused::Input`]. The answer is read back at any depth; one holding a
/// `Json` value the realization built with a number JSON does not spell is not JSON, and is
/// [`entry::Refused::Undelivered`] for a command, whose effect stands, and [`entry::Refused::Unmet`]
/// for a view.
pub fn handle<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, caller: Option<&crate::actor::Caller>, name: &str, input: json::Value) -> Result<json::Value, entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    handle_with_events(system, caller, name, input).0
}

/// Runs one command or view as [`handle`] does, and hands back every event published while it
/// ran, in log order: what the command published, what the bindings that reacted to it
/// published in turn, and what a held-back delivery retried during the call published. An
/// event already on the system's log when the call started is not the call's and is never
/// in the list. The list is empty where no command ran — an unknown name, a refusal before
/// the port, a view — and holds what was published where delivering it failed after the
/// command took effect. An event a component's port published through a direct call is in
/// that component's outbox until a pump collects it, so one no pump has collected yet is
/// listed as this call's: call `pump` after calling a port directly.
pub fn handle_with_events<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, caller: Option<&crate::actor::Caller>, name: &str, input: json::Value) -> (Result<json::Value, entry::Refused>, Vec<crate::system::SystemEvent>)
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let mut taken = Vec::new();
    let checked = |input: &json::Value| {
        json::admitted(input).map_err(|expected| {
            entry::Refused::Input(format!("the input is not JSON: expected {expected}"))
        })
    };
    let answered = match name {
        "inbox.messaging.Answer" => match admit(caller, "inbox.messaging.Answer") {
            Ok(()) => checked(&input).and_then(|()| run_inbox_messaging_answer(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "inbox.messaging.CloseInbox" => match admit(caller, "inbox.messaging.CloseInbox") {
            Ok(()) => checked(&input).and_then(|()| run_inbox_messaging_close_inbox(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "inbox.messaging.Collect" => match admit(caller, "inbox.messaging.Collect") {
            Ok(()) => checked(&input).and_then(|()| run_inbox_messaging_collect(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "inbox.messaging.Due" => match admit_read(caller, "inbox.messaging.Due") {
            Ok(()) => checked(&input).and_then(|()| run_inbox_messaging_due(system, &input)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "inbox.messaging.Expire" => match admit(caller, "inbox.messaging.Expire") {
            Ok(()) => checked(&input).and_then(|()| run_inbox_messaging_expire(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "inbox.messaging.Inboxes" => match admit_read(caller, "inbox.messaging.Inboxes") {
            Ok(()) => run_inbox_messaging_inboxes(system),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "inbox.messaging.MarkDelivered" => match admit(caller, "inbox.messaging.MarkDelivered") {
            Ok(()) => checked(&input).and_then(|()| run_inbox_messaging_mark_delivered(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "inbox.messaging.MarkRead" => match admit(caller, "inbox.messaging.MarkRead") {
            Ok(()) => checked(&input).and_then(|()| run_inbox_messaging_mark_read(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "inbox.messaging.Messages" => match admit_read(caller, "inbox.messaging.Messages") {
            Ok(()) => run_inbox_messaging_messages(system),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "inbox.messaging.OpenInbox" => match admit(caller, "inbox.messaging.OpenInbox") {
            Ok(()) => checked(&input).and_then(|()| run_inbox_messaging_open_inbox(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "inbox.messaging.Outbox" => match admit_read(caller, "inbox.messaging.Outbox") {
            Ok(()) => checked(&input).and_then(|()| run_inbox_messaging_outbox(system, &input)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "inbox.messaging.Pending" => match admit_read(caller, "inbox.messaging.Pending") {
            Ok(()) => checked(&input).and_then(|()| run_inbox_messaging_pending(system, &input)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "inbox.messaging.PostMessage" => match admit(caller, "inbox.messaging.PostMessage") {
            Ok(()) => checked(&input).and_then(|()| run_inbox_messaging_post_message(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "inbox.messaging.Refuse" => match admit(caller, "inbox.messaging.Refuse") {
            Ok(()) => checked(&input).and_then(|()| run_inbox_messaging_refuse(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "inbox.messaging.Replies" => match admit_read(caller, "inbox.messaging.Replies") {
            Ok(()) => checked(&input).and_then(|()| run_inbox_messaging_replies(system, &input)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "inbox.messaging.Settled" => match admit_read(caller, "inbox.messaging.Settled") {
            Ok(()) => checked(&input).and_then(|()| run_inbox_messaging_settled(system, &input)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "inbox.messaging.Thread" => match admit_read(caller, "inbox.messaging.Thread") {
            Ok(()) => checked(&input).and_then(|()| run_inbox_messaging_thread(system, &input)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "inbox.messaging.Unanswered" => match admit_read(caller, "inbox.messaging.Unanswered") {
            Ok(()) => checked(&input).and_then(|()| run_inbox_messaging_unanswered(system, &input)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        other => return (Err(entry::Refused::Unknown(other.to_owned())), taken),
    };
    let body = match answered {
        Ok((_, body)) => body,
        Err(refused) => return (Err(refused), taken),
    };
    let read = entry::read(&body).map_err(|error| {
        if matches!(name, "inbox.messaging.Answer" | "inbox.messaging.CloseInbox" | "inbox.messaging.Collect" | "inbox.messaging.Expire" | "inbox.messaging.MarkDelivered" | "inbox.messaging.MarkRead" | "inbox.messaging.OpenInbox" | "inbox.messaging.PostMessage" | "inbox.messaging.Refuse") {
            entry::Refused::Undelivered(format!(
                "delivering what the command published: its answer is not JSON this surface reads back: {error}"
            ))
        } else {
            entry::Refused::Unmet(format!(
                "unmet obligation: view_query `{name}` answered rows this surface cannot read back: {error}"
            ))
        }
    });
    (read, taken)
}

/// Nothing, where `caller` may invoke `command`; otherwise the actor the standard refusal names,
/// `None` where the request was authenticated as no actor.
///
/// Checked before the command runs, on the caller the realization authenticated the request
/// as and never on anything the request says about itself. Public, so code that drives the system
/// in process checks the grant exactly as every route does.
pub fn admit(caller: Option<&crate::actor::Caller>, command: &str) -> Result<(), Option<&'static str>> {
    match caller {
        Some(caller) if caller.may(command) => Ok(()),
        Some(caller) => Err(Some(caller.actor.name())),
        None => Err(None),
    }
}

/// The standard refusal for an actor no grant admits, as the contract declares it: `403`,
/// `{"refused": "not granted", "actor": <name or null>}`.
fn not_granted(actor: Option<&str>) -> http::Response {
    let mut body = String::from("{");
    json::member(&mut body, "refused");
    json::push_text(&mut body, "not granted");
    json::member(&mut body, "actor");
    match actor {
        Some(actor) => json::push_text(&mut body, actor),
        None => body.push_str("null"),
    }
    body.push('}');
    http::Response::new(403, http::JSON, body)
}

/// Nothing, where `caller` may read `view`, a view some actor's grant names; otherwise the actor
/// the standard refusal names, `None` where the request was authenticated as no actor.
///
/// Checked before the view is read, on the caller the realization authenticated the request as.
/// Public, so code that reads the system in process checks the grant exactly as every route does.
pub fn admit_read(caller: Option<&crate::actor::Caller>, view: &str) -> Result<(), Option<&'static str>> {
    match caller {
        Some(caller) if caller.may_read(view) => Ok(()),
        Some(caller) => Err(Some(caller.actor.name())),
        None => Err(None),
    }
}

/// `POST` `inbox.messaging.Answer`: reads the declared input, runs the port, answers the declared outcome.
fn serve_inbox_messaging_answer<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_inbox_messaging_answer(system, &value, taken))
}

/// `inbox.messaging.Answer` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_inbox_messaging_answer<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_inbox_messaging_answer(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.inbox_service.answer(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_inbox_messaging_answer(&outcome))
}

/// One declared outcome of `inbox.messaging.Answer`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_inbox_messaging_answer(outcome: &crate::messaging::AnswerOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::messaging::AnswerOutcome::NotSender { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-sender");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.NotSender");
            403
        }
        crate::messaging::AnswerOutcome::NoAuthority { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "no-authority");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.NoAuthority");
            422
        }
        crate::messaging::AnswerOutcome::AlreadyExpired { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "already-expired");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.AlreadyExpired");
            422
        }
        crate::messaging::AnswerOutcome::SignatureInvalid { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "signature-invalid");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.SignatureInvalid");
            502
        }
        crate::messaging::AnswerOutcome::AuthorityInvalid { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "authority-invalid");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.NoAuthority");
            502
        }
        crate::messaging::AnswerOutcome::UnknownMessage { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unknown-message");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.UnknownMessage");
            404
        }
        crate::messaging::AnswerOutcome::ForeignMessage { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "foreign-message");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.UnknownMessage");
            403
        }
        crate::messaging::AnswerOutcome::NotRecipient { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-recipient");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.NotRecipient");
            403
        }
        crate::messaging::AnswerOutcome::AlreadySettled { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "already-settled");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.AlreadySettled");
            json::member(&mut body, "payload");
            wire::encode_error_inbox_messaging_already_settled(error, &mut body);
            409
        }
        crate::messaging::AnswerOutcome::Note { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "note");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.NotAnswerable");
            409
        }
        crate::messaging::AnswerOutcome::DeadlinePassed { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "deadline-passed");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.Expired");
            502
        }
        crate::messaging::AnswerOutcome::ExpiredOnArrival { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "expired-on-arrival");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.AlreadyExpired");
            502
        }
        crate::messaging::AnswerOutcome::ShapeMismatch { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "shape-mismatch");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.ShapeMismatch");
            502
        }
        crate::messaging::AnswerOutcome::Answered { message_answered, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "answered");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "inbox.messaging.MessageAnswered");
            json::member(&mut body, "payload");
            wire::encode_event_inbox_messaging_message_answered(message_answered, &mut body);
            body.push('}');
            body.push(']');
            202
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `inbox.messaging.CloseInbox`: reads the declared input, runs the port, answers the declared outcome.
fn serve_inbox_messaging_close_inbox<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_inbox_messaging_close_inbox(system, &value, taken))
}

/// `inbox.messaging.CloseInbox` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_inbox_messaging_close_inbox<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_inbox_messaging_close_inbox(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.inbox_service.close_inbox(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_inbox_messaging_close_inbox(&outcome))
}

/// One declared outcome of `inbox.messaging.CloseInbox`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_inbox_messaging_close_inbox(outcome: &crate::messaging::CloseInboxOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::messaging::CloseInboxOutcome::UnknownInbox { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unknown-inbox");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.UnknownInbox");
            404
        }
        crate::messaging::CloseInboxOutcome::NotOwner { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-owner");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.NotRecipient");
            403
        }
        crate::messaging::CloseInboxOutcome::Closed { inbox_closed, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "closed");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "inbox.messaging.InboxClosed");
            json::member(&mut body, "payload");
            wire::encode_event_inbox_messaging_inbox_closed(inbox_closed, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::messaging::CloseInboxOutcome::AlreadyClosed => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "already-closed");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            202
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `inbox.messaging.Collect`: reads the declared input, runs the port, answers the declared outcome.
fn serve_inbox_messaging_collect<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_inbox_messaging_collect(system, &value, taken))
}

/// `inbox.messaging.Collect` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_inbox_messaging_collect<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_inbox_messaging_collect(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.inbox_service.collect(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_inbox_messaging_collect(&outcome))
}

/// One declared outcome of `inbox.messaging.Collect`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_inbox_messaging_collect(outcome: &crate::messaging::CollectOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::messaging::CollectOutcome::UnknownMessage { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unknown-message");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.UnknownMessage");
            404
        }
        crate::messaging::CollectOutcome::ForeignMessage { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "foreign-message");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.UnknownMessage");
            403
        }
        crate::messaging::CollectOutcome::NotSender { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-sender");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.NotSender");
            403
        }
        crate::messaging::CollectOutcome::Collected { response, message_handled, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "collected");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "inbox.messaging.MessageHandled");
            json::member(&mut body, "payload");
            wire::encode_event_inbox_messaging_message_handled(message_handled, &mut body);
            body.push('}');
            body.push(']');
            json::member(&mut body, "response");
            wire::encode_response_inbox_messaging_collect(response, &mut body);
            200
        }
        crate::messaging::CollectOutcome::NotSettled { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-settled");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.NotSettled");
            422
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `inbox.messaging.Expire`: reads the declared input, runs the port, answers the declared outcome.
fn serve_inbox_messaging_expire<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_inbox_messaging_expire(system, &value, taken))
}

/// `inbox.messaging.Expire` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_inbox_messaging_expire<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_inbox_messaging_expire(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.inbox_service.expire(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_inbox_messaging_expire(&outcome))
}

/// One declared outcome of `inbox.messaging.Expire`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_inbox_messaging_expire(outcome: &crate::messaging::ExpireOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::messaging::ExpireOutcome::UnknownMessage { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unknown-message");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.UnknownMessage");
            404
        }
        crate::messaging::ExpireOutcome::NotDue { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-due");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.NotDue");
            409
        }
        crate::messaging::ExpireOutcome::DecidedOnSilence { decision_taken_on_silence, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "decided-on-silence");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "inbox.messaging.DecisionTakenOnSilence");
            json::member(&mut body, "payload");
            wire::encode_event_inbox_messaging_decision_taken_on_silence(decision_taken_on_silence, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::messaging::ExpireOutcome::Expired { message_expired, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "expired");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "inbox.messaging.MessageExpired");
            json::member(&mut body, "payload");
            wire::encode_event_inbox_messaging_message_expired(message_expired, &mut body);
            body.push('}');
            body.push(']');
            202
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `inbox.messaging.MarkDelivered`: reads the declared input, runs the port, answers the declared outcome.
fn serve_inbox_messaging_mark_delivered<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_inbox_messaging_mark_delivered(system, &value, taken))
}

/// `inbox.messaging.MarkDelivered` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_inbox_messaging_mark_delivered<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_inbox_messaging_mark_delivered(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.inbox_service.mark_delivered(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_inbox_messaging_mark_delivered(&outcome))
}

/// One declared outcome of `inbox.messaging.MarkDelivered`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_inbox_messaging_mark_delivered(outcome: &crate::messaging::MarkDeliveredOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::messaging::MarkDeliveredOutcome::UnknownMessage { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unknown-message");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.UnknownMessage");
            404
        }
        crate::messaging::MarkDeliveredOutcome::ForeignMessage { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "foreign-message");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.UnknownMessage");
            403
        }
        crate::messaging::MarkDeliveredOutcome::NotRecipient { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-recipient");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.NotRecipient");
            403
        }
        crate::messaging::MarkDeliveredOutcome::DeadlinePassed { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "deadline-passed");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.Expired");
            502
        }
        crate::messaging::MarkDeliveredOutcome::Delivered { message_delivered, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "delivered");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "inbox.messaging.MessageDelivered");
            json::member(&mut body, "payload");
            wire::encode_event_inbox_messaging_message_delivered(message_delivered, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::messaging::MarkDeliveredOutcome::AlreadyDelivered => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "already-delivered");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            202
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `inbox.messaging.OpenInbox`: reads the declared input, runs the port, answers the declared outcome.
fn serve_inbox_messaging_open_inbox<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_inbox_messaging_open_inbox(system, &value, taken))
}

/// `inbox.messaging.OpenInbox` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_inbox_messaging_open_inbox<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_inbox_messaging_open_inbox(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.inbox_service.open_inbox(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_inbox_messaging_open_inbox(&outcome))
}

/// One declared outcome of `inbox.messaging.OpenInbox`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_inbox_messaging_open_inbox(outcome: &crate::messaging::OpenInboxOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::messaging::OpenInboxOutcome::NotOwner { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-owner");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.NotRecipient");
            403
        }
        crate::messaging::OpenInboxOutcome::Opened { inbox_opened, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "opened");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "inbox.messaging.InboxOpened");
            json::member(&mut body, "payload");
            wire::encode_event_inbox_messaging_inbox_opened(inbox_opened, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::messaging::OpenInboxOutcome::Reopened { inbox_opened, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "reopened");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "inbox.messaging.InboxOpened");
            json::member(&mut body, "payload");
            wire::encode_event_inbox_messaging_inbox_opened(inbox_opened, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::messaging::OpenInboxOutcome::AlreadyOpen => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "already-open");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            202
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `inbox.messaging.PostMessage`: reads the declared input, runs the port, answers the declared outcome.
fn serve_inbox_messaging_post_message<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_inbox_messaging_post_message(system, &value, taken))
}

/// `inbox.messaging.PostMessage` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_inbox_messaging_post_message<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_inbox_messaging_post_message(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.inbox_service.post_message(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_inbox_messaging_post_message(&outcome))
}

/// One declared outcome of `inbox.messaging.PostMessage`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_inbox_messaging_post_message(outcome: &crate::messaging::PostMessageOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::messaging::PostMessageOutcome::NotSender { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-sender");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.NotSender");
            403
        }
        crate::messaging::PostMessageOutcome::NoAuthority { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "no-authority");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.NoAuthority");
            422
        }
        crate::messaging::PostMessageOutcome::AlreadyExpired { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "already-expired");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.AlreadyExpired");
            422
        }
        crate::messaging::PostMessageOutcome::RequestWithoutShape { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "request-without-shape");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.ShapeMismatch");
            422
        }
        crate::messaging::PostMessageOutcome::DecisionWithoutOptions { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "decision-without-options");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.ShapeMismatch");
            422
        }
        crate::messaging::PostMessageOutcome::UnknownOption { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unknown-option");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.ShapeMismatch");
            422
        }
        crate::messaging::PostMessageOutcome::ExpiredOnArrival { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "expired-on-arrival");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.AlreadyExpired");
            502
        }
        crate::messaging::PostMessageOutcome::SignatureInvalid { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "signature-invalid");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.SignatureInvalid");
            502
        }
        crate::messaging::PostMessageOutcome::AuthorityInvalid { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "authority-invalid");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.NoAuthority");
            502
        }
        crate::messaging::PostMessageOutcome::Duplicate { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "duplicate");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.Duplicate");
            409
        }
        crate::messaging::PostMessageOutcome::DuplicateKey { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "duplicate-key");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.Duplicate");
            409
        }
        crate::messaging::PostMessageOutcome::UnknownInbox { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unknown-inbox");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.UnknownInbox");
            409
        }
        crate::messaging::PostMessageOutcome::InboxClosed { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "inbox-closed");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.UnknownInbox");
            409
        }
        crate::messaging::PostMessageOutcome::IdTaken { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "id-taken");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.MessageIdTaken");
            409
        }
        crate::messaging::PostMessageOutcome::Posted { message_posted, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "posted");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "inbox.messaging.MessagePosted");
            json::member(&mut body, "payload");
            wire::encode_event_inbox_messaging_message_posted(message_posted, &mut body);
            body.push('}');
            body.push(']');
            202
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `inbox.messaging.MarkRead`: reads the declared input, runs the port, answers the declared outcome.
fn serve_inbox_messaging_mark_read<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_inbox_messaging_mark_read(system, &value, taken))
}

/// `inbox.messaging.MarkRead` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_inbox_messaging_mark_read<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_inbox_messaging_mark_read(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.inbox_service.mark_read(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_inbox_messaging_mark_read(&outcome))
}

/// One declared outcome of `inbox.messaging.MarkRead`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_inbox_messaging_mark_read(outcome: &crate::messaging::MarkReadOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::messaging::MarkReadOutcome::UnknownMessage { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unknown-message");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.UnknownMessage");
            404
        }
        crate::messaging::MarkReadOutcome::ForeignMessage { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "foreign-message");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.UnknownMessage");
            403
        }
        crate::messaging::MarkReadOutcome::NotRecipient { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-recipient");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.NotRecipient");
            403
        }
        crate::messaging::MarkReadOutcome::DeadlinePassed { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "deadline-passed");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.Expired");
            502
        }
        crate::messaging::MarkReadOutcome::Handled { message_handled, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "handled");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "inbox.messaging.MessageHandled");
            json::member(&mut body, "payload");
            wire::encode_event_inbox_messaging_message_handled(message_handled, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::messaging::MarkReadOutcome::Read { message_read, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "read");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "inbox.messaging.MessageRead");
            json::member(&mut body, "payload");
            wire::encode_event_inbox_messaging_message_read(message_read, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::messaging::MarkReadOutcome::AlreadyRead => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "already-read");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            202
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `inbox.messaging.Refuse`: reads the declared input, runs the port, answers the declared outcome.
fn serve_inbox_messaging_refuse<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_inbox_messaging_refuse(system, &value, taken))
}

/// `inbox.messaging.Refuse` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_inbox_messaging_refuse<InboxServiceBehaviors>(system: &mut crate::system::System<InboxServiceBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_inbox_messaging_refuse(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.inbox_service.refuse(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_inbox_messaging_refuse(&outcome))
}

/// One declared outcome of `inbox.messaging.Refuse`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_inbox_messaging_refuse(outcome: &crate::messaging::RefuseOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::messaging::RefuseOutcome::UnknownMessage { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unknown-message");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.UnknownMessage");
            404
        }
        crate::messaging::RefuseOutcome::ForeignMessage { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "foreign-message");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.UnknownMessage");
            403
        }
        crate::messaging::RefuseOutcome::NotRecipient { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-recipient");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.NotRecipient");
            403
        }
        crate::messaging::RefuseOutcome::Note { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "note");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.NotAnswerable");
            409
        }
        crate::messaging::RefuseOutcome::DeadlinePassed { .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "deadline-passed");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.Expired");
            502
        }
        crate::messaging::RefuseOutcome::Refused { message_refused, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "refused");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "inbox.messaging.MessageRefused");
            json::member(&mut body, "payload");
            wire::encode_event_inbox_messaging_message_refused(message_refused, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::messaging::RefuseOutcome::AlreadySettled { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "already-settled");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "inbox.messaging.AlreadySettled");
            json::member(&mut body, "payload");
            wire::encode_error_inbox_messaging_already_settled(error, &mut body);
            409
        }
    };
    body.push('}');
    (status, body)
}

/// `GET` `inbox.messaging.Due`: reads the declared parameters from the query string by their wire names, runs the
/// port, answers its rows. A key the view does not declare is ignored.
fn serve_inbox_messaging_due<InboxServiceBehaviors>(system: &crate::system::System<InboxServiceBehaviors>, query: &str) -> http::Response
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    match http::query_object(query, &[("now", http::Scalar::Text)], "query") {
        Ok(value) => http::answer(run_inbox_messaging_due(system, &value)),
        Err(refused) => http::Response::refusal(400, &refused),
    }
}

/// `GET` `inbox.messaging.Due` at `read_your_writes` consistency: every row the owed projection holds for the declared
/// parameters, read from `value`, an object keyed by their wire names.
///
/// The one path the `GET` route and [`handle`] share. A decoding failure is located under
/// `query`, as the route reports it.
fn run_inbox_messaging_due<InboxServiceBehaviors>(system: &crate::system::System<InboxServiceBehaviors>, value: &json::Value) -> Result<(u16, String), entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let (param0,) = match wire::decode_params_inbox_messaging_due(value, "query") {
        Ok(params) => params,
        Err(error) => return Err(entry::Refused::Input(format!("{error}"))),
    };
    match system.inbox_service.due(param0) {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_inbox_messaging_due(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `GET` `inbox.messaging.Inboxes` at `read_your_writes` consistency: every row the owed projection holds.
///
/// The one path the `GET` route and [`handle`] share.
fn run_inbox_messaging_inboxes<InboxServiceBehaviors>(system: &crate::system::System<InboxServiceBehaviors>) -> Result<(u16, String), entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    match system.inbox_service.inboxes() {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_inbox_messaging_inboxes(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `GET` `inbox.messaging.Messages` at `read_your_writes` consistency: every row the owed projection holds.
///
/// The one path the `GET` route and [`handle`] share.
fn run_inbox_messaging_messages<InboxServiceBehaviors>(system: &crate::system::System<InboxServiceBehaviors>) -> Result<(u16, String), entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    match system.inbox_service.messages() {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_inbox_messaging_messages(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `GET` `inbox.messaging.Outbox`: reads the declared parameters from the query string by their wire names, runs the
/// port, answers its rows. A key the view does not declare is ignored.
fn serve_inbox_messaging_outbox<InboxServiceBehaviors>(system: &crate::system::System<InboxServiceBehaviors>, query: &str) -> http::Response
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    match http::query_object(query, &[("sender", http::Scalar::Text)], "query") {
        Ok(value) => http::answer(run_inbox_messaging_outbox(system, &value)),
        Err(refused) => http::Response::refusal(400, &refused),
    }
}

/// `GET` `inbox.messaging.Outbox` at `read_your_writes` consistency: every row the owed projection holds for the declared
/// parameters, read from `value`, an object keyed by their wire names.
///
/// The one path the `GET` route and [`handle`] share. A decoding failure is located under
/// `query`, as the route reports it.
fn run_inbox_messaging_outbox<InboxServiceBehaviors>(system: &crate::system::System<InboxServiceBehaviors>, value: &json::Value) -> Result<(u16, String), entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let (param0,) = match wire::decode_params_inbox_messaging_outbox(value, "query") {
        Ok(params) => params,
        Err(error) => return Err(entry::Refused::Input(format!("{error}"))),
    };
    match system.inbox_service.outbox(param0) {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_inbox_messaging_outbox(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `GET` `inbox.messaging.Pending`: reads the declared parameters from the query string by their wire names, runs the
/// port, answers its rows. A key the view does not declare is ignored.
fn serve_inbox_messaging_pending<InboxServiceBehaviors>(system: &crate::system::System<InboxServiceBehaviors>, query: &str) -> http::Response
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    match http::query_object(query, &[("inbox", http::Scalar::Text)], "query") {
        Ok(value) => http::answer(run_inbox_messaging_pending(system, &value)),
        Err(refused) => http::Response::refusal(400, &refused),
    }
}

/// `GET` `inbox.messaging.Pending` at `read_your_writes` consistency: every row the owed projection holds for the declared
/// parameters, read from `value`, an object keyed by their wire names.
///
/// The one path the `GET` route and [`handle`] share. A decoding failure is located under
/// `query`, as the route reports it.
fn run_inbox_messaging_pending<InboxServiceBehaviors>(system: &crate::system::System<InboxServiceBehaviors>, value: &json::Value) -> Result<(u16, String), entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let (param0,) = match wire::decode_params_inbox_messaging_pending(value, "query") {
        Ok(params) => params,
        Err(error) => return Err(entry::Refused::Input(format!("{error}"))),
    };
    match system.inbox_service.pending(param0) {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_inbox_messaging_pending(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `GET` `inbox.messaging.Replies`: reads the declared parameters from the query string by their wire names, runs the
/// port, answers its rows. A key the view does not declare is ignored.
fn serve_inbox_messaging_replies<InboxServiceBehaviors>(system: &crate::system::System<InboxServiceBehaviors>, query: &str) -> http::Response
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    match http::query_object(query, &[("thread", http::Scalar::Text)], "query") {
        Ok(value) => http::answer(run_inbox_messaging_replies(system, &value)),
        Err(refused) => http::Response::refusal(400, &refused),
    }
}

/// `GET` `inbox.messaging.Replies` at `read_your_writes` consistency: every row the owed projection holds for the declared
/// parameters, read from `value`, an object keyed by their wire names.
///
/// The one path the `GET` route and [`handle`] share. A decoding failure is located under
/// `query`, as the route reports it.
fn run_inbox_messaging_replies<InboxServiceBehaviors>(system: &crate::system::System<InboxServiceBehaviors>, value: &json::Value) -> Result<(u16, String), entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let (param0,) = match wire::decode_params_inbox_messaging_replies(value, "query") {
        Ok(params) => params,
        Err(error) => return Err(entry::Refused::Input(format!("{error}"))),
    };
    match system.inbox_service.replies(param0) {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_inbox_messaging_replies(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `GET` `inbox.messaging.Settled`: reads the declared parameters from the query string by their wire names, runs the
/// port, answers its rows. A key the view does not declare is ignored.
fn serve_inbox_messaging_settled<InboxServiceBehaviors>(system: &crate::system::System<InboxServiceBehaviors>, query: &str) -> http::Response
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    match http::query_object(query, &[("sender", http::Scalar::Text)], "query") {
        Ok(value) => http::answer(run_inbox_messaging_settled(system, &value)),
        Err(refused) => http::Response::refusal(400, &refused),
    }
}

/// `GET` `inbox.messaging.Settled` at `read_your_writes` consistency: every row the owed projection holds for the declared
/// parameters, read from `value`, an object keyed by their wire names.
///
/// The one path the `GET` route and [`handle`] share. A decoding failure is located under
/// `query`, as the route reports it.
fn run_inbox_messaging_settled<InboxServiceBehaviors>(system: &crate::system::System<InboxServiceBehaviors>, value: &json::Value) -> Result<(u16, String), entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let (param0,) = match wire::decode_params_inbox_messaging_settled(value, "query") {
        Ok(params) => params,
        Err(error) => return Err(entry::Refused::Input(format!("{error}"))),
    };
    match system.inbox_service.settled(param0) {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_inbox_messaging_settled(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `GET` `inbox.messaging.Thread`: reads the declared parameters from the query string by their wire names, runs the
/// port, answers its rows. A key the view does not declare is ignored.
fn serve_inbox_messaging_thread<InboxServiceBehaviors>(system: &crate::system::System<InboxServiceBehaviors>, query: &str) -> http::Response
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    match http::query_object(query, &[("thread", http::Scalar::Text)], "query") {
        Ok(value) => http::answer(run_inbox_messaging_thread(system, &value)),
        Err(refused) => http::Response::refusal(400, &refused),
    }
}

/// `GET` `inbox.messaging.Thread` at `read_your_writes` consistency: every row the owed projection holds for the declared
/// parameters, read from `value`, an object keyed by their wire names.
///
/// The one path the `GET` route and [`handle`] share. A decoding failure is located under
/// `query`, as the route reports it.
fn run_inbox_messaging_thread<InboxServiceBehaviors>(system: &crate::system::System<InboxServiceBehaviors>, value: &json::Value) -> Result<(u16, String), entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let (param0,) = match wire::decode_params_inbox_messaging_thread(value, "query") {
        Ok(params) => params,
        Err(error) => return Err(entry::Refused::Input(format!("{error}"))),
    };
    match system.inbox_service.thread(param0) {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_inbox_messaging_thread(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `GET` `inbox.messaging.Unanswered`: reads the declared parameters from the query string by their wire names, runs the
/// port, answers its rows. A key the view does not declare is ignored.
fn serve_inbox_messaging_unanswered<InboxServiceBehaviors>(system: &crate::system::System<InboxServiceBehaviors>, query: &str) -> http::Response
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    match http::query_object(query, &[("inbox", http::Scalar::Text)], "query") {
        Ok(value) => http::answer(run_inbox_messaging_unanswered(system, &value)),
        Err(refused) => http::Response::refusal(400, &refused),
    }
}

/// `GET` `inbox.messaging.Unanswered` at `read_your_writes` consistency: every row the owed projection holds for the declared
/// parameters, read from `value`, an object keyed by their wire names.
///
/// The one path the `GET` route and [`handle`] share. A decoding failure is located under
/// `query`, as the route reports it.
fn run_inbox_messaging_unanswered<InboxServiceBehaviors>(system: &crate::system::System<InboxServiceBehaviors>, value: &json::Value) -> Result<(u16, String), entry::Refused>
where
    InboxServiceBehaviors: crate::messaging::obligations::AnswerBehavior + crate::messaging::obligations::CloseInboxBehavior + crate::messaging::obligations::CollectBehavior + crate::messaging::obligations::ExpireBehavior + crate::messaging::obligations::MarkDeliveredBehavior + crate::messaging::obligations::MarkReadBehavior + crate::messaging::obligations::OpenInboxBehavior + crate::messaging::obligations::PostMessageBehavior + crate::messaging::obligations::RefuseBehavior + crate::messaging::obligations::DueQuery + crate::messaging::obligations::InboxesQuery + crate::messaging::obligations::MessagesQuery + crate::messaging::obligations::OutboxQuery + crate::messaging::obligations::PendingQuery + crate::messaging::obligations::RepliesQuery + crate::messaging::obligations::SettledQuery + crate::messaging::obligations::ThreadQuery + crate::messaging::obligations::UnansweredQuery,
{
    let (param0,) = match wire::decode_params_inbox_messaging_unanswered(value, "query") {
        Ok(params) => params,
        Err(error) => return Err(entry::Refused::Input(format!("{error}"))),
    };
    match system.inbox_service.unanswered(param0) {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_inbox_messaging_unanswered(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}
