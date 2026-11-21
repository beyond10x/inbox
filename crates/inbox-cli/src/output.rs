//! What a command prints: one JSON document with `--json`, text for a person without.

use std::fmt::Write as _;

use inbox::server::json::{self, Value};

/// What a finished command prints.
#[derive(Debug)]
pub struct Done {
    /// The JSON document, printed with `--json`.
    pub document: String,
    /// The text, printed without it.
    pub text: String,
}

/// Appends `value` as JSON. Numbers keep the spelling they arrived in.
pub fn write(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(flag) => json::push_bool(out, *flag),
        Value::Number(spelling) => out.push_str(spelling),
        Value::Text(text) => json::push_text(out, text),
        Value::Array(items) => {
            out.push('[');
            for (at, item) in items.iter().enumerate() {
                if at > 0 {
                    out.push(',');
                }
                write(item, out);
            }
            out.push(']');
        }
        Value::Object(members) => {
            out.push('{');
            for (name, member) in members {
                json::member(out, name);
                write(member, out);
            }
            out.push('}');
        }
    }
}

/// `value` as a JSON document.
pub fn document(value: &Value) -> String {
    let mut out = String::new();
    write(value, &mut out);
    out
}

/// The text member `name` of `value`, or an empty text.
pub fn text<'a>(value: &'a Value, name: &str) -> &'a str {
    match value.member(name) {
        Some(Value::Text(text)) => text,
        _ => "",
    }
}

/// The rows of a view answer.
pub fn rows(value: &Value) -> &[Value] {
    match value.member("rows") {
        Some(Value::Array(rows)) => rows,
        _ => &[],
    }
}

/// The rows of a view answer, for a person: a head line per message, its body indented under
/// it, then what the message carries beyond the body (the options of a decision, its reply, its
/// refusal).
pub fn rows_text(answer: &Value) -> String {
    let rows = rows(answer);
    if rows.is_empty() {
        return "no messages".to_owned();
    }
    let mut out = String::new();
    for row in rows {
        let _ = write!(
            out,
            "{} {} {} from {} to {} thread {} expires {}",
            text(row, "message_id"),
            text(row, "kind"),
            text(row, "state"),
            text(row, "from"),
            text(row, "to"),
            text(row, "thread"),
            text(row, "expires"),
        );
        if text(row, "on_behalf_of") != text(row, "from") {
            let _ = write!(out, " on behalf of {}", text(row, "on_behalf_of"));
        }
        out.push('\n');
        indent(&mut out, text(row, "body"));
        if let Some(Value::Object(_)) = row.member("decision") {
            let decision = row.member("decision").unwrap_or(&Value::Null);
            if let Some(Value::Array(options)) = decision.member("options") {
                for option in options {
                    let _ = writeln!(
                        out,
                        "  {}) {}",
                        text(option, "letter"),
                        text(option, "text")
                    );
                }
            }
        }
        if let Some(reply) = row.member("reply") {
            let option = text(reply, "option");
            let _ = writeln!(
                out,
                "  reply {} from {}{}:",
                text(reply, "reply_id"),
                text(reply, "from"),
                if option.is_empty() {
                    String::new()
                } else {
                    format!(", option {option}")
                }
            );
            indent(&mut out, text(reply, "body"));
        } else if !text(row, "decided_option").is_empty() {
            let _ = writeln!(
                out,
                "  decided on silence: option {}",
                text(row, "decided_option")
            );
        }
        if !text(row, "refusal").is_empty() {
            let _ = writeln!(out, "  refused: {}", text(row, "refusal"));
        }
    }
    out.trim_end().to_owned()
}

fn indent(out: &mut String, body: &str) {
    for line in body.lines() {
        let _ = writeln!(out, "    {line}");
    }
}

#[cfg(test)]
mod tests {
    use inbox::server::json;

    use super::document;

    #[test]
    fn a_document_is_written_as_it_was_read() {
        let text = r#"{"rows":[{"a":"x\"y\n","n":1.50,"b":true,"c":null,"d":[]}],"e":{}}"#;
        assert_eq!(document(&json::parse(text).unwrap()), text);
    }
}
