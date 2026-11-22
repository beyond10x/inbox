//! The settled record a `collected` answer carries, held to the declared response of `Collect`.
//!
//! ESS writes an encoder of the response (`encode_response_inbox_messaging_collect`) and no
//! decoder, so [`record`] decodes it member by member with the generated decoders of each
//! member's type, closed to the declared members, in the order of the declaration.

use inbox::messaging::{CollectResponse, MessageState};
use inbox::primitives::Timestamp;
use inbox::server::json::{self, DecodeError, Value};
use inbox::server::wire;

/// The record of `message_id` in `answer`, or why the answer is not one: the outcome is
/// `collected`, no member of the answer is stated twice, `response` decodes as the declared
/// response of `Collect`, names `message_id` and is settled (`Answered` or `Refused`).
pub fn record<'a>(answer: &'a Value, message_id: &str) -> Result<&'a Value, String> {
    if !matches!(answer.member("outcome"), Some(Value::Text(outcome)) if outcome == "collected") {
        return Err("its outcome is not `collected`".to_owned());
    }
    if let Some(path) = inbox_verify::duplicate_member(answer) {
        return Err(format!("it states `{path}` twice"));
    }
    let Some(record) = answer.member("response") else {
        return Err("it carries no `response`".to_owned());
    };
    let decoded = decode(record, "response").map_err(|error| error.to_string())?;
    if decoded.message_id.0 != message_id {
        return Err(format!("its record is of {}", decoded.message_id.0));
    }
    if !matches!(
        decoded.state,
        MessageState::Answered | MessageState::Refused
    ) {
        return Err(format!("its record is in state {:?}", decoded.state));
    }
    Ok(record)
}

fn required<'a>(
    value: &'a Value,
    at: &str,
    name: &str,
) -> Result<(&'a Value, String), DecodeError> {
    Ok((json::member_at(value, at, name)?, json::nested(at, name)))
}

fn optional<'a>(value: &'a Value, at: &str, name: &str) -> Option<(&'a Value, String)> {
    match value.member(name) {
        None | Some(Value::Null) => None,
        Some(member) => Some((member, json::nested(at, name))),
    }
}

fn timestamp(value: &Value, at: &str, name: &str) -> Result<Timestamp, DecodeError> {
    let (member, at) = required(value, at, name)?;
    Ok(Timestamp(
        json::timestamp_at(member, &at, "an RFC 3339 instant")?.to_owned(),
    ))
}

/// The declared response of `Collect`, read from `value`.
fn decode(value: &Value, at: &str) -> Result<CollectResponse, DecodeError> {
    json::closed_at(
        value,
        at,
        &[
            "message_id",
            "from",
            "on_behalf_of",
            "authority",
            "to",
            "kind",
            "thread",
            "reply_shape",
            "decision",
            "default_on_silence",
            "created",
            "expires",
            "dedup_key",
            "body",
            "signature",
            "answered_by",
            "decided_option",
            "reply",
            "refusal",
            "state",
        ],
    )?;
    let field = |name: &str| required(value, at, name);
    let maybe = |name: &str| optional(value, at, name);
    Ok(CollectResponse {
        message_id: {
            let (member, at) = field("message_id")?;
            wire::decode_inbox_messaging_message_id(member, &at)?
        },
        from: {
            let (member, at) = field("from")?;
            wire::decode_inbox_messaging_principal(member, &at)?
        },
        on_behalf_of: {
            let (member, at) = field("on_behalf_of")?;
            wire::decode_inbox_messaging_principal(member, &at)?
        },
        authority: maybe("authority")
            .map(|(member, at)| wire::decode_inbox_messaging_delegation_id(member, &at))
            .transpose()?,
        to: {
            let (member, at) = field("to")?;
            wire::decode_inbox_messaging_address(member, &at)?
        },
        kind: {
            let (member, at) = field("kind")?;
            wire::decode_inbox_messaging_kind(member, &at)?
        },
        thread: {
            let (member, at) = field("thread")?;
            wire::decode_inbox_messaging_thread_id(member, &at)?
        },
        reply_shape: maybe("reply_shape")
            .map(|(member, at)| wire::decode_inbox_messaging_reply_shape(member, &at))
            .transpose()?,
        decision: maybe("decision")
            .map(|(member, at)| wire::decode_inbox_messaging_decision_ask(member, &at))
            .transpose()?,
        default_on_silence: maybe("default_on_silence")
            .map(|(member, at)| wire::decode_inbox_messaging_option_letter(member, &at))
            .transpose()?,
        created: timestamp(value, at, "created")?,
        expires: timestamp(value, at, "expires")?,
        dedup_key: {
            let (member, at) = field("dedup_key")?;
            wire::decode_inbox_messaging_dedup_key(member, &at)?
        },
        body: {
            let (member, at) = field("body")?;
            json::text_at(member, &at, "a string")?.to_owned()
        },
        signature: {
            let (member, at) = field("signature")?;
            wire::decode_inbox_messaging_signature(member, &at)?
        },
        answered_by: maybe("answered_by")
            .map(|(member, at)| wire::decode_inbox_messaging_answered_by(member, &at))
            .transpose()?,
        decided_option: maybe("decided_option")
            .map(|(member, at)| wire::decode_inbox_messaging_option_letter(member, &at))
            .transpose()?,
        reply: maybe("reply")
            .map(|(member, at)| wire::decode_inbox_messaging_collected_reply(member, &at))
            .transpose()?,
        refusal: maybe("refusal")
            .map(|(member, at)| json::text_at(member, &at, "a string").map(str::to_owned))
            .transpose()?,
        state: {
            let (member, at) = field("state")?;
            wire::decode_inbox_messaging_message_state(member, &at)?
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The record `inbox-core` answers, encoded by the generated encoder, decodes to itself.
    #[test]
    fn the_encoded_response_decodes_to_the_same_record() {
        let text = concat!(
            "{\"outcome\":\"collected\",\"response\":{",
            "\"message_id\":\"01JGT0AE000000000000000001\",\"from\":\"agent:a\",",
            "\"on_behalf_of\":\"agent:a\",\"to\":\"agent:b\",\"kind\":\"decision\",",
            "\"thread\":\"01JGT0AE000000000000000001\",",
            "\"reply_shape\":{\"shape\":\"lines\",\"value\":2},",
            "\"decision\":{\"options\":[{\"letter\":\"A\",\"text\":\"Go\"}]},",
            "\"default_on_silence\":\"A\",\"created\":\"2026-01-05T09:00:00Z\",",
            "\"expires\":\"2026-01-06T09:00:00Z\",\"dedup_key\":\"k\",\"body\":\"Which?\",",
            "\"signature\":{\"key_id\":\"a-1\",\"value\":\"c2ln\"},",
            "\"answered_by\":\"silence\",\"decided_option\":\"A\",\"state\":\"Answered\"}}"
        );
        let answer = json::parse(text).expect("JSON");
        let record = record(&answer, "01JGT0AE000000000000000001").expect("a record");
        let decoded = decode(record, "response").expect("decodes");
        let mut again = String::new();
        wire::encode_response_inbox_messaging_collect(&decoded, &mut again);
        assert_eq!(json::parse(&again).expect("JSON"), *record);
    }
}
