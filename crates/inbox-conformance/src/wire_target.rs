//! The second target: a running `inbox serve`, driven over HTTP the way a client drives it.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read as _, Write as _};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command as Process, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use inbox::actor::{Actor, may_read};
use inbox::messaging::{DelegationEntry, DelegationId, KeyEntry, KeyFile, Principal};
use inbox::primitives::Timestamp;
use inbox::server::inbox_service::{OPENAPI, ROUTES};
use inbox::server::json::{self, Value};
use inbox::server::wire;
use inbox_server::{Clock, StartedClock, SystemClock, format_instant, request_headers};
use inbox_verify::{Instant, SecretKey, sign_delegation_entry, sign_envelope, sign_key_entry};

use crate::runner::{Answer, Command, Event, Executed, Read, Reader, Target, expected_outcome};
use crate::suite::{Expectation, Ref, Step};
use crate::value::{exact_equal, render};

/// The actor every request over HTTP is (`docs/design/service.md`, the caller).
const CLIENT: &str = "inbox.messaging.Client";
/// The service's own expiry task; no request over HTTP is it.
const SWEEPER: &str = "inbox.messaging.Sweeper";
/// Until when every key and delegation of the key file the target writes is live.
const VALID_UNTIL: &str = "9999-12-31T23:59:59Z";
/// `--sweep-interval`: longer than any run, so the expiry task, which would remove every message
/// of the suite at once (their `expires` lie in 2020 and earlier), never sweeps during one. A
/// deadline beyond what the clock can add is no deadline (`docs/design/service.md`, the limits).
const SWEEP_INTERVAL: &str = "4294967296";
/// `--clock-start`: the service's clock starts here, before every instant the suite's envelopes
/// carry (the earliest is a `created` of 2019-12-31T00:00:00Z, the earliest `expires`
/// 2020-01-01T00:00:00Z), so no post is due on arrival and no message is due when it is answered
/// or refused. The target signs each request at the same clock's reading.
pub const CLOCK_START: &str = "2019-12-30T00:00:00Z";
/// The outcomes the service takes by its own clock, which the target cannot move to a deadline.
const CLOCK_OUTCOMES: [&str; 2] = ["expired-on-arrival", "deadline-passed"];
/// How long a started service may take to print its address.
const START_TIMEOUT: Duration = Duration::from_secs(10);
/// How long a request may take to connect, and to be answered.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const ANSWER_TIMEOUT: Duration = Duration::from_secs(30);

/// Why the wire cannot witness a scenario: one reason per kind of step, the text the runner
/// reports and `unsupported.tsv` lists.
pub mod reason {
    /// A step sent or read as the Sweeper.
    pub const SWEEPER: &str = "a step is the inbox.messaging.Sweeper: over HTTP every caller is an inbox.messaging.Client, and only the service's own expiry task is the Sweeper (docs/design/service.md, the caller)";
    /// A step sent or read as no actor.
    pub const NO_ACTOR: &str = "a step is sent or read as no actor: over HTTP every request carries a caller, and one without a request signature is answered 401 before any command or view runs";
    /// A caller whose `principal` and `inbox` differ.
    pub const SPLIT_CALLER: &str = "a caller's principal and inbox differ: over HTTP both are the principal of the caller's key (docs/design/service.md, the caller)";
    /// A row claim on the stored signature.
    pub const SIGNATURE: &str = "a view claim on the stored `signature`: the wire target replaces each envelope's placeholder signature with a real one";
    /// A step that expects an outcome the service's clock decides.
    pub const CLOCK: &str = "a step expects `expired-on-arrival` or `deadline-passed`, which the service takes by its own clock: the wire target starts that clock before every instant of the suite (--clock-start) and cannot move it past a deadline";
}

/// One key the target made for a principal of the scenario.
struct Member {
    principal: String,
    key_id: String,
    key: SecretKey,
}

/// One delegation the key file holds: `agent` may speak for `on_behalf_of`.
struct Delegation {
    id: String,
    agent: String,
    on_behalf_of: String,
}

/// Where the target finds the service.
enum Launch {
    /// It starts `<binary> serve` afresh for each scenario.
    Binary(PathBuf),
    /// A service someone else runs at this address; nothing is started, reset or stopped.
    At(SocketAddr),
}

/// A started `inbox serve`. Dropped, it is killed and waited for, so no process outlives it.
struct Service {
    child: Child,
    address: SocketAddr,
    /// The service's clock as the target reads it: started with the service, at [`CLOCK_START`].
    clock: StartedClock,
    _stdout: BufReader<ChildStdout>,
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A child that is killed and waited for when dropped, until it has printed its address.
struct Starting(Option<Child>);

impl Drop for Starting {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// `inbox serve` over HTTP, a fresh service per scenario.
///
/// For each scenario [`Target::prepare`] reads the steps and either says why the wire cannot
/// witness them ([`reason`]) or makes the keys they need; [`Target::reset`] stops the previous
/// service, writes the key file into the work directory and starts `<binary> serve --listen
/// 127.0.0.1:0 --key-file <work>/keys.json --operator-public-key <key> --sweep-interval <a
/// century> --clock-start 2019-12-30T00:00:00Z`, reading its URL from the first line of its standard output.
///
/// - **Keys.** The target makes an operator key once, and for each scenario one key per
///   principal the scenario names: each caller's `principal` and each envelope's `from`, in the
///   order they first appear, with the key ids `key-1`, `key-2` and so on. The key file holds
///   these entries, signed with the operator key, and one delegation per `authority` an envelope
///   names, from its `from` to its `on_behalf_of`. Every key is made at run time and held in
///   memory; the key file holds public keys only.
/// - **A command** is sent as `POST` to the path the generated contract gives the command,
///   signed with the key of the step's caller. A `PostMessage` or `Answer` envelope is decoded
///   with the generated decoder, signed with the key of its `from` (its placeholder `signature`
///   replaced), and encoded with the generated encoder. Where the step expects
///   `signature-invalid`, the envelope is signed with a key the key file does not hold; where it
///   expects `authority-invalid`, the key file does not hold the delegation it names. The answer
///   is the outcome and error the body names, and the events of its `published` list: the
///   generated answer of a command writes every event its outcome carries, the generated port
///   publishes exactly those, and no binding reacts to any (the generated `System::pump`), so the
///   list is every event the command published. The body `{"refused": "not granted", "actor"}`
///   is a refusal as not granted.
/// - **Statuses.** Each answer is held to the status the generated contract gives it (the
///   `responses` of the operation in `inbox::server::inbox_service::OPENAPI`): a declared outcome
///   under the status whose schema names that outcome, a refusal as not granted under the status
///   whose schema holds it, a view's rows under the status whose schema names the view's
///   response. An answer under any other status is a refusal the step does not expect, so the
///   step fails.
/// - **A read** is sent as `GET` with the parameters as the query, once as each principal of the
///   key file, and the rows are the union of what the reads answered. The service answers a
///   message row only to its sender and its recipient (`docs/design/service.md`, the read rule),
///   and every stored message has a `from` that holds a key here (a post is stored only once its
///   signature verifies under the key of its `from`), so the union holds every row of the view.
/// - **The clock.** The suite's envelopes carry fixed readings (2019 and 2020), and the service
///   runs on a clock that starts before all of them ([`CLOCK_START`]) and moves on with the
///   system's, a few seconds per scenario. It reads that clock for the request window (the target
///   signs each request at the same reading), for the lapse of a key entry or delegation (every
///   entry is live until 9999), for the deadline of a post, an answer and a refusal (none is due
///   at that reading) and in the expiry task, which the sweep interval keeps from running.
///
/// What the target cannot witness makes the scenario unsupported, before anything is sent: a step
/// as the `Sweeper` or as no actor, a caller whose `principal` and `inbox` differ, a row claim on
/// `signature`, an envelope whose `from` or `authority` is not a literal, one delegation id for
/// two pairs, a step that expects an outcome of the service's clock ([`reason::CLOCK`]).
pub struct WireTarget {
    launch: Launch,
    work: PathBuf,
    operator: SecretKey,
    routes: BTreeMap<String, Route>,
    members: Vec<Member>,
    delegations: Vec<Delegation>,
    service: Option<Service>,
    broken: Option<String>,
}

impl WireTarget {
    /// A target that starts `<binary> serve` for each scenario and writes its key file into
    /// `work`.
    ///
    /// # Errors
    ///
    /// The OS random source does not answer, or the generated contract does not read.
    pub fn new(binary: impl Into<PathBuf>, work: impl Into<PathBuf>) -> Result<Self, String> {
        Self::with(Launch::Binary(binary.into()), work.into())
    }

    /// A target over a service already listening at `address` (a stand-in, in a test): nothing
    /// is started, reset or stopped, and the service does not read the key file the target
    /// writes into `work`.
    ///
    /// # Errors
    ///
    /// As [`WireTarget::new`].
    pub fn at(address: SocketAddr, work: impl Into<PathBuf>) -> Result<Self, String> {
        Self::with(Launch::At(address), work.into())
    }

    fn with(launch: Launch, work: PathBuf) -> Result<Self, String> {
        Ok(Self {
            launch,
            work,
            operator: SecretKey::generate().map_err(|error| error.0)?,
            routes: routes()?,
            members: Vec::new(),
            delegations: Vec::new(),
            service: None,
            broken: None,
        })
    }

    /// The process id of the service the target runs now, if one runs.
    pub fn service_pid(&self) -> Option<u32> {
        self.service.as_ref().map(|service| service.child.id())
    }

    /// The clock reading a request is signed at: the started service's clock, or the system's
    /// for a service someone else runs.
    fn reading(&self) -> Instant {
        match &self.service {
            Some(service) => service.clock.now(),
            None => SystemClock.now(),
        }
    }

    fn address(&self) -> Result<SocketAddr, String> {
        if let Some(reason) = &self.broken {
            return Err(reason.clone());
        }
        match (&self.launch, &self.service) {
            (Launch::At(address), _) => Ok(*address),
            (Launch::Binary(_), Some(service)) => Ok(service.address),
            (Launch::Binary(_), None) => Err("no service runs".to_owned()),
        }
    }

    fn member(&self, principal: &str) -> Result<&Member, String> {
        self.members
            .iter()
            .find(|member| member.principal == principal)
            .ok_or_else(|| format!("the key file holds no key of `{principal}`"))
    }

    /// The key file: every member's key entry and every delegation, signed with the operator key.
    fn key_file(&self) -> String {
        let keys = self
            .members
            .iter()
            .map(|member| {
                let mut entry = KeyEntry {
                    key_id: member.key_id.clone(),
                    principal: Principal(member.principal.clone()),
                    public_key: member.key.public_key().to_base64(),
                    valid_until: Timestamp(VALID_UNTIL.to_owned()),
                    operator_signature: String::new(),
                };
                sign_key_entry(&mut entry, &self.operator);
                entry
            })
            .collect();
        let delegations = self
            .delegations
            .iter()
            .map(|delegation| {
                let mut entry = DelegationEntry {
                    delegation_id: DelegationId(delegation.id.clone()),
                    agent: Principal(delegation.agent.clone()),
                    on_behalf_of: Principal(delegation.on_behalf_of.clone()),
                    valid_until: Timestamp(VALID_UNTIL.to_owned()),
                    operator_signature: String::new(),
                };
                sign_delegation_entry(&mut entry, &self.operator);
                entry
            })
            .collect();
        let mut text = String::new();
        wire::encode_inbox_messaging_key_file(&KeyFile { keys, delegations }, &mut text);
        text
    }

    /// Writes the key file and starts the service over it.
    fn start(&self, binary: &Path) -> Result<Service, String> {
        fs::create_dir_all(&self.work)
            .map_err(|error| format!("{}: {error}", self.work.display()))?;
        let key_file = self.work.join("keys.json");
        fs::write(&key_file, self.key_file())
            .map_err(|error| format!("{}: {error}", key_file.display()))?;
        let errors = self.work.join("serve.err");
        let stderr =
            File::create(&errors).map_err(|error| format!("{}: {error}", errors.display()))?;
        let mut command = Process::new(binary);
        command
            .arg("serve")
            .arg("--listen")
            .arg("127.0.0.1:0")
            .arg("--key-file")
            .arg(&key_file)
            .arg("--operator-public-key")
            .arg(self.operator.public_key().to_base64())
            .arg("--sweep-interval")
            .arg(SWEEP_INTERVAL)
            .arg("--clock-start")
            .arg(CLOCK_START)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(stderr);
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("INBOX_") {
                command.env_remove(name);
            }
        }
        let clock = StartedClock::new(
            Instant::parse(CLOCK_START).map_err(|error| format!("--clock-start: {error}"))?,
        );
        let mut child = command
            .spawn()
            .map_err(|error| format!("{}: {error}", binary.display()))?;
        let stdout = child.stdout.take();
        let mut starting = Starting(Some(child));
        let stdout = stdout.ok_or("the service's standard output is not piped")?;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            let read = reader.read_line(&mut line);
            let _ = sender.send((read.map_err(|error| error.to_string()), line, reader));
        });
        let said = || fs::read_to_string(&errors).unwrap_or_default();
        let (line, reader) = match receiver.recv_timeout(START_TIMEOUT) {
            Ok((Ok(read), line, reader)) if read > 0 => (line, reader),
            Ok((read, _, _)) => {
                let status = starting.0.as_mut().and_then(|child| child.wait().ok());
                return Err(format!(
                    "it ended before it printed its address ({}, exit {}); its standard error: {}",
                    read.err().unwrap_or_else(|| "end of output".to_owned()),
                    status.map_or("unknown".to_owned(), |status| status.to_string()),
                    said().trim()
                ));
            }
            Err(_) => {
                return Err(format!(
                    "it printed no address within {} s; its standard error: {}",
                    START_TIMEOUT.as_secs(),
                    said().trim()
                ));
            }
        };
        let address = line
            .trim()
            .strip_prefix("http://")
            .and_then(|address| address.parse::<SocketAddr>().ok())
            .filter(|address| address.ip().is_loopback())
            .ok_or_else(|| format!("its first line is {line:?}, not http://<loopback address>"))?;
        let child = starting.0.take().ok_or("the child is gone")?;
        Ok(Service {
            child,
            address,
            clock,
            _stdout: reader,
        })
    }

    /// Sends one request signed by `member` and reads the whole answer: its status and its body
    /// read as JSON, or the body's text where it is not JSON.
    fn send(
        &self,
        member: &Member,
        method: &str,
        target: &str,
        body: &[u8],
    ) -> Result<(u16, Result<Value, String>), String> {
        let address = self.address()?;
        let timestamp = format_instant(self.reading());
        let headers = request_headers(
            method,
            target,
            body,
            &member.key_id,
            &member.key,
            &timestamp,
        );
        let transport = |error: std::io::Error| format!("{address}: {error}");
        let mut stream =
            TcpStream::connect_timeout(&address, CONNECT_TIMEOUT).map_err(transport)?;
        stream
            .set_read_timeout(Some(ANSWER_TIMEOUT))
            .map_err(transport)?;
        stream
            .set_write_timeout(Some(ANSWER_TIMEOUT))
            .map_err(transport)?;
        let mut head = format!(
            "{method} {target} HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
            body.len()
        );
        for (name, value) in headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        head.push_str("\r\n");
        stream.write_all(head.as_bytes()).map_err(transport)?;
        stream.write_all(body).map_err(transport)?;
        stream.flush().map_err(transport)?;
        let mut answer = Vec::new();
        stream.read_to_end(&mut answer).map_err(transport)?;
        let answer = String::from_utf8_lossy(&answer);
        let (head, text) = answer
            .split_once("\r\n\r\n")
            .ok_or_else(|| format!("{address}: the answer has no end of its head"))?;
        let status = head
            .split(' ')
            .nth(1)
            .and_then(|status| status.parse::<u16>().ok())
            .ok_or_else(|| format!("{address}: the answer has no status line"))?;
        Ok((status, json::parse(text).map_err(|_| text.to_owned())))
    }

    /// The body of a command: an envelope signed as [`WireTarget`] says, any other input as it is.
    fn body(&self, command: &Command<'_>) -> Result<String, String> {
        let signer = |from: &str| -> Result<(String, SecretKey), String> {
            let member = self.member(from)?;
            if command.expected == Some("signature-invalid") {
                let stranger = SecretKey::generate().map_err(|error| error.0)?;
                return Ok((member.key_id.clone(), stranger));
            }
            Ok((member.key_id.clone(), member.key.clone()))
        };
        let mut out = String::new();
        match command.name {
            "inbox.messaging.PostMessage" => {
                match wire::decode_command_inbox_messaging_post_message(&command.input, "body") {
                    Ok(mut post) => {
                        let (key_id, key) = signer(&post.from.0)?;
                        sign_envelope(&mut post, &key_id, &key)
                            .map_err(|error| format!("the envelope: {error}"))?;
                        wire::encode_command_inbox_messaging_post_message(&post, &mut out);
                    }
                    Err(_) => out = render(&command.input),
                }
            }
            "inbox.messaging.Answer" => {
                match wire::decode_command_inbox_messaging_answer(&command.input, "body") {
                    Ok(mut reply) => {
                        let (key_id, key) = signer(&reply.from.0)?;
                        sign_envelope(&mut reply, &key_id, &key)
                            .map_err(|error| format!("the envelope: {error}"))?;
                        wire::encode_command_inbox_messaging_answer(&reply, &mut out);
                    }
                    Err(_) => out = render(&command.input),
                }
            }
            _ => out = render(&command.input),
        }
        Ok(out)
    }

    /// One read of `target` of `route` as `member`: rows under the status the contract gives a
    /// view's rows, a refusal as not granted under the status it gives that refusal.
    fn read_as(&self, route: &Route, member: &Member, target: &str) -> Read {
        match self.send(member, route.method, target, b"") {
            Err(reason) => Read::Broken(reason),
            Ok((status, Ok(body))) => match (body.member("rows"), refusal(&body)) {
                (Some(Value::Array(rows)), _) if route.rows.contains(&status) => {
                    Read::Rows(rows.clone())
                }
                (_, Some(actor)) if route.not_granted.contains(&status) => Read::NotGranted(actor),
                _ => Read::Refused(format!("{status} {}", render(&body))),
            },
            Ok((status, Err(text))) => Read::Refused(format!("{status} {text}")),
        }
    }
}

/// One command or view of the generated contract: where it is served, and the statuses the
/// contract gives its answers.
#[derive(Debug, Clone)]
struct Route {
    method: &'static str,
    path: &'static str,
    /// Each declared outcome of a command, with the statuses the contract gives it.
    outcomes: BTreeMap<String, Vec<u16>>,
    /// The status of the refusal as not granted.
    not_granted: Vec<u16>,
    /// The status of a view's rows.
    rows: Vec<u16>,
}

impl Route {
    /// Whether the contract gives `outcome` the status `status`.
    fn admits(&self, outcome: &str, status: u16) -> bool {
        self.outcomes
            .get(outcome)
            .is_some_and(|statuses| statuses.contains(&status))
    }
}

/// Every `$ref` text inside `value`.
fn references<'a>(value: &'a Value, out: &mut Vec<&'a str>) {
    match value {
        Value::Object(members) => {
            for (name, member) in members {
                match (name.as_str(), member) {
                    ("$ref", Value::Text(reference)) => out.push(reference),
                    _ => references(member, out),
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|item| references(item, out)),
        _ => {}
    }
}

/// Whether `value` holds the schema of the refusal as not granted: a member whose `enum` is
/// exactly `["not granted"]`.
fn holds_not_granted(value: &Value) -> bool {
    match value {
        Value::Object(members) => members.iter().any(|(name, member)| {
            (name == "enum"
                && matches!(member, Value::Array(items)
                    if matches!(items.as_slice(), [Value::Text(text)] if text == "not granted")))
                || holds_not_granted(member)
        }),
        Value::Array(items) => items.iter().any(holds_not_granted),
        _ => false,
    }
}

/// The route of each command and view, by its qualified name, from the generated contract
/// (`operationId` in `inbox::server::inbox_service::OPENAPI`, the document `generated/openapi`
/// projects), each held to the generated route table, with the statuses of its `responses`: a
/// declared outcome is the status whose schema names `<name>.<outcome>.Response`, a view's rows
/// the status whose schema names `<name>.Response`, the refusal as not granted the status whose
/// schema holds `"enum": ["not granted"]`.
fn routes() -> Result<BTreeMap<String, Route>, String> {
    let contract =
        json::parse(OPENAPI).map_err(|error| format!("the generated contract: {error}"))?;
    let Some(Value::Object(paths)) = contract.member("paths") else {
        return Err("the generated contract has no `paths`".to_owned());
    };
    let mut routes = BTreeMap::new();
    for (path, operations) in paths {
        let Value::Object(operations) = operations else {
            continue;
        };
        for (method, operation) in operations {
            let Some(Value::Text(name)) = operation.member("operationId") else {
                continue;
            };
            let method = method.to_uppercase();
            let (method, path) = *ROUTES
                .iter()
                .find(|(held, held_path)| *held == method && held_path == path)
                .ok_or_else(|| {
                    format!("the generated route table holds no `{method} {path}` for `{name}`")
                })?;
            let mut route = Route {
                method,
                path,
                outcomes: BTreeMap::new(),
                not_granted: Vec::new(),
                rows: Vec::new(),
            };
            let Some(Value::Object(responses)) = operation.member("responses") else {
                return Err(format!(
                    "the generated contract gives `{name}` no `responses`"
                ));
            };
            let schemas = format!("#/components/schemas/{name}.");
            for (status, response) in responses {
                let Ok(status) = status.parse::<u16>() else {
                    continue;
                };
                let mut named = Vec::new();
                references(response, &mut named);
                for reference in named {
                    match reference.strip_prefix(&schemas) {
                        Some("Response") => route.rows.push(status),
                        Some(rest) => {
                            if let Some(outcome) = rest.strip_suffix(".Response") {
                                let statuses =
                                    route.outcomes.entry(outcome.to_owned()).or_default();
                                if !statuses.contains(&status) {
                                    statuses.push(status);
                                }
                            }
                        }
                        None => {}
                    }
                }
                if holds_not_granted(response) {
                    route.not_granted.push(status);
                }
            }
            routes.insert(name.clone(), route);
        }
    }
    Ok(routes)
}

/// The actor a `403` refusal as not granted names: `Some(None)` for no actor.
fn refusal(body: &Value) -> Option<Option<String>> {
    match (body.member("refused"), body.member("actor")) {
        (Some(Value::Text(refused)), Some(Value::Text(actor))) if refused == "not granted" => {
            Some(Some(actor.clone()))
        }
        (Some(Value::Text(refused)), Some(Value::Null)) if refused == "not granted" => Some(None),
        _ => None,
    }
}

/// The answer to a command, as the body says it, under a status the contract gives it: an
/// outcome under another status than the contract gives that outcome of `route`, or a refusal as
/// not granted under another than the contract's, is a refusal the step does not expect.
fn executed(route: &Route, status: u16, body: &Value) -> Executed {
    let refused = |why: String| Executed {
        answer: Answer::Refused(why),
        events: Vec::new(),
    };
    if let Some(Value::Text(outcome)) = body.member("outcome") {
        if !route.admits(outcome, status) {
            return refused(format!(
                "{status} {}: the contract gives the outcome `{outcome}` the status {}",
                render(body),
                route.outcomes.get(outcome).map_or_else(
                    || "of no declared outcome".to_owned(),
                    |statuses| format!("{statuses:?}")
                )
            ));
        }
        let error = match body.member("error") {
            Some(Value::Text(error)) => Some(error.clone()),
            _ => None,
        };
        let Some(Value::Array(published)) = body.member("published") else {
            return refused(format!(
                "{status} {}: an outcome without its `published` list",
                render(body)
            ));
        };
        let mut events = Vec::new();
        for item in published {
            match (item.member("event"), item.member("payload")) {
                (Some(Value::Text(name)), Some(payload)) => events.push(Event {
                    name: name.clone(),
                    payload: payload.clone(),
                }),
                _ => {
                    return refused(format!(
                        "{status} {}: a published event without its name and payload",
                        render(body)
                    ));
                }
            }
        }
        return Executed {
            answer: Answer::Outcome {
                outcome: outcome.clone(),
                error,
            },
            events,
        };
    }
    if route.not_granted.contains(&status)
        && let Some(actor) = refusal(body)
    {
        return Executed {
            answer: Answer::NotGranted(actor),
            events: Vec::new(),
        };
    }
    refused(format!("{status} {}", render(body)))
}

/// `text` percent-encoded for a query value: every byte but the unreserved ones as `%XX`.
fn query_value(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// The literal text of the input member `name`, if it is one.
fn literal<'a>(input: &'a [(String, Ref)], name: &str) -> Option<Result<&'a str, ()>> {
    input
        .iter()
        .find(|(member, _)| member == name)
        .map(|(_, value)| match value {
            Ref::Literal(Value::Text(text)) => Ok(text.as_str()),
            _ => Err(()),
        })
}

/// What [`WireTarget::prepare`] finds in a scenario.
#[derive(Default)]
struct Plan {
    reasons: Vec<String>,
    principals: Vec<String>,
    delegations: Vec<(String, String, String)>,
    withheld: Vec<(String, String, String)>,
}

impl Plan {
    fn refuse(&mut self, reason: &str) {
        if !self.reasons.iter().any(|known| known == reason) {
            self.reasons.push(reason.to_owned());
        }
    }

    fn principal(&mut self, principal: &str) {
        if !self.principals.iter().any(|known| known == principal) {
            self.principals.push(principal.to_owned());
        }
    }

    fn actor(&mut self, actor: Option<&str>) {
        match actor {
            Some(CLIENT) => {}
            Some(SWEEPER) => self.refuse(reason::SWEEPER),
            None => self.refuse(reason::NO_ACTOR),
            Some(other) => self.refuse(&format!("`{other}` is no actor the service knows")),
        }
    }

    fn read(steps: &[Step]) -> Self {
        let mut plan = Self::default();
        let mut reader: Option<Option<&str>> = None;
        for (at, step) in steps.iter().enumerate() {
            match step {
                Step::Execute {
                    command,
                    actor,
                    caller,
                    input,
                } => {
                    plan.actor(actor.as_deref());
                    if actor.as_deref() == Some(CLIENT) {
                        plan.caller(caller);
                    }
                    if expected_outcome(steps, at, command)
                        .is_some_and(|outcome| CLOCK_OUTCOMES.contains(&outcome))
                    {
                        plan.refuse(reason::CLOCK);
                    }
                    if matches!(
                        command.as_str(),
                        "inbox.messaging.PostMessage" | "inbox.messaging.Answer"
                    ) {
                        plan.envelope(input, expected_outcome(steps, at, command));
                    }
                }
                Step::ReadAs { actor } => {
                    plan.actor(actor.as_deref());
                    reader = Some(actor.as_deref());
                }
                Step::Query { view, .. } | Step::Snapshot { view, .. }
                    if reader.is_none() && !may_read(Actor::Client).contains(&view.as_str()) =>
                {
                    plan.refuse(reason::SWEEPER);
                }
                Step::ExpectView {
                    expectation: Expectation::Contains(fields) | Expectation::Excludes(fields),
                    ..
                } if fields
                    .iter()
                    .any(|(name, _)| name == "signature" || name.starts_with("signature.")) =>
                {
                    plan.refuse(reason::SIGNATURE);
                }
                _ => {}
            }
        }
        for (id, agent, on_behalf_of) in &plan.withheld.clone() {
            if plan
                .delegations
                .iter()
                .any(|held| held == &(id.clone(), agent.clone(), on_behalf_of.clone()))
            {
                plan.refuse(&format!(
                    "the delegation `{id}` is to be refused once and held once for the same pair"
                ));
            }
        }
        plan
    }

    fn caller(&mut self, caller: &[(String, String)]) {
        let attribute = |name: &str| {
            caller
                .iter()
                .find(|(held, _)| held == name)
                .map(|(_, value)| value.as_str())
        };
        let known = caller
            .iter()
            .all(|(name, _)| name == "principal" || name == "inbox");
        match (attribute("principal"), attribute("inbox")) {
            (Some(principal), Some(inbox)) if principal == inbox && known => {
                self.principal(principal);
            }
            (Some(_), Some(_)) if known => self.refuse(reason::SPLIT_CALLER),
            _ => self.refuse(
                "a caller of inbox.messaging.Client without exactly a principal and an inbox",
            ),
        }
    }

    fn envelope(&mut self, input: &[(String, Ref)], expected: Option<&str>) {
        let from = match literal(input, "from") {
            Some(Ok(from)) => from.to_owned(),
            _ => {
                self.refuse("an envelope whose `from` is not a literal: the key file is written before the service starts");
                return;
            }
        };
        self.principal(&from);
        let authority = match literal(input, "authority") {
            None => return,
            Some(Ok(authority)) => authority.to_owned(),
            Some(Err(())) => {
                self.refuse("an envelope whose `authority` is not a literal: the key file is written before the service starts");
                return;
            }
        };
        let Some(Ok(on_behalf_of)) = literal(input, "on_behalf_of") else {
            self.refuse("an envelope whose `on_behalf_of` is not a literal: the key file is written before the service starts");
            return;
        };
        let pair = (authority.clone(), from, on_behalf_of.to_owned());
        if expected == Some("authority-invalid") {
            self.withheld.push(pair);
            return;
        }
        match self.delegations.iter().find(|held| held.0 == authority) {
            None => self.delegations.push(pair),
            Some(held) if *held == pair => {}
            Some(_) => self.refuse(&format!(
                "the delegation `{authority}` is named for two pairs, and an id names one entry of the key file"
            )),
        }
    }
}

impl Target for WireTarget {
    fn prepare(&mut self, steps: &[Step]) -> Result<(), String> {
        let plan = Plan::read(steps);
        if !plan.reasons.is_empty() {
            return Err(plan.reasons.join("; "));
        }
        let mut principals = plan.principals;
        if principals.is_empty() {
            principals.push("conformance-reader".to_owned());
        }
        self.broken = None;
        self.members.clear();
        for (at, principal) in principals.into_iter().enumerate() {
            match SecretKey::generate() {
                Ok(key) => self.members.push(Member {
                    principal,
                    key_id: format!("key-{}", at + 1),
                    key,
                }),
                Err(error) => self.broken = Some(format!("a key for `{principal}`: {}", error.0)),
            }
        }
        self.delegations = plan
            .delegations
            .into_iter()
            .map(|(id, agent, on_behalf_of)| Delegation {
                id,
                agent,
                on_behalf_of,
            })
            .collect();
        Ok(())
    }

    fn reset(&mut self) {
        self.service = None;
        if self.broken.is_some() {
            return;
        }
        if let Launch::Binary(binary) = &self.launch {
            match self.start(binary) {
                Ok(service) => self.service = Some(service),
                Err(reason) => {
                    self.broken = Some(format!(
                        "the service {} did not start: {reason}",
                        binary.display()
                    ));
                }
            }
        }
    }

    fn execute(&mut self, command: &Command<'_>) -> Executed {
        let broken = |reason: String| Executed {
            answer: Answer::Broken(reason),
            events: Vec::new(),
        };
        let Some(route) = self.routes.get(command.name).cloned() else {
            return broken(format!(
                "the generated contract has no route for `{}`",
                command.name
            ));
        };
        let principal =
            match (
                command.actor,
                command.caller.iter().find(|(name, _)| name == "principal"),
            ) {
                (Some(CLIENT), Some((_, principal))) => principal,
                _ => return broken(
                    "the wire sends a command as an inbox.messaging.Client with a principal only"
                        .to_owned(),
                ),
            };
        let member = match self.member(principal) {
            Ok(member) => member,
            Err(reason) => return broken(reason),
        };
        let body = match self.body(command) {
            Ok(body) => body,
            Err(reason) => return broken(reason),
        };
        match self.send(member, route.method, route.path, body.as_bytes()) {
            Err(reason) => broken(reason),
            Ok((status, Ok(answer))) => executed(&route, status, &answer),
            Ok((status, Err(text))) => Executed {
                answer: Answer::Refused(format!("{status} {text}")),
                events: Vec::new(),
            },
        }
    }

    fn query(&mut self, view: &str, reader: &Reader, params: Value) -> Read {
        match reader {
            Reader::Unstated => {}
            Reader::As(Some(actor)) if actor == CLIENT => {}
            Reader::As(other) => {
                return Read::Broken(format!(
                    "the wire reads as an inbox.messaging.Client only, not as {other:?}"
                ));
            }
        }
        let Some(route) = self.routes.get(view).cloned() else {
            return Read::Broken(format!("the generated contract has no route for `{view}`"));
        };
        let Value::Object(params) = params else {
            return Read::Broken("the parameters are not an object".to_owned());
        };
        let mut query = Vec::new();
        for (name, value) in &params {
            let Value::Text(text) = value else {
                return Read::Broken(format!("the parameter `{name}` is not text"));
            };
            query.push(format!("{}={}", query_value(name), query_value(text)));
        }
        let target = if query.is_empty() {
            route.path.to_owned()
        } else {
            format!("{}?{}", route.path, query.join("&"))
        };
        let mut union: Option<Vec<Value>> = None;
        for member in &self.members {
            match (self.read_as(&route, member, &target), &mut union) {
                (Read::Rows(rows), Some(kept)) => {
                    for row in rows {
                        if !kept.iter().any(|held| exact_equal(held, &row)) {
                            kept.push(row);
                        }
                    }
                }
                (Read::Rows(rows), None) => union = Some(rows),
                (other, None) => return other,
                (other, Some(_)) => {
                    return Read::Refused(format!(
                        "`{view}` answered rows to one key and {other:?} to the key of `{}`",
                        member.principal
                    ));
                }
            }
        }
        match union {
            Some(rows) => Read::Rows(rows),
            None => Read::Broken("the key file holds no key to read with".to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::routes;

    #[test]
    fn the_statuses_are_read_from_the_generated_contract() {
        let routes = routes().expect("the contract reads");
        let open = &routes["inbox.messaging.OpenInbox"];
        assert_eq!(
            (open.method, open.path),
            ("POST", "/messaging/commands/open-inbox")
        );
        assert_eq!(open.outcomes["opened"], [202]);
        assert_eq!(open.outcomes["already-open"], [202]);
        assert_eq!(open.outcomes["not-owner"], [403]);
        assert_eq!(open.not_granted, [403]);
        assert!(open.rows.is_empty());
        let post = &routes["inbox.messaging.PostMessage"];
        assert_eq!(post.outcomes["duplicate"], [409]);
        assert_eq!(post.outcomes["signature-invalid"], [502]);
        assert_eq!(post.outcomes["already-expired"], [422]);
        let pending = &routes["inbox.messaging.Pending"];
        assert_eq!(pending.rows, [200]);
        assert_eq!(pending.not_granted, [403]);
        assert!(pending.outcomes.is_empty());
    }
}
