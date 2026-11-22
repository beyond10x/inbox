// generated from inbox v1
// contract digest 9447d0d3fd8d202100ab11dab84fc004c77f28cae8e3e28a0e0222ae7f780f19
// do not edit: regenerate with `ess synthesize --layout crate`
//! Every generated declaration, as JSON, in the renderings the published wire contracts fix.
//!
//! Generated from the model beside the types it crosses, so a field renamed in the specification
//! is renamed here in the same regeneration. An absent optional field is omitted rather
//! than sent as `null`, which is what the `required` list of the published schema says.

use crate::server::json;

/// Writes `inbox.messaging.Adapter` as JSON.
pub fn encode_inbox_messaging_adapter(value: &crate::messaging::Adapter, out: &mut String) {
    match value {
        crate::messaging::Adapter::ClaudeCode => json::push_text(out, "claude_code"),
        crate::messaging::Adapter::Metaharness => json::push_text(out, "metaharness"),
        crate::messaging::Adapter::Slack => json::push_text(out, "slack"),
        crate::messaging::Adapter::Terminal => json::push_text(out, "terminal"),
    }
}

/// Reads `inbox.messaging.Adapter` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_adapter(value: &json::Value, at: &str) -> Result<crate::messaging::Adapter, json::DecodeError> {
    Ok(match json::text_at(value, at, "one of `claude_code`, `metaharness`, `slack`, `terminal`")? {
        "claude_code" => crate::messaging::Adapter::ClaudeCode,
        "metaharness" => crate::messaging::Adapter::Metaharness,
        "slack" => crate::messaging::Adapter::Slack,
        "terminal" => crate::messaging::Adapter::Terminal,
        other => return Err(json::DecodeError { at: at.to_owned(), expected: "one of `claude_code`, `metaharness`, `slack`, `terminal`".to_owned(), found: format!("`{other}`") }),
    })
}

/// Writes `inbox.messaging.Address` as JSON.
pub fn encode_inbox_messaging_address(value: &crate::messaging::Address, out: &mut String) {
    json::push_text(out, &value.0);
}

/// Reads `inbox.messaging.Address` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_address(value: &json::Value, at: &str) -> Result<crate::messaging::Address, json::DecodeError> {
    Ok(crate::messaging::Address(json::text_at(value, at, "a string")?.to_owned()))
}

/// Writes `inbox.messaging.AnsweredBy` as JSON.
pub fn encode_inbox_messaging_answered_by(value: &crate::messaging::AnsweredBy, out: &mut String) {
    match value {
        crate::messaging::AnsweredBy::Principal => json::push_text(out, "principal"),
        crate::messaging::AnsweredBy::Silence => json::push_text(out, "silence"),
    }
}

/// Reads `inbox.messaging.AnsweredBy` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_answered_by(value: &json::Value, at: &str) -> Result<crate::messaging::AnsweredBy, json::DecodeError> {
    Ok(match json::text_at(value, at, "one of `principal`, `silence`")? {
        "principal" => crate::messaging::AnsweredBy::Principal,
        "silence" => crate::messaging::AnsweredBy::Silence,
        other => return Err(json::DecodeError { at: at.to_owned(), expected: "one of `principal`, `silence`".to_owned(), found: format!("`{other}`") }),
    })
}

/// Writes `inbox.messaging.Body` as JSON.
pub fn encode_inbox_messaging_body(value: &crate::messaging::Body, out: &mut String) {
    json::push_text(out, &value.0);
}

/// Reads `inbox.messaging.Body` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_body(value: &json::Value, at: &str) -> Result<crate::messaging::Body, json::DecodeError> {
    let decoded = crate::messaging::Body(json::text_at(value, at, "a string")?.to_owned());
    match decoded.broken_constraint() {
        Some(expected) => Err(json::DecodeError::of(at, expected, value)),
        None => Ok(decoded),
    }
}

/// Writes `inbox.messaging.DecisionAsk` as JSON.
pub fn encode_inbox_messaging_decision_ask(value: &crate::messaging::DecisionAsk, out: &mut String) {
    out.push('{');
    json::member(out, "options");
    out.push('[');
    for (index0, item0) in value.options.iter().enumerate() {
        if index0 > 0 {
            out.push(',');
        }
        encode_inbox_messaging_decision_option(&*item0, out);
    }
    out.push(']');
    if let Some(held0) = &value.recommended_option {
        json::member(out, "recommended_option");
        encode_inbox_messaging_option_letter(&*held0, out);
    }
    if let Some(held0) = &value.request_ref {
        json::member(out, "request_ref");
        json::push_text(out, &*held0);
    }
    out.push('}');
}

/// Reads `inbox.messaging.DecisionAsk` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_decision_ask(value: &json::Value, at: &str) -> Result<crate::messaging::DecisionAsk, json::DecodeError> {
    json::closed_at(value, at, &["options", "recommended_option", "request_ref"])?;
    Ok(crate::messaging::DecisionAsk {
        options: {
            let at0 = json::nested(at, "options");
            let member0 = json::member_at(value, at, "options")?;
            {
                let mut items0 = Vec::new();
                for (index0, element0) in json::items_at(member0, &at0, "an array")?.iter().enumerate() {
                    let nested0 = json::nested(&at0, &index0.to_string());
                    items0.push(decode_inbox_messaging_decision_option(element0, &nested0)?);
                }
                items0
            }
        },
        recommended_option: match value.member("recommended_option") {
            None | Some(json::Value::Null) => None,
            Some(member1) => {
                let at1 = json::nested(at, "recommended_option");
                Some(decode_inbox_messaging_option_letter(member1, &at1)?)
            }
        },
        request_ref: match value.member("request_ref") {
            None | Some(json::Value::Null) => None,
            Some(member2) => {
                let at2 = json::nested(at, "request_ref");
                Some(json::text_at(member2, &at2, "a string")?.to_owned())
            }
        },
    })
}

/// Writes `inbox.messaging.DecisionOption` as JSON.
pub fn encode_inbox_messaging_decision_option(value: &crate::messaging::DecisionOption, out: &mut String) {
    out.push('{');
    json::member(out, "letter");
    encode_inbox_messaging_option_letter(&value.letter, out);
    json::member(out, "text");
    json::push_text(out, &value.text);
    out.push('}');
}

/// Reads `inbox.messaging.DecisionOption` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_decision_option(value: &json::Value, at: &str) -> Result<crate::messaging::DecisionOption, json::DecodeError> {
    json::closed_at(value, at, &["letter", "text"])?;
    Ok(crate::messaging::DecisionOption {
        letter: {
            let at0 = json::nested(at, "letter");
            let member0 = json::member_at(value, at, "letter")?;
            decode_inbox_messaging_option_letter(member0, &at0)?
        },
        text: {
            let at1 = json::nested(at, "text");
            let member1 = json::member_at(value, at, "text")?;
            json::text_at(member1, &at1, "a string")?.to_owned()
        },
    })
}

/// Writes `inbox.messaging.DedupKey` as JSON.
pub fn encode_inbox_messaging_dedup_key(value: &crate::messaging::DedupKey, out: &mut String) {
    json::push_text(out, &value.0);
}

/// Reads `inbox.messaging.DedupKey` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_dedup_key(value: &json::Value, at: &str) -> Result<crate::messaging::DedupKey, json::DecodeError> {
    Ok(crate::messaging::DedupKey(json::text_at(value, at, "a string")?.to_owned()))
}

/// Writes `inbox.messaging.DelegationEntry` as JSON.
pub fn encode_inbox_messaging_delegation_entry(value: &crate::messaging::DelegationEntry, out: &mut String) {
    out.push('{');
    json::member(out, "delegation_id");
    encode_inbox_messaging_delegation_id(&value.delegation_id, out);
    json::member(out, "agent");
    encode_inbox_messaging_principal(&value.agent, out);
    json::member(out, "on_behalf_of");
    encode_inbox_messaging_principal(&value.on_behalf_of, out);
    json::member(out, "valid_until");
    json::push_text(out, &value.valid_until.0);
    json::member(out, "operator_signature");
    json::push_text(out, &value.operator_signature);
    out.push('}');
}

/// Reads `inbox.messaging.DelegationEntry` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_delegation_entry(value: &json::Value, at: &str) -> Result<crate::messaging::DelegationEntry, json::DecodeError> {
    json::closed_at(value, at, &["delegation_id", "agent", "on_behalf_of", "valid_until", "operator_signature"])?;
    Ok(crate::messaging::DelegationEntry {
        delegation_id: {
            let at0 = json::nested(at, "delegation_id");
            let member0 = json::member_at(value, at, "delegation_id")?;
            decode_inbox_messaging_delegation_id(member0, &at0)?
        },
        agent: {
            let at1 = json::nested(at, "agent");
            let member1 = json::member_at(value, at, "agent")?;
            decode_inbox_messaging_principal(member1, &at1)?
        },
        on_behalf_of: {
            let at2 = json::nested(at, "on_behalf_of");
            let member2 = json::member_at(value, at, "on_behalf_of")?;
            decode_inbox_messaging_principal(member2, &at2)?
        },
        valid_until: {
            let at3 = json::nested(at, "valid_until");
            let member3 = json::member_at(value, at, "valid_until")?;
            crate::primitives::Timestamp(json::timestamp_at(member3, &at3, "an RFC 3339 instant")?.to_owned())
        },
        operator_signature: {
            let at4 = json::nested(at, "operator_signature");
            let member4 = json::member_at(value, at, "operator_signature")?;
            json::text_at(member4, &at4, "a string")?.to_owned()
        },
    })
}

/// Writes `inbox.messaging.DelegationId` as JSON.
pub fn encode_inbox_messaging_delegation_id(value: &crate::messaging::DelegationId, out: &mut String) {
    json::push_text(out, &value.0);
}

/// Reads `inbox.messaging.DelegationId` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_delegation_id(value: &json::Value, at: &str) -> Result<crate::messaging::DelegationId, json::DecodeError> {
    Ok(crate::messaging::DelegationId(json::text_at(value, at, "a string")?.to_owned()))
}

/// Writes `inbox.messaging.FreeShape` as JSON.
pub fn encode_inbox_messaging_free_shape(value: &crate::messaging::FreeShape, out: &mut String) {
    out.push('{');
    out.push('}');
}

/// Reads `inbox.messaging.FreeShape` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_free_shape(value: &json::Value, at: &str) -> Result<crate::messaging::FreeShape, json::DecodeError> {
    json::members_at(value, at, "an object")?;
    json::closed_at(value, at, &[])?;
    Ok(crate::messaging::FreeShape {
    })
}

/// Writes `inbox.messaging.Inbox.State` as JSON.
pub fn encode_inbox_messaging_inbox_state(value: &crate::messaging::InboxState, out: &mut String) {
    match value {
        crate::messaging::InboxState::Closed => json::push_text(out, "Closed"),
        crate::messaging::InboxState::Open => json::push_text(out, "Open"),
    }
}

/// Reads `inbox.messaging.Inbox.State` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_inbox_state(value: &json::Value, at: &str) -> Result<crate::messaging::InboxState, json::DecodeError> {
    Ok(match json::text_at(value, at, "one of `Closed`, `Open`")? {
        "Closed" => crate::messaging::InboxState::Closed,
        "Open" => crate::messaging::InboxState::Open,
        other => return Err(json::DecodeError { at: at.to_owned(), expected: "one of `Closed`, `Open`".to_owned(), found: format!("`{other}`") }),
    })
}

/// Writes `inbox.messaging.KeyEntry` as JSON.
pub fn encode_inbox_messaging_key_entry(value: &crate::messaging::KeyEntry, out: &mut String) {
    out.push('{');
    json::member(out, "key_id");
    json::push_text(out, &value.key_id);
    json::member(out, "principal");
    encode_inbox_messaging_principal(&value.principal, out);
    json::member(out, "public_key");
    json::push_text(out, &value.public_key);
    json::member(out, "valid_until");
    json::push_text(out, &value.valid_until.0);
    json::member(out, "operator_signature");
    json::push_text(out, &value.operator_signature);
    out.push('}');
}

/// Reads `inbox.messaging.KeyEntry` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_key_entry(value: &json::Value, at: &str) -> Result<crate::messaging::KeyEntry, json::DecodeError> {
    json::closed_at(value, at, &["key_id", "principal", "public_key", "valid_until", "operator_signature"])?;
    Ok(crate::messaging::KeyEntry {
        key_id: {
            let at0 = json::nested(at, "key_id");
            let member0 = json::member_at(value, at, "key_id")?;
            json::text_at(member0, &at0, "a string")?.to_owned()
        },
        principal: {
            let at1 = json::nested(at, "principal");
            let member1 = json::member_at(value, at, "principal")?;
            decode_inbox_messaging_principal(member1, &at1)?
        },
        public_key: {
            let at2 = json::nested(at, "public_key");
            let member2 = json::member_at(value, at, "public_key")?;
            json::text_at(member2, &at2, "a string")?.to_owned()
        },
        valid_until: {
            let at3 = json::nested(at, "valid_until");
            let member3 = json::member_at(value, at, "valid_until")?;
            crate::primitives::Timestamp(json::timestamp_at(member3, &at3, "an RFC 3339 instant")?.to_owned())
        },
        operator_signature: {
            let at4 = json::nested(at, "operator_signature");
            let member4 = json::member_at(value, at, "operator_signature")?;
            json::text_at(member4, &at4, "a string")?.to_owned()
        },
    })
}

/// Writes `inbox.messaging.KeyFile` as JSON.
pub fn encode_inbox_messaging_key_file(value: &crate::messaging::KeyFile, out: &mut String) {
    out.push('{');
    json::member(out, "keys");
    out.push('[');
    for (index0, item0) in value.keys.iter().enumerate() {
        if index0 > 0 {
            out.push(',');
        }
        encode_inbox_messaging_key_entry(&*item0, out);
    }
    out.push(']');
    json::member(out, "delegations");
    out.push('[');
    for (index0, item0) in value.delegations.iter().enumerate() {
        if index0 > 0 {
            out.push(',');
        }
        encode_inbox_messaging_delegation_entry(&*item0, out);
    }
    out.push(']');
    out.push('}');
}

/// Reads `inbox.messaging.KeyFile` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_key_file(value: &json::Value, at: &str) -> Result<crate::messaging::KeyFile, json::DecodeError> {
    json::closed_at(value, at, &["keys", "delegations"])?;
    Ok(crate::messaging::KeyFile {
        keys: {
            let at0 = json::nested(at, "keys");
            let member0 = json::member_at(value, at, "keys")?;
            {
                let mut items0 = Vec::new();
                for (index0, element0) in json::items_at(member0, &at0, "an array")?.iter().enumerate() {
                    let nested0 = json::nested(&at0, &index0.to_string());
                    items0.push(decode_inbox_messaging_key_entry(element0, &nested0)?);
                }
                items0
            }
        },
        delegations: {
            let at1 = json::nested(at, "delegations");
            let member1 = json::member_at(value, at, "delegations")?;
            {
                let mut items1 = Vec::new();
                for (index1, element1) in json::items_at(member1, &at1, "an array")?.iter().enumerate() {
                    let nested1 = json::nested(&at1, &index1.to_string());
                    items1.push(decode_inbox_messaging_delegation_entry(element1, &nested1)?);
                }
                items1
            }
        },
    })
}

/// Writes `inbox.messaging.Kind` as JSON.
pub fn encode_inbox_messaging_kind(value: &crate::messaging::Kind, out: &mut String) {
    match value {
        crate::messaging::Kind::Note => json::push_text(out, "note"),
        crate::messaging::Kind::Request => json::push_text(out, "request"),
        crate::messaging::Kind::Decision => json::push_text(out, "decision"),
    }
}

/// Reads `inbox.messaging.Kind` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_kind(value: &json::Value, at: &str) -> Result<crate::messaging::Kind, json::DecodeError> {
    Ok(match json::text_at(value, at, "one of `note`, `request`, `decision`")? {
        "note" => crate::messaging::Kind::Note,
        "request" => crate::messaging::Kind::Request,
        "decision" => crate::messaging::Kind::Decision,
        other => return Err(json::DecodeError { at: at.to_owned(), expected: "one of `note`, `request`, `decision`".to_owned(), found: format!("`{other}`") }),
    })
}

/// Writes `inbox.messaging.Message.State` as JSON.
pub fn encode_inbox_messaging_message_state(value: &crate::messaging::MessageState, out: &mut String) {
    match value {
        crate::messaging::MessageState::Answered => json::push_text(out, "Answered"),
        crate::messaging::MessageState::Delivered => json::push_text(out, "Delivered"),
        crate::messaging::MessageState::Read => json::push_text(out, "Read"),
        crate::messaging::MessageState::Refused => json::push_text(out, "Refused"),
        crate::messaging::MessageState::Sent => json::push_text(out, "Sent"),
    }
}

/// Reads `inbox.messaging.Message.State` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_message_state(value: &json::Value, at: &str) -> Result<crate::messaging::MessageState, json::DecodeError> {
    Ok(match json::text_at(value, at, "one of `Answered`, `Delivered`, `Read`, `Refused`, `Sent`")? {
        "Answered" => crate::messaging::MessageState::Answered,
        "Delivered" => crate::messaging::MessageState::Delivered,
        "Read" => crate::messaging::MessageState::Read,
        "Refused" => crate::messaging::MessageState::Refused,
        "Sent" => crate::messaging::MessageState::Sent,
        other => return Err(json::DecodeError { at: at.to_owned(), expected: "one of `Answered`, `Delivered`, `Read`, `Refused`, `Sent`".to_owned(), found: format!("`{other}`") }),
    })
}

/// Writes `inbox.messaging.MessageId` as JSON.
pub fn encode_inbox_messaging_message_id(value: &crate::messaging::MessageId, out: &mut String) {
    json::push_text(out, &value.0);
}

/// Reads `inbox.messaging.MessageId` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_message_id(value: &json::Value, at: &str) -> Result<crate::messaging::MessageId, json::DecodeError> {
    let decoded = crate::messaging::MessageId(json::text_at(value, at, "a string")?.to_owned());
    match decoded.broken_constraint() {
        Some(expected) => Err(json::DecodeError::of(at, expected, value)),
        None => Ok(decoded),
    }
}

/// Writes `inbox.messaging.OptionLetter` as JSON.
pub fn encode_inbox_messaging_option_letter(value: &crate::messaging::OptionLetter, out: &mut String) {
    json::push_text(out, &value.0);
}

/// Reads `inbox.messaging.OptionLetter` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_option_letter(value: &json::Value, at: &str) -> Result<crate::messaging::OptionLetter, json::DecodeError> {
    let decoded = crate::messaging::OptionLetter(json::text_at(value, at, "a string")?.to_owned());
    match decoded.broken_constraint() {
        Some(expected) => Err(json::DecodeError::of(at, expected, value)),
        None => Ok(decoded),
    }
}

/// Writes `inbox.messaging.Principal` as JSON.
pub fn encode_inbox_messaging_principal(value: &crate::messaging::Principal, out: &mut String) {
    json::push_text(out, &value.0);
}

/// Reads `inbox.messaging.Principal` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_principal(value: &json::Value, at: &str) -> Result<crate::messaging::Principal, json::DecodeError> {
    Ok(crate::messaging::Principal(json::text_at(value, at, "a string")?.to_owned()))
}

/// Writes `inbox.messaging.Reply` as JSON.
pub fn encode_inbox_messaging_reply(value: &crate::messaging::Reply, out: &mut String) {
    out.push('{');
    json::member(out, "reply_id");
    encode_inbox_messaging_message_id(&value.reply_id, out);
    json::member(out, "from");
    encode_inbox_messaging_principal(&value.from, out);
    json::member(out, "on_behalf_of");
    encode_inbox_messaging_principal(&value.on_behalf_of, out);
    if let Some(held0) = &value.authority {
        json::member(out, "authority");
        encode_inbox_messaging_delegation_id(&*held0, out);
    }
    json::member(out, "created");
    json::push_text(out, &value.created.0);
    if let Some(held0) = &value.option {
        json::member(out, "option");
        encode_inbox_messaging_option_letter(&*held0, out);
    }
    json::member(out, "body");
    encode_inbox_messaging_body(&value.body, out);
    json::member(out, "signature");
    encode_inbox_messaging_signature(&value.signature, out);
    out.push('}');
}

/// Reads `inbox.messaging.Reply` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_reply(value: &json::Value, at: &str) -> Result<crate::messaging::Reply, json::DecodeError> {
    json::closed_at(value, at, &["reply_id", "from", "on_behalf_of", "authority", "created", "option", "body", "signature"])?;
    Ok(crate::messaging::Reply {
        reply_id: {
            let at0 = json::nested(at, "reply_id");
            let member0 = json::member_at(value, at, "reply_id")?;
            decode_inbox_messaging_message_id(member0, &at0)?
        },
        from: {
            let at1 = json::nested(at, "from");
            let member1 = json::member_at(value, at, "from")?;
            decode_inbox_messaging_principal(member1, &at1)?
        },
        on_behalf_of: {
            let at2 = json::nested(at, "on_behalf_of");
            let member2 = json::member_at(value, at, "on_behalf_of")?;
            decode_inbox_messaging_principal(member2, &at2)?
        },
        authority: match value.member("authority") {
            None | Some(json::Value::Null) => None,
            Some(member3) => {
                let at3 = json::nested(at, "authority");
                Some(decode_inbox_messaging_delegation_id(member3, &at3)?)
            }
        },
        created: {
            let at4 = json::nested(at, "created");
            let member4 = json::member_at(value, at, "created")?;
            crate::primitives::Timestamp(json::timestamp_at(member4, &at4, "an RFC 3339 instant")?.to_owned())
        },
        option: match value.member("option") {
            None | Some(json::Value::Null) => None,
            Some(member5) => {
                let at5 = json::nested(at, "option");
                Some(decode_inbox_messaging_option_letter(member5, &at5)?)
            }
        },
        body: {
            let at6 = json::nested(at, "body");
            let member6 = json::member_at(value, at, "body")?;
            decode_inbox_messaging_body(member6, &at6)?
        },
        signature: {
            let at7 = json::nested(at, "signature");
            let member7 = json::member_at(value, at, "signature")?;
            decode_inbox_messaging_signature(member7, &at7)?
        },
    })
}

/// Writes `inbox.messaging.ReplyShape` as JSON.
pub fn encode_inbox_messaging_reply_shape(value: &crate::messaging::ReplyShape, out: &mut String) {
    match value {
        crate::messaging::ReplyShape::Free(held) => {
            out.push('{');
            json::member(out, "shape");
            json::push_text(out, "free");
            json::member(out, "value");
            encode_inbox_messaging_free_shape(&*held, out);
            out.push('}');
        }
        crate::messaging::ReplyShape::Lines(held) => {
            out.push('{');
            json::member(out, "shape");
            json::push_text(out, "lines");
            json::member(out, "value");
            json::push_integer(out, *held);
            out.push('}');
        }
        crate::messaging::ReplyShape::Schema(held) => {
            out.push('{');
            json::member(out, "shape");
            json::push_text(out, "schema");
            json::member(out, "value");
            json::push_text(out, &*held);
            out.push('}');
        }
    }
}

/// Reads `inbox.messaging.ReplyShape` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_reply_shape(value: &json::Value, at: &str) -> Result<crate::messaging::ReplyShape, json::DecodeError> {
    json::closed_at(value, at, &["shape", "value"])?;
    let tag = json::member_at(value, at, "shape")?;
    let at_tag = json::nested(at, "shape");
    Ok(match json::text_at(tag, &at_tag, "one of `free`, `lines`, `schema`")? {
        "free" => crate::messaging::ReplyShape::Free({
            let at0 = json::nested(at, "value");
            let member0 = json::member_at(value, at, "value")?;
            decode_inbox_messaging_free_shape(member0, &at0)?
        }),
        "lines" => crate::messaging::ReplyShape::Lines({
            let at1 = json::nested(at, "value");
            let member1 = json::member_at(value, at, "value")?;
            json::integer_at(member1, &at1, "an integer")?
        }),
        "schema" => crate::messaging::ReplyShape::Schema({
            let at2 = json::nested(at, "value");
            let member2 = json::member_at(value, at, "value")?;
            json::text_at(member2, &at2, "a string")?.to_owned()
        }),
        other => return Err(json::DecodeError { at: at_tag.clone(), expected: "one of `free`, `lines`, `schema`".to_owned(), found: format!("`{other}`") }),
    })
}

/// Writes `inbox.messaging.Signature` as JSON.
pub fn encode_inbox_messaging_signature(value: &crate::messaging::Signature, out: &mut String) {
    out.push('{');
    json::member(out, "key_id");
    json::push_text(out, &value.key_id);
    json::member(out, "value");
    json::push_text(out, &value.value);
    out.push('}');
}

/// Reads `inbox.messaging.Signature` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_signature(value: &json::Value, at: &str) -> Result<crate::messaging::Signature, json::DecodeError> {
    json::closed_at(value, at, &["key_id", "value"])?;
    Ok(crate::messaging::Signature {
        key_id: {
            let at0 = json::nested(at, "key_id");
            let member0 = json::member_at(value, at, "key_id")?;
            json::text_at(member0, &at0, "a string")?.to_owned()
        },
        value: {
            let at1 = json::nested(at, "value");
            let member1 = json::member_at(value, at, "value")?;
            json::text_at(member1, &at1, "a string")?.to_owned()
        },
    })
}

/// Writes `inbox.messaging.ThreadId` as JSON.
pub fn encode_inbox_messaging_thread_id(value: &crate::messaging::ThreadId, out: &mut String) {
    json::push_text(out, &value.0);
}

/// Reads `inbox.messaging.ThreadId` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_inbox_messaging_thread_id(value: &json::Value, at: &str) -> Result<crate::messaging::ThreadId, json::DecodeError> {
    let decoded = crate::messaging::ThreadId(json::text_at(value, at, "a string")?.to_owned());
    match decoded.broken_constraint() {
        Some(expected) => Err(json::DecodeError::of(at, expected, value)),
        None => Ok(decoded),
    }
}

/// Writes the event `inbox.messaging.DecisionTakenOnSilence` as JSON.
pub fn encode_event_inbox_messaging_decision_taken_on_silence(value: &crate::messaging::DecisionTakenOnSilence, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    json::member(out, "to");
    encode_inbox_messaging_principal(&value.to, out);
    json::member(out, "thread");
    encode_inbox_messaging_thread_id(&value.thread, out);
    out.push('}');
}

/// Reads the event `inbox.messaging.DecisionTakenOnSilence` from JSON, the payload [`encode_event_inbox_messaging_decision_taken_on_silence`] writes.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_event_inbox_messaging_decision_taken_on_silence(value: &json::Value, at: &str) -> Result<crate::messaging::DecisionTakenOnSilence, json::DecodeError> {
    Ok(crate::messaging::DecisionTakenOnSilence {
        message_id: {
            let at0 = json::nested(at, "message_id");
            let member0 = json::member_at(value, at, "message_id")?;
            decode_inbox_messaging_message_id(member0, &at0)?
        },
        to: {
            let at1 = json::nested(at, "to");
            let member1 = json::member_at(value, at, "to")?;
            decode_inbox_messaging_principal(member1, &at1)?
        },
        thread: {
            let at2 = json::nested(at, "thread");
            let member2 = json::member_at(value, at, "thread")?;
            decode_inbox_messaging_thread_id(member2, &at2)?
        },
    })
}

/// Writes the event `inbox.messaging.InboxClosed` as JSON.
pub fn encode_event_inbox_messaging_inbox_closed(value: &crate::messaging::InboxClosed, out: &mut String) {
    out.push('{');
    json::member(out, "address");
    encode_inbox_messaging_address(&value.address, out);
    out.push('}');
}

/// Reads the event `inbox.messaging.InboxClosed` from JSON, the payload [`encode_event_inbox_messaging_inbox_closed`] writes.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_event_inbox_messaging_inbox_closed(value: &json::Value, at: &str) -> Result<crate::messaging::InboxClosed, json::DecodeError> {
    Ok(crate::messaging::InboxClosed {
        address: {
            let at0 = json::nested(at, "address");
            let member0 = json::member_at(value, at, "address")?;
            decode_inbox_messaging_address(member0, &at0)?
        },
    })
}

/// Writes the event `inbox.messaging.InboxOpened` as JSON.
pub fn encode_event_inbox_messaging_inbox_opened(value: &crate::messaging::InboxOpened, out: &mut String) {
    out.push('{');
    json::member(out, "address");
    encode_inbox_messaging_address(&value.address, out);
    out.push('}');
}

/// Reads the event `inbox.messaging.InboxOpened` from JSON, the payload [`encode_event_inbox_messaging_inbox_opened`] writes.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_event_inbox_messaging_inbox_opened(value: &json::Value, at: &str) -> Result<crate::messaging::InboxOpened, json::DecodeError> {
    Ok(crate::messaging::InboxOpened {
        address: {
            let at0 = json::nested(at, "address");
            let member0 = json::member_at(value, at, "address")?;
            decode_inbox_messaging_address(member0, &at0)?
        },
    })
}

/// Writes the event `inbox.messaging.MessageAnswered` as JSON.
pub fn encode_event_inbox_messaging_message_answered(value: &crate::messaging::MessageAnswered, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    json::member(out, "reply_id");
    encode_inbox_messaging_message_id(&value.reply_id, out);
    json::member(out, "to");
    encode_inbox_messaging_principal(&value.to, out);
    json::member(out, "thread");
    encode_inbox_messaging_thread_id(&value.thread, out);
    out.push('}');
}

/// Reads the event `inbox.messaging.MessageAnswered` from JSON, the payload [`encode_event_inbox_messaging_message_answered`] writes.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_event_inbox_messaging_message_answered(value: &json::Value, at: &str) -> Result<crate::messaging::MessageAnswered, json::DecodeError> {
    Ok(crate::messaging::MessageAnswered {
        message_id: {
            let at0 = json::nested(at, "message_id");
            let member0 = json::member_at(value, at, "message_id")?;
            decode_inbox_messaging_message_id(member0, &at0)?
        },
        reply_id: {
            let at1 = json::nested(at, "reply_id");
            let member1 = json::member_at(value, at, "reply_id")?;
            decode_inbox_messaging_message_id(member1, &at1)?
        },
        to: {
            let at2 = json::nested(at, "to");
            let member2 = json::member_at(value, at, "to")?;
            decode_inbox_messaging_principal(member2, &at2)?
        },
        thread: {
            let at3 = json::nested(at, "thread");
            let member3 = json::member_at(value, at, "thread")?;
            decode_inbox_messaging_thread_id(member3, &at3)?
        },
    })
}

/// Writes the event `inbox.messaging.MessageDelivered` as JSON.
pub fn encode_event_inbox_messaging_message_delivered(value: &crate::messaging::MessageDelivered, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    out.push('}');
}

/// Reads the event `inbox.messaging.MessageDelivered` from JSON, the payload [`encode_event_inbox_messaging_message_delivered`] writes.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_event_inbox_messaging_message_delivered(value: &json::Value, at: &str) -> Result<crate::messaging::MessageDelivered, json::DecodeError> {
    Ok(crate::messaging::MessageDelivered {
        message_id: {
            let at0 = json::nested(at, "message_id");
            let member0 = json::member_at(value, at, "message_id")?;
            decode_inbox_messaging_message_id(member0, &at0)?
        },
    })
}

/// Writes the event `inbox.messaging.MessageExpired` as JSON.
pub fn encode_event_inbox_messaging_message_expired(value: &crate::messaging::MessageExpired, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    json::member(out, "to");
    encode_inbox_messaging_principal(&value.to, out);
    json::member(out, "thread");
    encode_inbox_messaging_thread_id(&value.thread, out);
    out.push('}');
}

/// Reads the event `inbox.messaging.MessageExpired` from JSON, the payload [`encode_event_inbox_messaging_message_expired`] writes.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_event_inbox_messaging_message_expired(value: &json::Value, at: &str) -> Result<crate::messaging::MessageExpired, json::DecodeError> {
    Ok(crate::messaging::MessageExpired {
        message_id: {
            let at0 = json::nested(at, "message_id");
            let member0 = json::member_at(value, at, "message_id")?;
            decode_inbox_messaging_message_id(member0, &at0)?
        },
        to: {
            let at1 = json::nested(at, "to");
            let member1 = json::member_at(value, at, "to")?;
            decode_inbox_messaging_principal(member1, &at1)?
        },
        thread: {
            let at2 = json::nested(at, "thread");
            let member2 = json::member_at(value, at, "thread")?;
            decode_inbox_messaging_thread_id(member2, &at2)?
        },
    })
}

/// Writes the event `inbox.messaging.MessageHandled` as JSON.
pub fn encode_event_inbox_messaging_message_handled(value: &crate::messaging::MessageHandled, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    out.push('}');
}

/// Reads the event `inbox.messaging.MessageHandled` from JSON, the payload [`encode_event_inbox_messaging_message_handled`] writes.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_event_inbox_messaging_message_handled(value: &json::Value, at: &str) -> Result<crate::messaging::MessageHandled, json::DecodeError> {
    Ok(crate::messaging::MessageHandled {
        message_id: {
            let at0 = json::nested(at, "message_id");
            let member0 = json::member_at(value, at, "message_id")?;
            decode_inbox_messaging_message_id(member0, &at0)?
        },
    })
}

/// Writes the event `inbox.messaging.MessagePosted` as JSON.
pub fn encode_event_inbox_messaging_message_posted(value: &crate::messaging::MessagePosted, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    json::member(out, "to");
    encode_inbox_messaging_address(&value.to, out);
    json::member(out, "from");
    encode_inbox_messaging_principal(&value.from, out);
    json::member(out, "thread");
    encode_inbox_messaging_thread_id(&value.thread, out);
    json::member(out, "kind");
    encode_inbox_messaging_kind(&value.kind, out);
    out.push('}');
}

/// Reads the event `inbox.messaging.MessagePosted` from JSON, the payload [`encode_event_inbox_messaging_message_posted`] writes.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_event_inbox_messaging_message_posted(value: &json::Value, at: &str) -> Result<crate::messaging::MessagePosted, json::DecodeError> {
    Ok(crate::messaging::MessagePosted {
        message_id: {
            let at0 = json::nested(at, "message_id");
            let member0 = json::member_at(value, at, "message_id")?;
            decode_inbox_messaging_message_id(member0, &at0)?
        },
        to: {
            let at1 = json::nested(at, "to");
            let member1 = json::member_at(value, at, "to")?;
            decode_inbox_messaging_address(member1, &at1)?
        },
        from: {
            let at2 = json::nested(at, "from");
            let member2 = json::member_at(value, at, "from")?;
            decode_inbox_messaging_principal(member2, &at2)?
        },
        thread: {
            let at3 = json::nested(at, "thread");
            let member3 = json::member_at(value, at, "thread")?;
            decode_inbox_messaging_thread_id(member3, &at3)?
        },
        kind: {
            let at4 = json::nested(at, "kind");
            let member4 = json::member_at(value, at, "kind")?;
            decode_inbox_messaging_kind(member4, &at4)?
        },
    })
}

/// Writes the event `inbox.messaging.MessageRead` as JSON.
pub fn encode_event_inbox_messaging_message_read(value: &crate::messaging::MessageRead, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    out.push('}');
}

/// Reads the event `inbox.messaging.MessageRead` from JSON, the payload [`encode_event_inbox_messaging_message_read`] writes.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_event_inbox_messaging_message_read(value: &json::Value, at: &str) -> Result<crate::messaging::MessageRead, json::DecodeError> {
    Ok(crate::messaging::MessageRead {
        message_id: {
            let at0 = json::nested(at, "message_id");
            let member0 = json::member_at(value, at, "message_id")?;
            decode_inbox_messaging_message_id(member0, &at0)?
        },
    })
}

/// Writes the event `inbox.messaging.MessageRefused` as JSON.
pub fn encode_event_inbox_messaging_message_refused(value: &crate::messaging::MessageRefused, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    json::member(out, "to");
    encode_inbox_messaging_principal(&value.to, out);
    json::member(out, "thread");
    encode_inbox_messaging_thread_id(&value.thread, out);
    out.push('}');
}

/// Reads the event `inbox.messaging.MessageRefused` from JSON, the payload [`encode_event_inbox_messaging_message_refused`] writes.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_event_inbox_messaging_message_refused(value: &json::Value, at: &str) -> Result<crate::messaging::MessageRefused, json::DecodeError> {
    Ok(crate::messaging::MessageRefused {
        message_id: {
            let at0 = json::nested(at, "message_id");
            let member0 = json::member_at(value, at, "message_id")?;
            decode_inbox_messaging_message_id(member0, &at0)?
        },
        to: {
            let at1 = json::nested(at, "to");
            let member1 = json::member_at(value, at, "to")?;
            decode_inbox_messaging_principal(member1, &at1)?
        },
        thread: {
            let at2 = json::nested(at, "thread");
            let member2 = json::member_at(value, at, "thread")?;
            decode_inbox_messaging_thread_id(member2, &at2)?
        },
    })
}

/// Writes the declared error `inbox.messaging.AlreadyExpired` as JSON.
pub fn encode_error_inbox_messaging_already_expired(_value: &crate::messaging::AlreadyExpired, out: &mut String) {
    out.push('{');
    out.push('}');
}

/// Writes the declared error `inbox.messaging.AlreadySettled` as JSON.
pub fn encode_error_inbox_messaging_already_settled(value: &crate::messaging::AlreadySettled, out: &mut String) {
    out.push('{');
    json::member(out, "state");
    encode_inbox_messaging_message_state(&value.state, out);
    out.push('}');
}

/// Writes the declared error `inbox.messaging.Duplicate` as JSON.
pub fn encode_error_inbox_messaging_duplicate(_value: &crate::messaging::Duplicate, out: &mut String) {
    out.push('{');
    out.push('}');
}

/// Writes the declared error `inbox.messaging.Expired` as JSON.
pub fn encode_error_inbox_messaging_expired(_value: &crate::messaging::Expired, out: &mut String) {
    out.push('{');
    out.push('}');
}

/// Writes the declared error `inbox.messaging.MessageIdTaken` as JSON.
pub fn encode_error_inbox_messaging_message_id_taken(_value: &crate::messaging::MessageIdTaken, out: &mut String) {
    out.push('{');
    out.push('}');
}

/// Writes the declared error `inbox.messaging.NoAuthority` as JSON.
pub fn encode_error_inbox_messaging_no_authority(_value: &crate::messaging::NoAuthority, out: &mut String) {
    out.push('{');
    out.push('}');
}

/// Writes the declared error `inbox.messaging.NotAnswerable` as JSON.
pub fn encode_error_inbox_messaging_not_answerable(_value: &crate::messaging::NotAnswerable, out: &mut String) {
    out.push('{');
    out.push('}');
}

/// Writes the declared error `inbox.messaging.NotDue` as JSON.
pub fn encode_error_inbox_messaging_not_due(_value: &crate::messaging::NotDue, out: &mut String) {
    out.push('{');
    out.push('}');
}

/// Writes the declared error `inbox.messaging.NotRecipient` as JSON.
pub fn encode_error_inbox_messaging_not_recipient(_value: &crate::messaging::NotRecipient, out: &mut String) {
    out.push('{');
    out.push('}');
}

/// Writes the declared error `inbox.messaging.NotSender` as JSON.
pub fn encode_error_inbox_messaging_not_sender(_value: &crate::messaging::NotSender, out: &mut String) {
    out.push('{');
    out.push('}');
}

/// Writes the declared error `inbox.messaging.NotSettled` as JSON.
pub fn encode_error_inbox_messaging_not_settled(_value: &crate::messaging::NotSettled, out: &mut String) {
    out.push('{');
    out.push('}');
}

/// Writes the declared error `inbox.messaging.ShapeMismatch` as JSON.
pub fn encode_error_inbox_messaging_shape_mismatch(_value: &crate::messaging::ShapeMismatch, out: &mut String) {
    out.push('{');
    out.push('}');
}

/// Writes the declared error `inbox.messaging.SignatureInvalid` as JSON.
pub fn encode_error_inbox_messaging_signature_invalid(_value: &crate::messaging::SignatureInvalid, out: &mut String) {
    out.push('{');
    out.push('}');
}

/// Writes the declared error `inbox.messaging.UnknownInbox` as JSON.
pub fn encode_error_inbox_messaging_unknown_inbox(_value: &crate::messaging::UnknownInbox, out: &mut String) {
    out.push('{');
    out.push('}');
}

/// Writes the declared error `inbox.messaging.UnknownMessage` as JSON.
pub fn encode_error_inbox_messaging_unknown_message(_value: &crate::messaging::UnknownMessage, out: &mut String) {
    out.push('{');
    out.push('}');
}

/// Writes one row of the view `inbox.messaging.Due` as JSON.
pub fn encode_view_inbox_messaging_due(value: &crate::messaging::Due, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    json::member(out, "from");
    encode_inbox_messaging_principal(&value.from, out);
    json::member(out, "on_behalf_of");
    encode_inbox_messaging_principal(&value.on_behalf_of, out);
    if let Some(held0) = &value.authority {
        json::member(out, "authority");
        encode_inbox_messaging_delegation_id(&*held0, out);
    }
    json::member(out, "to");
    encode_inbox_messaging_address(&value.to, out);
    json::member(out, "kind");
    encode_inbox_messaging_kind(&value.kind, out);
    json::member(out, "thread");
    encode_inbox_messaging_thread_id(&value.thread, out);
    if let Some(held0) = &value.reply_shape {
        json::member(out, "reply_shape");
        encode_inbox_messaging_reply_shape(&*held0, out);
    }
    if let Some(held0) = &value.decision {
        json::member(out, "decision");
        encode_inbox_messaging_decision_ask(&*held0, out);
    }
    if let Some(held0) = &value.default_on_silence {
        json::member(out, "default_on_silence");
        encode_inbox_messaging_option_letter(&*held0, out);
    }
    json::member(out, "created");
    json::push_text(out, &value.created.0);
    json::member(out, "expires");
    json::push_text(out, &value.expires.0);
    json::member(out, "dedup_key");
    encode_inbox_messaging_dedup_key(&value.dedup_key, out);
    json::member(out, "body");
    encode_inbox_messaging_body(&value.body, out);
    json::member(out, "signature");
    encode_inbox_messaging_signature(&value.signature, out);
    if let Some(held0) = &value.answered_by {
        json::member(out, "answered_by");
        encode_inbox_messaging_answered_by(&*held0, out);
    }
    if let Some(held0) = &value.decided_option {
        json::member(out, "decided_option");
        encode_inbox_messaging_option_letter(&*held0, out);
    }
    if let Some(held0) = &value.reply {
        json::member(out, "reply");
        encode_inbox_messaging_reply(&*held0, out);
    }
    if let Some(held0) = &value.refusal {
        json::member(out, "refusal");
        json::push_text(out, &*held0);
    }
    json::member(out, "state");
    encode_inbox_messaging_message_state(&value.state, out);
    out.push('}');
}

/// Reads the parameters of `inbox.messaging.Due` from an object keyed by their wire names.
///
/// # Errors
///
/// [`json::DecodeError`] naming the parameter and what the declaration says belongs there.
pub fn decode_params_inbox_messaging_due(value: &json::Value, at: &str) -> Result<(crate::primitives::Timestamp,), json::DecodeError> {
    Ok((
        {
            let at0 = json::nested(at, "now");
            let member0 = json::member_at(value, at, "now")?;
            crate::primitives::Timestamp(json::timestamp_at(member0, &at0, "an RFC 3339 instant")?.to_owned())
        },
    ))
}

/// Writes one row of the view `inbox.messaging.Inboxes` as JSON.
pub fn encode_view_inbox_messaging_inboxes(value: &crate::messaging::Inboxes, out: &mut String) {
    out.push('{');
    json::member(out, "address");
    encode_inbox_messaging_address(&value.address, out);
    json::member(out, "owner");
    encode_inbox_messaging_principal(&value.owner, out);
    json::member(out, "adapter");
    encode_inbox_messaging_adapter(&value.adapter, out);
    json::member(out, "state");
    encode_inbox_messaging_inbox_state(&value.state, out);
    out.push('}');
}

/// Writes one row of the view `inbox.messaging.Messages` as JSON.
pub fn encode_view_inbox_messaging_messages(value: &crate::messaging::Messages, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    json::member(out, "from");
    encode_inbox_messaging_principal(&value.from, out);
    json::member(out, "on_behalf_of");
    encode_inbox_messaging_principal(&value.on_behalf_of, out);
    if let Some(held0) = &value.authority {
        json::member(out, "authority");
        encode_inbox_messaging_delegation_id(&*held0, out);
    }
    json::member(out, "to");
    encode_inbox_messaging_address(&value.to, out);
    json::member(out, "kind");
    encode_inbox_messaging_kind(&value.kind, out);
    json::member(out, "thread");
    encode_inbox_messaging_thread_id(&value.thread, out);
    if let Some(held0) = &value.reply_shape {
        json::member(out, "reply_shape");
        encode_inbox_messaging_reply_shape(&*held0, out);
    }
    if let Some(held0) = &value.decision {
        json::member(out, "decision");
        encode_inbox_messaging_decision_ask(&*held0, out);
    }
    if let Some(held0) = &value.default_on_silence {
        json::member(out, "default_on_silence");
        encode_inbox_messaging_option_letter(&*held0, out);
    }
    json::member(out, "created");
    json::push_text(out, &value.created.0);
    json::member(out, "expires");
    json::push_text(out, &value.expires.0);
    json::member(out, "dedup_key");
    encode_inbox_messaging_dedup_key(&value.dedup_key, out);
    json::member(out, "body");
    encode_inbox_messaging_body(&value.body, out);
    json::member(out, "signature");
    encode_inbox_messaging_signature(&value.signature, out);
    if let Some(held0) = &value.answered_by {
        json::member(out, "answered_by");
        encode_inbox_messaging_answered_by(&*held0, out);
    }
    if let Some(held0) = &value.decided_option {
        json::member(out, "decided_option");
        encode_inbox_messaging_option_letter(&*held0, out);
    }
    if let Some(held0) = &value.reply {
        json::member(out, "reply");
        encode_inbox_messaging_reply(&*held0, out);
    }
    if let Some(held0) = &value.refusal {
        json::member(out, "refusal");
        json::push_text(out, &*held0);
    }
    json::member(out, "state");
    encode_inbox_messaging_message_state(&value.state, out);
    out.push('}');
}

/// Writes one row of the view `inbox.messaging.Outbox` as JSON.
pub fn encode_view_inbox_messaging_outbox(value: &crate::messaging::Outbox, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    json::member(out, "from");
    encode_inbox_messaging_principal(&value.from, out);
    json::member(out, "on_behalf_of");
    encode_inbox_messaging_principal(&value.on_behalf_of, out);
    if let Some(held0) = &value.authority {
        json::member(out, "authority");
        encode_inbox_messaging_delegation_id(&*held0, out);
    }
    json::member(out, "to");
    encode_inbox_messaging_address(&value.to, out);
    json::member(out, "kind");
    encode_inbox_messaging_kind(&value.kind, out);
    json::member(out, "thread");
    encode_inbox_messaging_thread_id(&value.thread, out);
    if let Some(held0) = &value.reply_shape {
        json::member(out, "reply_shape");
        encode_inbox_messaging_reply_shape(&*held0, out);
    }
    if let Some(held0) = &value.decision {
        json::member(out, "decision");
        encode_inbox_messaging_decision_ask(&*held0, out);
    }
    if let Some(held0) = &value.default_on_silence {
        json::member(out, "default_on_silence");
        encode_inbox_messaging_option_letter(&*held0, out);
    }
    json::member(out, "created");
    json::push_text(out, &value.created.0);
    json::member(out, "expires");
    json::push_text(out, &value.expires.0);
    json::member(out, "dedup_key");
    encode_inbox_messaging_dedup_key(&value.dedup_key, out);
    json::member(out, "body");
    encode_inbox_messaging_body(&value.body, out);
    json::member(out, "signature");
    encode_inbox_messaging_signature(&value.signature, out);
    if let Some(held0) = &value.answered_by {
        json::member(out, "answered_by");
        encode_inbox_messaging_answered_by(&*held0, out);
    }
    if let Some(held0) = &value.decided_option {
        json::member(out, "decided_option");
        encode_inbox_messaging_option_letter(&*held0, out);
    }
    if let Some(held0) = &value.reply {
        json::member(out, "reply");
        encode_inbox_messaging_reply(&*held0, out);
    }
    if let Some(held0) = &value.refusal {
        json::member(out, "refusal");
        json::push_text(out, &*held0);
    }
    json::member(out, "state");
    encode_inbox_messaging_message_state(&value.state, out);
    out.push('}');
}

/// Reads the parameters of `inbox.messaging.Outbox` from an object keyed by their wire names.
///
/// # Errors
///
/// [`json::DecodeError`] naming the parameter and what the declaration says belongs there.
pub fn decode_params_inbox_messaging_outbox(value: &json::Value, at: &str) -> Result<(crate::messaging::Principal,), json::DecodeError> {
    Ok((
        {
            let at0 = json::nested(at, "sender");
            let member0 = json::member_at(value, at, "sender")?;
            decode_inbox_messaging_principal(member0, &at0)?
        },
    ))
}

/// Writes one row of the view `inbox.messaging.Pending` as JSON.
pub fn encode_view_inbox_messaging_pending(value: &crate::messaging::Pending, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    json::member(out, "from");
    encode_inbox_messaging_principal(&value.from, out);
    json::member(out, "on_behalf_of");
    encode_inbox_messaging_principal(&value.on_behalf_of, out);
    if let Some(held0) = &value.authority {
        json::member(out, "authority");
        encode_inbox_messaging_delegation_id(&*held0, out);
    }
    json::member(out, "to");
    encode_inbox_messaging_address(&value.to, out);
    json::member(out, "kind");
    encode_inbox_messaging_kind(&value.kind, out);
    json::member(out, "thread");
    encode_inbox_messaging_thread_id(&value.thread, out);
    if let Some(held0) = &value.reply_shape {
        json::member(out, "reply_shape");
        encode_inbox_messaging_reply_shape(&*held0, out);
    }
    if let Some(held0) = &value.decision {
        json::member(out, "decision");
        encode_inbox_messaging_decision_ask(&*held0, out);
    }
    if let Some(held0) = &value.default_on_silence {
        json::member(out, "default_on_silence");
        encode_inbox_messaging_option_letter(&*held0, out);
    }
    json::member(out, "created");
    json::push_text(out, &value.created.0);
    json::member(out, "expires");
    json::push_text(out, &value.expires.0);
    json::member(out, "dedup_key");
    encode_inbox_messaging_dedup_key(&value.dedup_key, out);
    json::member(out, "body");
    encode_inbox_messaging_body(&value.body, out);
    json::member(out, "signature");
    encode_inbox_messaging_signature(&value.signature, out);
    json::member(out, "state");
    encode_inbox_messaging_message_state(&value.state, out);
    out.push('}');
}

/// Reads the parameters of `inbox.messaging.Pending` from an object keyed by their wire names.
///
/// # Errors
///
/// [`json::DecodeError`] naming the parameter and what the declaration says belongs there.
pub fn decode_params_inbox_messaging_pending(value: &json::Value, at: &str) -> Result<(crate::messaging::Address,), json::DecodeError> {
    Ok((
        {
            let at0 = json::nested(at, "inbox");
            let member0 = json::member_at(value, at, "inbox")?;
            decode_inbox_messaging_address(member0, &at0)?
        },
    ))
}

/// Writes one row of the view `inbox.messaging.Replies` as JSON.
pub fn encode_view_inbox_messaging_replies(value: &crate::messaging::Replies, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    json::member(out, "from");
    encode_inbox_messaging_principal(&value.from, out);
    json::member(out, "on_behalf_of");
    encode_inbox_messaging_principal(&value.on_behalf_of, out);
    if let Some(held0) = &value.authority {
        json::member(out, "authority");
        encode_inbox_messaging_delegation_id(&*held0, out);
    }
    json::member(out, "to");
    encode_inbox_messaging_address(&value.to, out);
    json::member(out, "kind");
    encode_inbox_messaging_kind(&value.kind, out);
    json::member(out, "thread");
    encode_inbox_messaging_thread_id(&value.thread, out);
    if let Some(held0) = &value.reply_shape {
        json::member(out, "reply_shape");
        encode_inbox_messaging_reply_shape(&*held0, out);
    }
    if let Some(held0) = &value.decision {
        json::member(out, "decision");
        encode_inbox_messaging_decision_ask(&*held0, out);
    }
    if let Some(held0) = &value.default_on_silence {
        json::member(out, "default_on_silence");
        encode_inbox_messaging_option_letter(&*held0, out);
    }
    json::member(out, "created");
    json::push_text(out, &value.created.0);
    json::member(out, "expires");
    json::push_text(out, &value.expires.0);
    json::member(out, "dedup_key");
    encode_inbox_messaging_dedup_key(&value.dedup_key, out);
    json::member(out, "body");
    encode_inbox_messaging_body(&value.body, out);
    json::member(out, "signature");
    encode_inbox_messaging_signature(&value.signature, out);
    if let Some(held0) = &value.answered_by {
        json::member(out, "answered_by");
        encode_inbox_messaging_answered_by(&*held0, out);
    }
    if let Some(held0) = &value.decided_option {
        json::member(out, "decided_option");
        encode_inbox_messaging_option_letter(&*held0, out);
    }
    if let Some(held0) = &value.reply {
        json::member(out, "reply");
        encode_inbox_messaging_reply(&*held0, out);
    }
    json::member(out, "state");
    encode_inbox_messaging_message_state(&value.state, out);
    out.push('}');
}

/// Reads the parameters of `inbox.messaging.Replies` from an object keyed by their wire names.
///
/// # Errors
///
/// [`json::DecodeError`] naming the parameter and what the declaration says belongs there.
pub fn decode_params_inbox_messaging_replies(value: &json::Value, at: &str) -> Result<(crate::messaging::ThreadId,), json::DecodeError> {
    Ok((
        {
            let at0 = json::nested(at, "thread");
            let member0 = json::member_at(value, at, "thread")?;
            decode_inbox_messaging_thread_id(member0, &at0)?
        },
    ))
}

/// Writes one row of the view `inbox.messaging.Settled` as JSON.
pub fn encode_view_inbox_messaging_settled(value: &crate::messaging::Settled, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    json::member(out, "from");
    encode_inbox_messaging_principal(&value.from, out);
    json::member(out, "on_behalf_of");
    encode_inbox_messaging_principal(&value.on_behalf_of, out);
    if let Some(held0) = &value.authority {
        json::member(out, "authority");
        encode_inbox_messaging_delegation_id(&*held0, out);
    }
    json::member(out, "to");
    encode_inbox_messaging_address(&value.to, out);
    json::member(out, "kind");
    encode_inbox_messaging_kind(&value.kind, out);
    json::member(out, "thread");
    encode_inbox_messaging_thread_id(&value.thread, out);
    if let Some(held0) = &value.reply_shape {
        json::member(out, "reply_shape");
        encode_inbox_messaging_reply_shape(&*held0, out);
    }
    if let Some(held0) = &value.decision {
        json::member(out, "decision");
        encode_inbox_messaging_decision_ask(&*held0, out);
    }
    if let Some(held0) = &value.default_on_silence {
        json::member(out, "default_on_silence");
        encode_inbox_messaging_option_letter(&*held0, out);
    }
    json::member(out, "created");
    json::push_text(out, &value.created.0);
    json::member(out, "expires");
    json::push_text(out, &value.expires.0);
    json::member(out, "dedup_key");
    encode_inbox_messaging_dedup_key(&value.dedup_key, out);
    json::member(out, "body");
    encode_inbox_messaging_body(&value.body, out);
    json::member(out, "signature");
    encode_inbox_messaging_signature(&value.signature, out);
    if let Some(held0) = &value.answered_by {
        json::member(out, "answered_by");
        encode_inbox_messaging_answered_by(&*held0, out);
    }
    if let Some(held0) = &value.decided_option {
        json::member(out, "decided_option");
        encode_inbox_messaging_option_letter(&*held0, out);
    }
    if let Some(held0) = &value.reply {
        json::member(out, "reply");
        encode_inbox_messaging_reply(&*held0, out);
    }
    if let Some(held0) = &value.refusal {
        json::member(out, "refusal");
        json::push_text(out, &*held0);
    }
    json::member(out, "state");
    encode_inbox_messaging_message_state(&value.state, out);
    out.push('}');
}

/// Reads the parameters of `inbox.messaging.Settled` from an object keyed by their wire names.
///
/// # Errors
///
/// [`json::DecodeError`] naming the parameter and what the declaration says belongs there.
pub fn decode_params_inbox_messaging_settled(value: &json::Value, at: &str) -> Result<(crate::messaging::Principal,), json::DecodeError> {
    Ok((
        {
            let at0 = json::nested(at, "sender");
            let member0 = json::member_at(value, at, "sender")?;
            decode_inbox_messaging_principal(member0, &at0)?
        },
    ))
}

/// Writes one row of the view `inbox.messaging.Thread` as JSON.
pub fn encode_view_inbox_messaging_thread(value: &crate::messaging::Thread, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    json::member(out, "from");
    encode_inbox_messaging_principal(&value.from, out);
    json::member(out, "on_behalf_of");
    encode_inbox_messaging_principal(&value.on_behalf_of, out);
    if let Some(held0) = &value.authority {
        json::member(out, "authority");
        encode_inbox_messaging_delegation_id(&*held0, out);
    }
    json::member(out, "to");
    encode_inbox_messaging_address(&value.to, out);
    json::member(out, "kind");
    encode_inbox_messaging_kind(&value.kind, out);
    json::member(out, "thread");
    encode_inbox_messaging_thread_id(&value.thread, out);
    if let Some(held0) = &value.reply_shape {
        json::member(out, "reply_shape");
        encode_inbox_messaging_reply_shape(&*held0, out);
    }
    if let Some(held0) = &value.decision {
        json::member(out, "decision");
        encode_inbox_messaging_decision_ask(&*held0, out);
    }
    if let Some(held0) = &value.default_on_silence {
        json::member(out, "default_on_silence");
        encode_inbox_messaging_option_letter(&*held0, out);
    }
    json::member(out, "created");
    json::push_text(out, &value.created.0);
    json::member(out, "expires");
    json::push_text(out, &value.expires.0);
    json::member(out, "dedup_key");
    encode_inbox_messaging_dedup_key(&value.dedup_key, out);
    json::member(out, "body");
    encode_inbox_messaging_body(&value.body, out);
    json::member(out, "signature");
    encode_inbox_messaging_signature(&value.signature, out);
    if let Some(held0) = &value.answered_by {
        json::member(out, "answered_by");
        encode_inbox_messaging_answered_by(&*held0, out);
    }
    if let Some(held0) = &value.decided_option {
        json::member(out, "decided_option");
        encode_inbox_messaging_option_letter(&*held0, out);
    }
    if let Some(held0) = &value.reply {
        json::member(out, "reply");
        encode_inbox_messaging_reply(&*held0, out);
    }
    if let Some(held0) = &value.refusal {
        json::member(out, "refusal");
        json::push_text(out, &*held0);
    }
    json::member(out, "state");
    encode_inbox_messaging_message_state(&value.state, out);
    out.push('}');
}

/// Reads the parameters of `inbox.messaging.Thread` from an object keyed by their wire names.
///
/// # Errors
///
/// [`json::DecodeError`] naming the parameter and what the declaration says belongs there.
pub fn decode_params_inbox_messaging_thread(value: &json::Value, at: &str) -> Result<(crate::messaging::ThreadId,), json::DecodeError> {
    Ok((
        {
            let at0 = json::nested(at, "thread");
            let member0 = json::member_at(value, at, "thread")?;
            decode_inbox_messaging_thread_id(member0, &at0)?
        },
    ))
}

/// Writes one row of the view `inbox.messaging.Unanswered` as JSON.
pub fn encode_view_inbox_messaging_unanswered(value: &crate::messaging::Unanswered, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    json::member(out, "from");
    encode_inbox_messaging_principal(&value.from, out);
    json::member(out, "on_behalf_of");
    encode_inbox_messaging_principal(&value.on_behalf_of, out);
    if let Some(held0) = &value.authority {
        json::member(out, "authority");
        encode_inbox_messaging_delegation_id(&*held0, out);
    }
    json::member(out, "to");
    encode_inbox_messaging_address(&value.to, out);
    json::member(out, "kind");
    encode_inbox_messaging_kind(&value.kind, out);
    json::member(out, "thread");
    encode_inbox_messaging_thread_id(&value.thread, out);
    if let Some(held0) = &value.reply_shape {
        json::member(out, "reply_shape");
        encode_inbox_messaging_reply_shape(&*held0, out);
    }
    if let Some(held0) = &value.decision {
        json::member(out, "decision");
        encode_inbox_messaging_decision_ask(&*held0, out);
    }
    if let Some(held0) = &value.default_on_silence {
        json::member(out, "default_on_silence");
        encode_inbox_messaging_option_letter(&*held0, out);
    }
    json::member(out, "created");
    json::push_text(out, &value.created.0);
    json::member(out, "expires");
    json::push_text(out, &value.expires.0);
    json::member(out, "dedup_key");
    encode_inbox_messaging_dedup_key(&value.dedup_key, out);
    json::member(out, "body");
    encode_inbox_messaging_body(&value.body, out);
    json::member(out, "signature");
    encode_inbox_messaging_signature(&value.signature, out);
    json::member(out, "state");
    encode_inbox_messaging_message_state(&value.state, out);
    out.push('}');
}

/// Reads the parameters of `inbox.messaging.Unanswered` from an object keyed by their wire names.
///
/// # Errors
///
/// [`json::DecodeError`] naming the parameter and what the declaration says belongs there.
pub fn decode_params_inbox_messaging_unanswered(value: &json::Value, at: &str) -> Result<(crate::messaging::Address,), json::DecodeError> {
    Ok((
        {
            let at0 = json::nested(at, "inbox");
            let member0 = json::member_at(value, at, "inbox")?;
            decode_inbox_messaging_address(member0, &at0)?
        },
    ))
}

/// Writes the input of `inbox.messaging.Answer` as JSON.
pub fn encode_command_inbox_messaging_answer(value: &crate::messaging::Answer, out: &mut String) {
    out.push('{');
    json::member(out, "in_reply_to");
    encode_inbox_messaging_message_id(&value.in_reply_to, out);
    json::member(out, "reply_id");
    encode_inbox_messaging_message_id(&value.reply_id, out);
    json::member(out, "from");
    encode_inbox_messaging_principal(&value.from, out);
    json::member(out, "on_behalf_of");
    encode_inbox_messaging_principal(&value.on_behalf_of, out);
    if let Some(held0) = &value.authority {
        json::member(out, "authority");
        encode_inbox_messaging_delegation_id(&*held0, out);
    }
    json::member(out, "created");
    json::push_text(out, &value.created.0);
    json::member(out, "expires");
    json::push_text(out, &value.expires.0);
    if let Some(held0) = &value.option {
        json::member(out, "option");
        encode_inbox_messaging_option_letter(&*held0, out);
    }
    json::member(out, "body");
    encode_inbox_messaging_body(&value.body, out);
    json::member(out, "signature");
    encode_inbox_messaging_signature(&value.signature, out);
    out.push('}');
}

/// Reads the input of `inbox.messaging.Answer` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_command_inbox_messaging_answer(value: &json::Value, at: &str) -> Result<crate::messaging::Answer, json::DecodeError> {
    json::closed_at(value, at, &["in_reply_to", "reply_id", "from", "on_behalf_of", "authority", "created", "expires", "option", "body", "signature"])?;
    Ok(crate::messaging::Answer {
        in_reply_to: {
            let at0 = json::nested(at, "in_reply_to");
            let member0 = json::member_at(value, at, "in_reply_to")?;
            decode_inbox_messaging_message_id(member0, &at0)?
        },
        reply_id: {
            let at1 = json::nested(at, "reply_id");
            let member1 = json::member_at(value, at, "reply_id")?;
            decode_inbox_messaging_message_id(member1, &at1)?
        },
        from: {
            let at2 = json::nested(at, "from");
            let member2 = json::member_at(value, at, "from")?;
            decode_inbox_messaging_principal(member2, &at2)?
        },
        on_behalf_of: {
            let at3 = json::nested(at, "on_behalf_of");
            let member3 = json::member_at(value, at, "on_behalf_of")?;
            decode_inbox_messaging_principal(member3, &at3)?
        },
        authority: match value.member("authority") {
            None | Some(json::Value::Null) => None,
            Some(member4) => {
                let at4 = json::nested(at, "authority");
                Some(decode_inbox_messaging_delegation_id(member4, &at4)?)
            }
        },
        created: {
            let at5 = json::nested(at, "created");
            let member5 = json::member_at(value, at, "created")?;
            crate::primitives::Timestamp(json::timestamp_at(member5, &at5, "an RFC 3339 instant")?.to_owned())
        },
        expires: {
            let at6 = json::nested(at, "expires");
            let member6 = json::member_at(value, at, "expires")?;
            crate::primitives::Timestamp(json::timestamp_at(member6, &at6, "an RFC 3339 instant")?.to_owned())
        },
        option: match value.member("option") {
            None | Some(json::Value::Null) => None,
            Some(member7) => {
                let at7 = json::nested(at, "option");
                Some(decode_inbox_messaging_option_letter(member7, &at7)?)
            }
        },
        body: {
            let at8 = json::nested(at, "body");
            let member8 = json::member_at(value, at, "body")?;
            decode_inbox_messaging_body(member8, &at8)?
        },
        signature: {
            let at9 = json::nested(at, "signature");
            let member9 = json::member_at(value, at, "signature")?;
            decode_inbox_messaging_signature(member9, &at9)?
        },
    })
}

/// Writes the outcome of `inbox.messaging.Answer` as JSON: the branch taken, what it published, and the declared
/// refusal it carries where it carries one.
pub fn encode_outcome_inbox_messaging_answer(value: &crate::messaging::AnswerOutcome, out: &mut String) {
    out.push('{');
    match value {
        crate::messaging::AnswerOutcome::NotSender { error } => {
            json::member(out, "outcome");
            json::push_text(out, "not-sender");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.NotSender");
            json::member(out, "payload");
            encode_error_inbox_messaging_not_sender(error, out);
            out.push('}');
        }
        crate::messaging::AnswerOutcome::NoAuthority { error } => {
            json::member(out, "outcome");
            json::push_text(out, "no-authority");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.NoAuthority");
            json::member(out, "payload");
            encode_error_inbox_messaging_no_authority(error, out);
            out.push('}');
        }
        crate::messaging::AnswerOutcome::AlreadyExpired { error } => {
            json::member(out, "outcome");
            json::push_text(out, "already-expired");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.AlreadyExpired");
            json::member(out, "payload");
            encode_error_inbox_messaging_already_expired(error, out);
            out.push('}');
        }
        crate::messaging::AnswerOutcome::SignatureInvalid { error } => {
            json::member(out, "outcome");
            json::push_text(out, "signature-invalid");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.SignatureInvalid");
            json::member(out, "payload");
            encode_error_inbox_messaging_signature_invalid(error, out);
            out.push('}');
        }
        crate::messaging::AnswerOutcome::AuthorityInvalid { error } => {
            json::member(out, "outcome");
            json::push_text(out, "authority-invalid");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.NoAuthority");
            json::member(out, "payload");
            encode_error_inbox_messaging_no_authority(error, out);
            out.push('}');
        }
        crate::messaging::AnswerOutcome::UnknownMessage { error } => {
            json::member(out, "outcome");
            json::push_text(out, "unknown-message");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.UnknownMessage");
            json::member(out, "payload");
            encode_error_inbox_messaging_unknown_message(error, out);
            out.push('}');
        }
        crate::messaging::AnswerOutcome::ForeignMessage { error } => {
            json::member(out, "outcome");
            json::push_text(out, "foreign-message");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.UnknownMessage");
            json::member(out, "payload");
            encode_error_inbox_messaging_unknown_message(error, out);
            out.push('}');
        }
        crate::messaging::AnswerOutcome::NotRecipient { error } => {
            json::member(out, "outcome");
            json::push_text(out, "not-recipient");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.NotRecipient");
            json::member(out, "payload");
            encode_error_inbox_messaging_not_recipient(error, out);
            out.push('}');
        }
        crate::messaging::AnswerOutcome::AlreadySettled { error } => {
            json::member(out, "outcome");
            json::push_text(out, "already-settled");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.AlreadySettled");
            json::member(out, "payload");
            encode_error_inbox_messaging_already_settled(error, out);
            out.push('}');
        }
        crate::messaging::AnswerOutcome::Note { error } => {
            json::member(out, "outcome");
            json::push_text(out, "note");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.NotAnswerable");
            json::member(out, "payload");
            encode_error_inbox_messaging_not_answerable(error, out);
            out.push('}');
        }
        crate::messaging::AnswerOutcome::DeadlinePassed { error } => {
            json::member(out, "outcome");
            json::push_text(out, "deadline-passed");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.Expired");
            json::member(out, "payload");
            encode_error_inbox_messaging_expired(error, out);
            out.push('}');
        }
        crate::messaging::AnswerOutcome::ExpiredOnArrival { error } => {
            json::member(out, "outcome");
            json::push_text(out, "expired-on-arrival");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.AlreadyExpired");
            json::member(out, "payload");
            encode_error_inbox_messaging_already_expired(error, out);
            out.push('}');
        }
        crate::messaging::AnswerOutcome::ShapeMismatch { error } => {
            json::member(out, "outcome");
            json::push_text(out, "shape-mismatch");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.ShapeMismatch");
            json::member(out, "payload");
            encode_error_inbox_messaging_shape_mismatch(error, out);
            out.push('}');
        }
        crate::messaging::AnswerOutcome::Answered { message_answered } => {
            json::member(out, "outcome");
            json::push_text(out, "answered");
            json::member(out, "published");
            out.push('[');
            out.push('{');
            json::member(out, "event");
            json::push_text(out, "inbox.messaging.MessageAnswered");
            json::member(out, "payload");
            encode_event_inbox_messaging_message_answered(message_answered, out);
            out.push('}');
            out.push(']');
        }
    }
    out.push('}');
}

/// Writes the input of `inbox.messaging.CloseInbox` as JSON.
pub fn encode_command_inbox_messaging_close_inbox(value: &crate::messaging::CloseInbox, out: &mut String) {
    out.push('{');
    json::member(out, "address");
    encode_inbox_messaging_address(&value.address, out);
    out.push('}');
}

/// Reads the input of `inbox.messaging.CloseInbox` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_command_inbox_messaging_close_inbox(value: &json::Value, at: &str) -> Result<crate::messaging::CloseInbox, json::DecodeError> {
    json::closed_at(value, at, &["address"])?;
    Ok(crate::messaging::CloseInbox {
        address: {
            let at0 = json::nested(at, "address");
            let member0 = json::member_at(value, at, "address")?;
            decode_inbox_messaging_address(member0, &at0)?
        },
    })
}

/// Writes the outcome of `inbox.messaging.CloseInbox` as JSON: the branch taken, what it published, and the declared
/// refusal it carries where it carries one.
pub fn encode_outcome_inbox_messaging_close_inbox(value: &crate::messaging::CloseInboxOutcome, out: &mut String) {
    out.push('{');
    match value {
        crate::messaging::CloseInboxOutcome::UnknownInbox { error } => {
            json::member(out, "outcome");
            json::push_text(out, "unknown-inbox");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.UnknownInbox");
            json::member(out, "payload");
            encode_error_inbox_messaging_unknown_inbox(error, out);
            out.push('}');
        }
        crate::messaging::CloseInboxOutcome::NotOwner { error } => {
            json::member(out, "outcome");
            json::push_text(out, "not-owner");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.NotRecipient");
            json::member(out, "payload");
            encode_error_inbox_messaging_not_recipient(error, out);
            out.push('}');
        }
        crate::messaging::CloseInboxOutcome::Closed { inbox_closed } => {
            json::member(out, "outcome");
            json::push_text(out, "closed");
            json::member(out, "published");
            out.push('[');
            out.push('{');
            json::member(out, "event");
            json::push_text(out, "inbox.messaging.InboxClosed");
            json::member(out, "payload");
            encode_event_inbox_messaging_inbox_closed(inbox_closed, out);
            out.push('}');
            out.push(']');
        }
        crate::messaging::CloseInboxOutcome::AlreadyClosed => {
            json::member(out, "outcome");
            json::push_text(out, "already-closed");
            json::member(out, "published");
            out.push('[');
            out.push(']');
        }
    }
    out.push('}');
}

/// Writes the input of `inbox.messaging.Collect` as JSON.
pub fn encode_command_inbox_messaging_collect(value: &crate::messaging::Collect, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    out.push('}');
}

/// Reads the input of `inbox.messaging.Collect` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_command_inbox_messaging_collect(value: &json::Value, at: &str) -> Result<crate::messaging::Collect, json::DecodeError> {
    json::closed_at(value, at, &["message_id"])?;
    Ok(crate::messaging::Collect {
        message_id: {
            let at0 = json::nested(at, "message_id");
            let member0 = json::member_at(value, at, "message_id")?;
            decode_inbox_messaging_message_id(member0, &at0)?
        },
    })
}

/// Writes the outcome of `inbox.messaging.Collect` as JSON: the branch taken, what it published, and the declared
/// refusal it carries where it carries one.
pub fn encode_outcome_inbox_messaging_collect(value: &crate::messaging::CollectOutcome, out: &mut String) {
    out.push('{');
    match value {
        crate::messaging::CollectOutcome::UnknownMessage { error } => {
            json::member(out, "outcome");
            json::push_text(out, "unknown-message");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.UnknownMessage");
            json::member(out, "payload");
            encode_error_inbox_messaging_unknown_message(error, out);
            out.push('}');
        }
        crate::messaging::CollectOutcome::ForeignMessage { error } => {
            json::member(out, "outcome");
            json::push_text(out, "foreign-message");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.UnknownMessage");
            json::member(out, "payload");
            encode_error_inbox_messaging_unknown_message(error, out);
            out.push('}');
        }
        crate::messaging::CollectOutcome::NotSender { error } => {
            json::member(out, "outcome");
            json::push_text(out, "not-sender");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.NotSender");
            json::member(out, "payload");
            encode_error_inbox_messaging_not_sender(error, out);
            out.push('}');
        }
        crate::messaging::CollectOutcome::Collected { message_handled } => {
            json::member(out, "outcome");
            json::push_text(out, "collected");
            json::member(out, "published");
            out.push('[');
            out.push('{');
            json::member(out, "event");
            json::push_text(out, "inbox.messaging.MessageHandled");
            json::member(out, "payload");
            encode_event_inbox_messaging_message_handled(message_handled, out);
            out.push('}');
            out.push(']');
        }
        crate::messaging::CollectOutcome::NotSettled { error } => {
            json::member(out, "outcome");
            json::push_text(out, "not-settled");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.NotSettled");
            json::member(out, "payload");
            encode_error_inbox_messaging_not_settled(error, out);
            out.push('}');
        }
    }
    out.push('}');
}

/// Writes the input of `inbox.messaging.Expire` as JSON.
pub fn encode_command_inbox_messaging_expire(value: &crate::messaging::Expire, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    json::member(out, "now");
    json::push_text(out, &value.now.0);
    json::member(out, "keep_until");
    json::push_text(out, &value.keep_until.0);
    out.push('}');
}

/// Reads the input of `inbox.messaging.Expire` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_command_inbox_messaging_expire(value: &json::Value, at: &str) -> Result<crate::messaging::Expire, json::DecodeError> {
    json::closed_at(value, at, &["message_id", "now", "keep_until"])?;
    Ok(crate::messaging::Expire {
        message_id: {
            let at0 = json::nested(at, "message_id");
            let member0 = json::member_at(value, at, "message_id")?;
            decode_inbox_messaging_message_id(member0, &at0)?
        },
        now: {
            let at1 = json::nested(at, "now");
            let member1 = json::member_at(value, at, "now")?;
            crate::primitives::Timestamp(json::timestamp_at(member1, &at1, "an RFC 3339 instant")?.to_owned())
        },
        keep_until: {
            let at2 = json::nested(at, "keep_until");
            let member2 = json::member_at(value, at, "keep_until")?;
            crate::primitives::Timestamp(json::timestamp_at(member2, &at2, "an RFC 3339 instant")?.to_owned())
        },
    })
}

/// Writes the outcome of `inbox.messaging.Expire` as JSON: the branch taken, what it published, and the declared
/// refusal it carries where it carries one.
pub fn encode_outcome_inbox_messaging_expire(value: &crate::messaging::ExpireOutcome, out: &mut String) {
    out.push('{');
    match value {
        crate::messaging::ExpireOutcome::UnknownMessage { error } => {
            json::member(out, "outcome");
            json::push_text(out, "unknown-message");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.UnknownMessage");
            json::member(out, "payload");
            encode_error_inbox_messaging_unknown_message(error, out);
            out.push('}');
        }
        crate::messaging::ExpireOutcome::NotDue { error } => {
            json::member(out, "outcome");
            json::push_text(out, "not-due");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.NotDue");
            json::member(out, "payload");
            encode_error_inbox_messaging_not_due(error, out);
            out.push('}');
        }
        crate::messaging::ExpireOutcome::DecidedOnSilence { decision_taken_on_silence } => {
            json::member(out, "outcome");
            json::push_text(out, "decided-on-silence");
            json::member(out, "published");
            out.push('[');
            out.push('{');
            json::member(out, "event");
            json::push_text(out, "inbox.messaging.DecisionTakenOnSilence");
            json::member(out, "payload");
            encode_event_inbox_messaging_decision_taken_on_silence(decision_taken_on_silence, out);
            out.push('}');
            out.push(']');
        }
        crate::messaging::ExpireOutcome::Expired { message_expired } => {
            json::member(out, "outcome");
            json::push_text(out, "expired");
            json::member(out, "published");
            out.push('[');
            out.push('{');
            json::member(out, "event");
            json::push_text(out, "inbox.messaging.MessageExpired");
            json::member(out, "payload");
            encode_event_inbox_messaging_message_expired(message_expired, out);
            out.push('}');
            out.push(']');
        }
    }
    out.push('}');
}

/// Writes the input of `inbox.messaging.MarkDelivered` as JSON.
pub fn encode_command_inbox_messaging_mark_delivered(value: &crate::messaging::MarkDelivered, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    out.push('}');
}

/// Reads the input of `inbox.messaging.MarkDelivered` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_command_inbox_messaging_mark_delivered(value: &json::Value, at: &str) -> Result<crate::messaging::MarkDelivered, json::DecodeError> {
    json::closed_at(value, at, &["message_id"])?;
    Ok(crate::messaging::MarkDelivered {
        message_id: {
            let at0 = json::nested(at, "message_id");
            let member0 = json::member_at(value, at, "message_id")?;
            decode_inbox_messaging_message_id(member0, &at0)?
        },
    })
}

/// Writes the outcome of `inbox.messaging.MarkDelivered` as JSON: the branch taken, what it published, and the declared
/// refusal it carries where it carries one.
pub fn encode_outcome_inbox_messaging_mark_delivered(value: &crate::messaging::MarkDeliveredOutcome, out: &mut String) {
    out.push('{');
    match value {
        crate::messaging::MarkDeliveredOutcome::UnknownMessage { error } => {
            json::member(out, "outcome");
            json::push_text(out, "unknown-message");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.UnknownMessage");
            json::member(out, "payload");
            encode_error_inbox_messaging_unknown_message(error, out);
            out.push('}');
        }
        crate::messaging::MarkDeliveredOutcome::ForeignMessage { error } => {
            json::member(out, "outcome");
            json::push_text(out, "foreign-message");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.UnknownMessage");
            json::member(out, "payload");
            encode_error_inbox_messaging_unknown_message(error, out);
            out.push('}');
        }
        crate::messaging::MarkDeliveredOutcome::NotRecipient { error } => {
            json::member(out, "outcome");
            json::push_text(out, "not-recipient");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.NotRecipient");
            json::member(out, "payload");
            encode_error_inbox_messaging_not_recipient(error, out);
            out.push('}');
        }
        crate::messaging::MarkDeliveredOutcome::Delivered { message_delivered } => {
            json::member(out, "outcome");
            json::push_text(out, "delivered");
            json::member(out, "published");
            out.push('[');
            out.push('{');
            json::member(out, "event");
            json::push_text(out, "inbox.messaging.MessageDelivered");
            json::member(out, "payload");
            encode_event_inbox_messaging_message_delivered(message_delivered, out);
            out.push('}');
            out.push(']');
        }
        crate::messaging::MarkDeliveredOutcome::AlreadyDelivered => {
            json::member(out, "outcome");
            json::push_text(out, "already-delivered");
            json::member(out, "published");
            out.push('[');
            out.push(']');
        }
    }
    out.push('}');
}

/// Writes the input of `inbox.messaging.MarkRead` as JSON.
pub fn encode_command_inbox_messaging_mark_read(value: &crate::messaging::MarkRead, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    out.push('}');
}

/// Reads the input of `inbox.messaging.MarkRead` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_command_inbox_messaging_mark_read(value: &json::Value, at: &str) -> Result<crate::messaging::MarkRead, json::DecodeError> {
    json::closed_at(value, at, &["message_id"])?;
    Ok(crate::messaging::MarkRead {
        message_id: {
            let at0 = json::nested(at, "message_id");
            let member0 = json::member_at(value, at, "message_id")?;
            decode_inbox_messaging_message_id(member0, &at0)?
        },
    })
}

/// Writes the outcome of `inbox.messaging.MarkRead` as JSON: the branch taken, what it published, and the declared
/// refusal it carries where it carries one.
pub fn encode_outcome_inbox_messaging_mark_read(value: &crate::messaging::MarkReadOutcome, out: &mut String) {
    out.push('{');
    match value {
        crate::messaging::MarkReadOutcome::UnknownMessage { error } => {
            json::member(out, "outcome");
            json::push_text(out, "unknown-message");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.UnknownMessage");
            json::member(out, "payload");
            encode_error_inbox_messaging_unknown_message(error, out);
            out.push('}');
        }
        crate::messaging::MarkReadOutcome::ForeignMessage { error } => {
            json::member(out, "outcome");
            json::push_text(out, "foreign-message");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.UnknownMessage");
            json::member(out, "payload");
            encode_error_inbox_messaging_unknown_message(error, out);
            out.push('}');
        }
        crate::messaging::MarkReadOutcome::NotRecipient { error } => {
            json::member(out, "outcome");
            json::push_text(out, "not-recipient");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.NotRecipient");
            json::member(out, "payload");
            encode_error_inbox_messaging_not_recipient(error, out);
            out.push('}');
        }
        crate::messaging::MarkReadOutcome::Handled { message_handled } => {
            json::member(out, "outcome");
            json::push_text(out, "handled");
            json::member(out, "published");
            out.push('[');
            out.push('{');
            json::member(out, "event");
            json::push_text(out, "inbox.messaging.MessageHandled");
            json::member(out, "payload");
            encode_event_inbox_messaging_message_handled(message_handled, out);
            out.push('}');
            out.push(']');
        }
        crate::messaging::MarkReadOutcome::Read { message_read } => {
            json::member(out, "outcome");
            json::push_text(out, "read");
            json::member(out, "published");
            out.push('[');
            out.push('{');
            json::member(out, "event");
            json::push_text(out, "inbox.messaging.MessageRead");
            json::member(out, "payload");
            encode_event_inbox_messaging_message_read(message_read, out);
            out.push('}');
            out.push(']');
        }
        crate::messaging::MarkReadOutcome::AlreadyRead => {
            json::member(out, "outcome");
            json::push_text(out, "already-read");
            json::member(out, "published");
            out.push('[');
            out.push(']');
        }
    }
    out.push('}');
}

/// Writes the input of `inbox.messaging.OpenInbox` as JSON.
pub fn encode_command_inbox_messaging_open_inbox(value: &crate::messaging::OpenInbox, out: &mut String) {
    out.push('{');
    json::member(out, "address");
    encode_inbox_messaging_address(&value.address, out);
    json::member(out, "adapter");
    encode_inbox_messaging_adapter(&value.adapter, out);
    out.push('}');
}

/// Reads the input of `inbox.messaging.OpenInbox` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_command_inbox_messaging_open_inbox(value: &json::Value, at: &str) -> Result<crate::messaging::OpenInbox, json::DecodeError> {
    json::closed_at(value, at, &["address", "adapter"])?;
    Ok(crate::messaging::OpenInbox {
        address: {
            let at0 = json::nested(at, "address");
            let member0 = json::member_at(value, at, "address")?;
            decode_inbox_messaging_address(member0, &at0)?
        },
        adapter: {
            let at1 = json::nested(at, "adapter");
            let member1 = json::member_at(value, at, "adapter")?;
            decode_inbox_messaging_adapter(member1, &at1)?
        },
    })
}

/// Writes the outcome of `inbox.messaging.OpenInbox` as JSON: the branch taken, what it published, and the declared
/// refusal it carries where it carries one.
pub fn encode_outcome_inbox_messaging_open_inbox(value: &crate::messaging::OpenInboxOutcome, out: &mut String) {
    out.push('{');
    match value {
        crate::messaging::OpenInboxOutcome::NotOwner { error } => {
            json::member(out, "outcome");
            json::push_text(out, "not-owner");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.NotRecipient");
            json::member(out, "payload");
            encode_error_inbox_messaging_not_recipient(error, out);
            out.push('}');
        }
        crate::messaging::OpenInboxOutcome::Opened { inbox_opened } => {
            json::member(out, "outcome");
            json::push_text(out, "opened");
            json::member(out, "published");
            out.push('[');
            out.push('{');
            json::member(out, "event");
            json::push_text(out, "inbox.messaging.InboxOpened");
            json::member(out, "payload");
            encode_event_inbox_messaging_inbox_opened(inbox_opened, out);
            out.push('}');
            out.push(']');
        }
        crate::messaging::OpenInboxOutcome::Reopened { inbox_opened } => {
            json::member(out, "outcome");
            json::push_text(out, "reopened");
            json::member(out, "published");
            out.push('[');
            out.push('{');
            json::member(out, "event");
            json::push_text(out, "inbox.messaging.InboxOpened");
            json::member(out, "payload");
            encode_event_inbox_messaging_inbox_opened(inbox_opened, out);
            out.push('}');
            out.push(']');
        }
        crate::messaging::OpenInboxOutcome::AlreadyOpen => {
            json::member(out, "outcome");
            json::push_text(out, "already-open");
            json::member(out, "published");
            out.push('[');
            out.push(']');
        }
    }
    out.push('}');
}

/// Writes the input of `inbox.messaging.PostMessage` as JSON.
pub fn encode_command_inbox_messaging_post_message(value: &crate::messaging::PostMessage, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    json::member(out, "from");
    encode_inbox_messaging_principal(&value.from, out);
    json::member(out, "on_behalf_of");
    encode_inbox_messaging_principal(&value.on_behalf_of, out);
    if let Some(held0) = &value.authority {
        json::member(out, "authority");
        encode_inbox_messaging_delegation_id(&*held0, out);
    }
    json::member(out, "to");
    encode_inbox_messaging_address(&value.to, out);
    json::member(out, "kind");
    encode_inbox_messaging_kind(&value.kind, out);
    json::member(out, "thread");
    encode_inbox_messaging_thread_id(&value.thread, out);
    if let Some(held0) = &value.reply_shape {
        json::member(out, "reply_shape");
        encode_inbox_messaging_reply_shape(&*held0, out);
    }
    if let Some(held0) = &value.decision {
        json::member(out, "decision");
        encode_inbox_messaging_decision_ask(&*held0, out);
    }
    if let Some(held0) = &value.default_on_silence {
        json::member(out, "default_on_silence");
        encode_inbox_messaging_option_letter(&*held0, out);
    }
    json::member(out, "created");
    json::push_text(out, &value.created.0);
    json::member(out, "expires");
    json::push_text(out, &value.expires.0);
    json::member(out, "dedup_key");
    encode_inbox_messaging_dedup_key(&value.dedup_key, out);
    json::member(out, "body");
    encode_inbox_messaging_body(&value.body, out);
    json::member(out, "signature");
    encode_inbox_messaging_signature(&value.signature, out);
    out.push('}');
}

/// Reads the input of `inbox.messaging.PostMessage` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_command_inbox_messaging_post_message(value: &json::Value, at: &str) -> Result<crate::messaging::PostMessage, json::DecodeError> {
    json::closed_at(value, at, &["message_id", "from", "on_behalf_of", "authority", "to", "kind", "thread", "reply_shape", "decision", "default_on_silence", "created", "expires", "dedup_key", "body", "signature"])?;
    Ok(crate::messaging::PostMessage {
        message_id: {
            let at0 = json::nested(at, "message_id");
            let member0 = json::member_at(value, at, "message_id")?;
            decode_inbox_messaging_message_id(member0, &at0)?
        },
        from: {
            let at1 = json::nested(at, "from");
            let member1 = json::member_at(value, at, "from")?;
            decode_inbox_messaging_principal(member1, &at1)?
        },
        on_behalf_of: {
            let at2 = json::nested(at, "on_behalf_of");
            let member2 = json::member_at(value, at, "on_behalf_of")?;
            decode_inbox_messaging_principal(member2, &at2)?
        },
        authority: match value.member("authority") {
            None | Some(json::Value::Null) => None,
            Some(member3) => {
                let at3 = json::nested(at, "authority");
                Some(decode_inbox_messaging_delegation_id(member3, &at3)?)
            }
        },
        to: {
            let at4 = json::nested(at, "to");
            let member4 = json::member_at(value, at, "to")?;
            decode_inbox_messaging_address(member4, &at4)?
        },
        kind: {
            let at5 = json::nested(at, "kind");
            let member5 = json::member_at(value, at, "kind")?;
            decode_inbox_messaging_kind(member5, &at5)?
        },
        thread: {
            let at6 = json::nested(at, "thread");
            let member6 = json::member_at(value, at, "thread")?;
            decode_inbox_messaging_thread_id(member6, &at6)?
        },
        reply_shape: match value.member("reply_shape") {
            None | Some(json::Value::Null) => None,
            Some(member7) => {
                let at7 = json::nested(at, "reply_shape");
                Some(decode_inbox_messaging_reply_shape(member7, &at7)?)
            }
        },
        decision: match value.member("decision") {
            None | Some(json::Value::Null) => None,
            Some(member8) => {
                let at8 = json::nested(at, "decision");
                Some(decode_inbox_messaging_decision_ask(member8, &at8)?)
            }
        },
        default_on_silence: match value.member("default_on_silence") {
            None | Some(json::Value::Null) => None,
            Some(member9) => {
                let at9 = json::nested(at, "default_on_silence");
                Some(decode_inbox_messaging_option_letter(member9, &at9)?)
            }
        },
        created: {
            let at10 = json::nested(at, "created");
            let member10 = json::member_at(value, at, "created")?;
            crate::primitives::Timestamp(json::timestamp_at(member10, &at10, "an RFC 3339 instant")?.to_owned())
        },
        expires: {
            let at11 = json::nested(at, "expires");
            let member11 = json::member_at(value, at, "expires")?;
            crate::primitives::Timestamp(json::timestamp_at(member11, &at11, "an RFC 3339 instant")?.to_owned())
        },
        dedup_key: {
            let at12 = json::nested(at, "dedup_key");
            let member12 = json::member_at(value, at, "dedup_key")?;
            decode_inbox_messaging_dedup_key(member12, &at12)?
        },
        body: {
            let at13 = json::nested(at, "body");
            let member13 = json::member_at(value, at, "body")?;
            decode_inbox_messaging_body(member13, &at13)?
        },
        signature: {
            let at14 = json::nested(at, "signature");
            let member14 = json::member_at(value, at, "signature")?;
            decode_inbox_messaging_signature(member14, &at14)?
        },
    })
}

/// Writes the outcome of `inbox.messaging.PostMessage` as JSON: the branch taken, what it published, and the declared
/// refusal it carries where it carries one.
pub fn encode_outcome_inbox_messaging_post_message(value: &crate::messaging::PostMessageOutcome, out: &mut String) {
    out.push('{');
    match value {
        crate::messaging::PostMessageOutcome::NotSender { error } => {
            json::member(out, "outcome");
            json::push_text(out, "not-sender");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.NotSender");
            json::member(out, "payload");
            encode_error_inbox_messaging_not_sender(error, out);
            out.push('}');
        }
        crate::messaging::PostMessageOutcome::NoAuthority { error } => {
            json::member(out, "outcome");
            json::push_text(out, "no-authority");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.NoAuthority");
            json::member(out, "payload");
            encode_error_inbox_messaging_no_authority(error, out);
            out.push('}');
        }
        crate::messaging::PostMessageOutcome::AlreadyExpired { error } => {
            json::member(out, "outcome");
            json::push_text(out, "already-expired");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.AlreadyExpired");
            json::member(out, "payload");
            encode_error_inbox_messaging_already_expired(error, out);
            out.push('}');
        }
        crate::messaging::PostMessageOutcome::RequestWithoutShape { error } => {
            json::member(out, "outcome");
            json::push_text(out, "request-without-shape");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.ShapeMismatch");
            json::member(out, "payload");
            encode_error_inbox_messaging_shape_mismatch(error, out);
            out.push('}');
        }
        crate::messaging::PostMessageOutcome::DecisionWithoutOptions { error } => {
            json::member(out, "outcome");
            json::push_text(out, "decision-without-options");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.ShapeMismatch");
            json::member(out, "payload");
            encode_error_inbox_messaging_shape_mismatch(error, out);
            out.push('}');
        }
        crate::messaging::PostMessageOutcome::UnknownOption { error } => {
            json::member(out, "outcome");
            json::push_text(out, "unknown-option");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.ShapeMismatch");
            json::member(out, "payload");
            encode_error_inbox_messaging_shape_mismatch(error, out);
            out.push('}');
        }
        crate::messaging::PostMessageOutcome::ExpiredOnArrival { error } => {
            json::member(out, "outcome");
            json::push_text(out, "expired-on-arrival");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.AlreadyExpired");
            json::member(out, "payload");
            encode_error_inbox_messaging_already_expired(error, out);
            out.push('}');
        }
        crate::messaging::PostMessageOutcome::SignatureInvalid { error } => {
            json::member(out, "outcome");
            json::push_text(out, "signature-invalid");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.SignatureInvalid");
            json::member(out, "payload");
            encode_error_inbox_messaging_signature_invalid(error, out);
            out.push('}');
        }
        crate::messaging::PostMessageOutcome::AuthorityInvalid { error } => {
            json::member(out, "outcome");
            json::push_text(out, "authority-invalid");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.NoAuthority");
            json::member(out, "payload");
            encode_error_inbox_messaging_no_authority(error, out);
            out.push('}');
        }
        crate::messaging::PostMessageOutcome::Duplicate { error } => {
            json::member(out, "outcome");
            json::push_text(out, "duplicate");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.Duplicate");
            json::member(out, "payload");
            encode_error_inbox_messaging_duplicate(error, out);
            out.push('}');
        }
        crate::messaging::PostMessageOutcome::DuplicateKey { error } => {
            json::member(out, "outcome");
            json::push_text(out, "duplicate-key");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.Duplicate");
            json::member(out, "payload");
            encode_error_inbox_messaging_duplicate(error, out);
            out.push('}');
        }
        crate::messaging::PostMessageOutcome::UnknownInbox { error } => {
            json::member(out, "outcome");
            json::push_text(out, "unknown-inbox");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.UnknownInbox");
            json::member(out, "payload");
            encode_error_inbox_messaging_unknown_inbox(error, out);
            out.push('}');
        }
        crate::messaging::PostMessageOutcome::InboxClosed { error } => {
            json::member(out, "outcome");
            json::push_text(out, "inbox-closed");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.UnknownInbox");
            json::member(out, "payload");
            encode_error_inbox_messaging_unknown_inbox(error, out);
            out.push('}');
        }
        crate::messaging::PostMessageOutcome::IdTaken { error } => {
            json::member(out, "outcome");
            json::push_text(out, "id-taken");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.MessageIdTaken");
            json::member(out, "payload");
            encode_error_inbox_messaging_message_id_taken(error, out);
            out.push('}');
        }
        crate::messaging::PostMessageOutcome::Posted { message_posted } => {
            json::member(out, "outcome");
            json::push_text(out, "posted");
            json::member(out, "published");
            out.push('[');
            out.push('{');
            json::member(out, "event");
            json::push_text(out, "inbox.messaging.MessagePosted");
            json::member(out, "payload");
            encode_event_inbox_messaging_message_posted(message_posted, out);
            out.push('}');
            out.push(']');
        }
    }
    out.push('}');
}

/// Writes the input of `inbox.messaging.Refuse` as JSON.
pub fn encode_command_inbox_messaging_refuse(value: &crate::messaging::Refuse, out: &mut String) {
    out.push('{');
    json::member(out, "message_id");
    encode_inbox_messaging_message_id(&value.message_id, out);
    json::member(out, "reason");
    json::push_text(out, &value.reason);
    out.push('}');
}

/// Reads the input of `inbox.messaging.Refuse` from JSON.
///
/// # Errors
///
/// [`json::DecodeError`] naming the path and what the declaration says belongs there.
pub fn decode_command_inbox_messaging_refuse(value: &json::Value, at: &str) -> Result<crate::messaging::Refuse, json::DecodeError> {
    json::closed_at(value, at, &["message_id", "reason"])?;
    Ok(crate::messaging::Refuse {
        message_id: {
            let at0 = json::nested(at, "message_id");
            let member0 = json::member_at(value, at, "message_id")?;
            decode_inbox_messaging_message_id(member0, &at0)?
        },
        reason: {
            let at1 = json::nested(at, "reason");
            let member1 = json::member_at(value, at, "reason")?;
            json::text_at(member1, &at1, "a string")?.to_owned()
        },
    })
}

/// Writes the outcome of `inbox.messaging.Refuse` as JSON: the branch taken, what it published, and the declared
/// refusal it carries where it carries one.
pub fn encode_outcome_inbox_messaging_refuse(value: &crate::messaging::RefuseOutcome, out: &mut String) {
    out.push('{');
    match value {
        crate::messaging::RefuseOutcome::UnknownMessage { error } => {
            json::member(out, "outcome");
            json::push_text(out, "unknown-message");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.UnknownMessage");
            json::member(out, "payload");
            encode_error_inbox_messaging_unknown_message(error, out);
            out.push('}');
        }
        crate::messaging::RefuseOutcome::ForeignMessage { error } => {
            json::member(out, "outcome");
            json::push_text(out, "foreign-message");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.UnknownMessage");
            json::member(out, "payload");
            encode_error_inbox_messaging_unknown_message(error, out);
            out.push('}');
        }
        crate::messaging::RefuseOutcome::NotRecipient { error } => {
            json::member(out, "outcome");
            json::push_text(out, "not-recipient");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.NotRecipient");
            json::member(out, "payload");
            encode_error_inbox_messaging_not_recipient(error, out);
            out.push('}');
        }
        crate::messaging::RefuseOutcome::Note { error } => {
            json::member(out, "outcome");
            json::push_text(out, "note");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.NotAnswerable");
            json::member(out, "payload");
            encode_error_inbox_messaging_not_answerable(error, out);
            out.push('}');
        }
        crate::messaging::RefuseOutcome::DeadlinePassed { error } => {
            json::member(out, "outcome");
            json::push_text(out, "deadline-passed");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.Expired");
            json::member(out, "payload");
            encode_error_inbox_messaging_expired(error, out);
            out.push('}');
        }
        crate::messaging::RefuseOutcome::Refused { message_refused } => {
            json::member(out, "outcome");
            json::push_text(out, "refused");
            json::member(out, "published");
            out.push('[');
            out.push('{');
            json::member(out, "event");
            json::push_text(out, "inbox.messaging.MessageRefused");
            json::member(out, "payload");
            encode_event_inbox_messaging_message_refused(message_refused, out);
            out.push('}');
            out.push(']');
        }
        crate::messaging::RefuseOutcome::AlreadySettled { error } => {
            json::member(out, "outcome");
            json::push_text(out, "already-settled");
            json::member(out, "published");
            out.push('[');
            out.push(']');
            json::member(out, "refusal");
            out.push('{');
            json::member(out, "error");
            json::push_text(out, "inbox.messaging.AlreadySettled");
            json::member(out, "payload");
            encode_error_inbox_messaging_already_settled(error, out);
            out.push('}');
        }
    }
    out.push('}');
}

/// Writes any event on the system's log as JSON: its qualified name and its payload,
/// `{"event": …, "payload": {…}}`, the envelope a command's answer lists it in.
pub fn encode_system_event(value: &crate::system::SystemEvent) -> String {
    let mut out = String::from("{");
    json::member(&mut out, "event");
    json::push_text(&mut out, value.name());
    json::member(&mut out, "payload");
    match value {
        crate::system::SystemEvent::DecisionTakenOnSilence(event) => encode_event_inbox_messaging_decision_taken_on_silence(event, &mut out),
        crate::system::SystemEvent::InboxClosed(event) => encode_event_inbox_messaging_inbox_closed(event, &mut out),
        crate::system::SystemEvent::InboxOpened(event) => encode_event_inbox_messaging_inbox_opened(event, &mut out),
        crate::system::SystemEvent::MessageAnswered(event) => encode_event_inbox_messaging_message_answered(event, &mut out),
        crate::system::SystemEvent::MessageDelivered(event) => encode_event_inbox_messaging_message_delivered(event, &mut out),
        crate::system::SystemEvent::MessageExpired(event) => encode_event_inbox_messaging_message_expired(event, &mut out),
        crate::system::SystemEvent::MessageHandled(event) => encode_event_inbox_messaging_message_handled(event, &mut out),
        crate::system::SystemEvent::MessagePosted(event) => encode_event_inbox_messaging_message_posted(event, &mut out),
        crate::system::SystemEvent::MessageRead(event) => encode_event_inbox_messaging_message_read(event, &mut out),
        crate::system::SystemEvent::MessageRefused(event) => encode_event_inbox_messaging_message_refused(event, &mut out),
    }
    out.push('}');
    out
}

/// Reads any event on the system's log back from the envelope [`encode_system_event`] writes,
/// `{"event": …, "payload": {…}}`: its qualified name selects the event, and its payload is read
/// under the wire names that event declares.
///
/// # Errors
///
/// [`json::DecodeError`] at `event` when the envelope names no event this system publishes,
/// naming the one it carried, and at the payload's own path when the payload is not that
/// event's.
pub fn decode_system_event(value: &json::Value) -> Result<crate::system::SystemEvent, json::DecodeError> {
    let event = json::text_at(json::member_at(value, "", "event")?, "event", "a qualified event name")?;
    Ok(match event {
        "inbox.messaging.DecisionTakenOnSilence" => crate::system::SystemEvent::DecisionTakenOnSilence(decode_event_inbox_messaging_decision_taken_on_silence(json::member_at(value, "", "payload")?, "payload")?),
        "inbox.messaging.InboxClosed" => crate::system::SystemEvent::InboxClosed(decode_event_inbox_messaging_inbox_closed(json::member_at(value, "", "payload")?, "payload")?),
        "inbox.messaging.InboxOpened" => crate::system::SystemEvent::InboxOpened(decode_event_inbox_messaging_inbox_opened(json::member_at(value, "", "payload")?, "payload")?),
        "inbox.messaging.MessageAnswered" => crate::system::SystemEvent::MessageAnswered(decode_event_inbox_messaging_message_answered(json::member_at(value, "", "payload")?, "payload")?),
        "inbox.messaging.MessageDelivered" => crate::system::SystemEvent::MessageDelivered(decode_event_inbox_messaging_message_delivered(json::member_at(value, "", "payload")?, "payload")?),
        "inbox.messaging.MessageExpired" => crate::system::SystemEvent::MessageExpired(decode_event_inbox_messaging_message_expired(json::member_at(value, "", "payload")?, "payload")?),
        "inbox.messaging.MessageHandled" => crate::system::SystemEvent::MessageHandled(decode_event_inbox_messaging_message_handled(json::member_at(value, "", "payload")?, "payload")?),
        "inbox.messaging.MessagePosted" => crate::system::SystemEvent::MessagePosted(decode_event_inbox_messaging_message_posted(json::member_at(value, "", "payload")?, "payload")?),
        "inbox.messaging.MessageRead" => crate::system::SystemEvent::MessageRead(decode_event_inbox_messaging_message_read(json::member_at(value, "", "payload")?, "payload")?),
        "inbox.messaging.MessageRefused" => crate::system::SystemEvent::MessageRefused(decode_event_inbox_messaging_message_refused(json::member_at(value, "", "payload")?, "payload")?),
        other => {
            return Err(json::DecodeError { at: "event".to_owned(), expected: "one of `inbox.messaging.DecisionTakenOnSilence`, `inbox.messaging.InboxClosed`, `inbox.messaging.InboxOpened`, `inbox.messaging.MessageAnswered`, `inbox.messaging.MessageDelivered`, `inbox.messaging.MessageExpired`, `inbox.messaging.MessageHandled`, `inbox.messaging.MessagePosted`, `inbox.messaging.MessageRead`, `inbox.messaging.MessageRefused`".to_owned(), found: format!("`{other}`") })
        }
    })
}
