//! What every case shares: a verifier double, envelope builders, and a driver that runs one command
//! or view through the generated `handle` over a clone of one `MemoryBackend`.

use inbox::actor::{Actor, Caller};
use inbox::messaging::{Address, DelegationId, Principal};
use inbox::server::entry::Refused;
use inbox::server::inbox_service::handle;
use inbox::server::json::{self, Value};
use inbox_core::{CallerAttributes, Envelope, MemoryBackend, ServiceContext, SignatureVerifier};

/// A verifier double: it accepts every signature except one made with a refused key id, and every
/// delegation except a refused one.
#[derive(Debug, Clone, Default)]
pub struct KeyIdVerifier {
    refused_keys: Vec<String>,
    refused_delegations: Vec<String>,
}

impl KeyIdVerifier {
    pub fn accepting() -> Self {
        Self::default()
    }

    pub fn refusing_key(mut self, key_id: &str) -> Self {
        self.refused_keys.push(key_id.to_owned());
        self
    }

    pub fn refusing_delegation(mut self, delegation: &str) -> Self {
        self.refused_delegations.push(delegation.to_owned());
        self
    }
}

impl SignatureVerifier for KeyIdVerifier {
    fn verifies(&self, envelope: Envelope<'_>) -> bool {
        !self.refused_keys.contains(&envelope.signature().key_id)
    }

    fn authorizes(&self, _envelope: Envelope<'_>, authority: &DelegationId) -> bool {
        !self.refused_delegations.contains(&authority.0)
    }
}

/// One store, one verifier, and a fresh generated system per call, as an embedding with another
/// caller per request builds it.
pub struct World {
    pub backend: MemoryBackend,
    pub verifier: KeyIdVerifier,
}

impl World {
    pub fn new(verifier: KeyIdVerifier) -> Self {
        Self {
            backend: MemoryBackend::new(),
            verifier,
        }
    }

    fn run(
        &self,
        actor: Actor,
        caller: CallerAttributes,
        name: &str,
        input: &str,
    ) -> Result<Value, Refused> {
        let context = ServiceContext::new(caller, self.verifier.clone(), self.backend.clone());
        let mut system = inbox_core::system(self.backend.clone(), context);
        let input = json::parse(input).expect("the case writes JSON");
        handle(&mut system, Some(&Caller { actor }), name, input)
    }

    /// Runs `name` as a `Client` whose `principal` and `inbox` are both `who`.
    pub fn client(&self, who: &str, name: &str, input: &str) -> Value {
        let caller = CallerAttributes::new(Principal(who.to_owned()), Address(who.to_owned()));
        self.run(Actor::Client, caller, name, input)
            .unwrap_or_else(|refused| panic!("{name} as {who} refused: {refused}"))
    }

    /// Runs `name` as the `Sweeper`, which carries no attribute.
    pub fn sweeper(&self, name: &str, input: &str) -> Value {
        self.run(Actor::Sweeper, CallerAttributes::none(), name, input)
            .unwrap_or_else(|refused| panic!("{name} as the sweeper refused: {refused}"))
    }

    /// The rows of a view a `Client` reads.
    pub fn rows(&self, view: &str, params: &str) -> Vec<Value> {
        rows_of(&self.client("agent:ws/reader", view, params))
    }

    /// The rows of a view the `Sweeper` reads.
    pub fn sweeper_rows(&self, view: &str, params: &str) -> Vec<Value> {
        rows_of(&self.sweeper(view, params))
    }
}

fn rows_of(answer: &Value) -> Vec<Value> {
    match answer.member("rows") {
        Some(Value::Array(rows)) => rows.clone(),
        other => panic!("a view answers rows, not {other:?}"),
    }
}

/// The outcome a command answered.
pub fn outcome(answer: &Value) -> &str {
    text(answer, "outcome")
}

/// A text member.
pub fn text<'a>(value: &'a Value, name: &str) -> &'a str {
    match value.member(name) {
        Some(Value::Text(text)) => text,
        other => panic!("`{name}` is no text in {value:?}: {other:?}"),
    }
}

/// The events a command answered it published, as `(name, payload)`.
pub fn published(answer: &Value) -> Vec<(String, Value)> {
    match answer.member("published") {
        Some(Value::Array(events)) => events
            .iter()
            .map(|event| {
                let payload = event.member("payload").cloned().unwrap_or(Value::Null);
                (text(event, "event").to_owned(), payload)
            })
            .collect(),
        other => panic!("no published list: {other:?}"),
    }
}

/// `text` as a JSON string.
pub fn quoted(text: &str) -> String {
    let mut out = String::new();
    json::push_text(&mut out, text);
    out
}

/// The input of `PostMessage`, with the defaults of the authored scenarios.
#[derive(Debug, Clone)]
pub struct Post {
    pub message_id: String,
    pub from: String,
    pub on_behalf_of: String,
    pub authority: Option<String>,
    pub to: String,
    pub kind: String,
    pub reply_shape: Option<String>,
    pub decision: Option<String>,
    pub default_on_silence: Option<String>,
    pub created: String,
    pub expires: String,
    pub dedup_key: String,
    pub body: String,
    pub key_id: String,
}

impl Post {
    /// A note from `agent:ws/planner` to `agent:box/worker`; its thread and dedup key are its id.
    pub fn note(message_id: &str) -> Self {
        Self {
            message_id: message_id.to_owned(),
            from: "agent:ws/planner".to_owned(),
            on_behalf_of: "agent:ws/planner".to_owned(),
            authority: None,
            to: "agent:box/worker".to_owned(),
            kind: "note".to_owned(),
            reply_shape: None,
            decision: None,
            default_on_silence: None,
            created: "2026-01-05T09:00:00Z".to_owned(),
            expires: "2026-01-06T09:00:00Z".to_owned(),
            dedup_key: message_id.to_owned(),
            body: "Build 512 is green.".to_owned(),
            key_id: "key-1".to_owned(),
        }
    }

    /// A request whose reply is at most `lines` lines.
    pub fn request(message_id: &str, lines: i64) -> Self {
        Self {
            kind: "request".to_owned(),
            reply_shape: Some(format!("{{\"shape\":\"lines\",\"value\":{lines}}}")),
            ..Self::note(message_id)
        }
    }

    /// A decision between options `A` and `B`, recommending `B`.
    pub fn decision(message_id: &str, default_on_silence: Option<&str>) -> Self {
        Self {
            kind: "decision".to_owned(),
            decision: Some(
                "{\"options\":[{\"letter\":\"A\",\"text\":\"Ship the release today.\"},\
                 {\"letter\":\"B\",\"text\":\"Hold the release until Monday.\"}],\
                 \"recommended_option\":\"B\"}"
                    .to_owned(),
            ),
            default_on_silence: default_on_silence.map(str::to_owned),
            ..Self::note(message_id)
        }
    }

    pub fn json(&self) -> String {
        let mut out = String::from("{");
        out.push_str(&format!("\"message_id\":{}", quoted(&self.message_id)));
        out.push_str(&format!(",\"from\":{}", quoted(&self.from)));
        out.push_str(&format!(",\"on_behalf_of\":{}", quoted(&self.on_behalf_of)));
        if let Some(authority) = &self.authority {
            out.push_str(&format!(",\"authority\":{}", quoted(authority)));
        }
        out.push_str(&format!(",\"to\":{}", quoted(&self.to)));
        out.push_str(&format!(",\"kind\":{}", quoted(&self.kind)));
        out.push_str(&format!(",\"thread\":{}", quoted(&self.message_id)));
        if let Some(shape) = &self.reply_shape {
            out.push_str(&format!(",\"reply_shape\":{shape}"));
        }
        if let Some(decision) = &self.decision {
            out.push_str(&format!(",\"decision\":{decision}"));
        }
        if let Some(letter) = &self.default_on_silence {
            out.push_str(&format!(",\"default_on_silence\":{}", quoted(letter)));
        }
        out.push_str(&format!(",\"created\":{}", quoted(&self.created)));
        out.push_str(&format!(",\"expires\":{}", quoted(&self.expires)));
        out.push_str(&format!(",\"dedup_key\":{}", quoted(&self.dedup_key)));
        out.push_str(&format!(",\"body\":{}", quoted(&self.body)));
        out.push_str(&format!(
            ",\"signature\":{{\"key_id\":{},\"value\":\"c2lnbmF0dXJl\"}}",
            quoted(&self.key_id)
        ));
        out.push('}');
        out
    }
}

/// The input of `Answer`: a reply from `agent:box/worker`.
#[derive(Debug, Clone)]
pub struct Reply {
    pub in_reply_to: String,
    pub reply_id: String,
    pub from: String,
    pub on_behalf_of: String,
    pub authority: Option<String>,
    pub option: Option<String>,
    pub created: String,
    pub expires: String,
    pub body: String,
    pub key_id: String,
}

impl Reply {
    pub fn to(in_reply_to: &str, reply_id: &str) -> Self {
        Self {
            in_reply_to: in_reply_to.to_owned(),
            reply_id: reply_id.to_owned(),
            from: "agent:box/worker".to_owned(),
            on_behalf_of: "agent:box/worker".to_owned(),
            authority: None,
            option: None,
            created: "2026-01-05T09:05:00Z".to_owned(),
            expires: "2026-01-06T09:05:00Z".to_owned(),
            body: "One line.".to_owned(),
            key_id: "key-2".to_owned(),
        }
    }

    pub fn json(&self) -> String {
        let mut out = String::from("{");
        out.push_str(&format!("\"in_reply_to\":{}", quoted(&self.in_reply_to)));
        out.push_str(&format!(",\"reply_id\":{}", quoted(&self.reply_id)));
        out.push_str(&format!(",\"from\":{}", quoted(&self.from)));
        out.push_str(&format!(",\"on_behalf_of\":{}", quoted(&self.on_behalf_of)));
        if let Some(authority) = &self.authority {
            out.push_str(&format!(",\"authority\":{}", quoted(authority)));
        }
        out.push_str(&format!(",\"created\":{}", quoted(&self.created)));
        out.push_str(&format!(",\"expires\":{}", quoted(&self.expires)));
        if let Some(option) = &self.option {
            out.push_str(&format!(",\"option\":{}", quoted(option)));
        }
        out.push_str(&format!(",\"body\":{}", quoted(&self.body)));
        out.push_str(&format!(
            ",\"signature\":{{\"key_id\":{},\"value\":\"c2lnbmF0dXJl\"}}",
            quoted(&self.key_id)
        ));
        out.push('}');
        out
    }
}

/// Opens `agent:box/worker` as its owner.
pub fn open_worker(world: &World) {
    let answer = world.client(
        "agent:box/worker",
        "inbox.messaging.OpenInbox",
        r#"{"address":"agent:box/worker","adapter":"claude_code"}"#,
    );
    assert_eq!(outcome(&answer), "opened");
}

/// Posts as the sender the envelope names.
pub fn post(world: &World, post: &Post) -> Value {
    world.client(&post.from, "inbox.messaging.PostMessage", &post.json())
}

/// Answers as the owner of `agent:box/worker`.
pub fn answer(world: &World, reply: &Reply) -> Value {
    world.client("agent:box/worker", "inbox.messaging.Answer", &reply.json())
}
