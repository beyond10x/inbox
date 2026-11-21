//! JSON values as the suite compares them: rendered for a message, and compared as a view
//! assertion or as an event payload reads them.

use inbox::server::json::{self, Value};

/// The compact JSON text of `value`, for a message and for the results document.
pub fn render(value: &Value) -> String {
    let mut out = String::new();
    write(value, &mut out);
    out
}

fn write(value: &Value, out: &mut String) {
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
            for (name, item) in members {
                json::member(out, name);
                write(item, out);
            }
            out.push('}');
        }
    }
}

/// `render`, cut to `most` characters with a marker, so a message stays readable.
pub fn brief(value: &Value, most: usize) -> String {
    let text = render(value);
    if text.chars().count() <= most {
        return text;
    }
    let cut: String = text.chars().take(most).collect();
    format!("{cut}…")
}

/// The binary64 a JSON number denotes, as the Rust runner of ESS compares numbers.
fn number(spelling: &str) -> Option<f64> {
    spelling
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn same_number(left: &str, right: &str) -> bool {
    match (number(left), number(right)) {
        (Some(left), Some(right)) => left == right,
        _ => left == right,
    }
}

/// Whether `actual` holds what a view assertion claims: a member that is absent and one that is
/// `null` are one value at any depth, a nested value is compared whole, and numbers by their
/// binary64 value.
pub fn view_equal(claimed: Option<&Value>, actual: Option<&Value>) -> bool {
    let claimed = claimed.filter(|value| **value != Value::Null);
    let actual = actual.filter(|value| **value != Value::Null);
    match (claimed, actual) {
        (None, None) => true,
        (Some(Value::Object(left)), Some(Value::Object(right))) => left
            .iter()
            .map(|(name, _)| name)
            .chain(right.iter().map(|(name, _)| name))
            .all(|name| view_equal(member(left, name), member(right, name))),
        (Some(Value::Array(left)), Some(Value::Array(right))) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| view_equal(Some(left), Some(right)))
        }
        (Some(Value::Number(left)), Some(Value::Number(right))) => same_number(left, right),
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

/// Whether two event payloads are one: the same members, no `null` read as absent, numbers by
/// their binary64 value.
pub fn exact_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(name, value)| {
                    member(right, name).is_some_and(|other| exact_equal(value, other))
                })
        }
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| exact_equal(left, right))
        }
        (Value::Number(left), Value::Number(right)) => same_number(left, right),
        _ => left == right,
    }
}

fn member<'a>(members: &'a [(String, Value)], name: &str) -> Option<&'a Value> {
    members
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> Value {
        Value::Text(value.to_owned())
    }

    fn object(members: &[(&str, Value)]) -> Value {
        Value::Object(
            members
                .iter()
                .map(|(name, value)| ((*name).to_owned(), value.clone()))
                .collect(),
        )
    }

    #[test]
    fn a_view_reads_absent_and_null_as_one_value_at_any_depth() {
        let claimed = object(&[("a", text("x")), ("b", Value::Null)]);
        let actual = object(&[("a", text("x"))]);
        assert!(view_equal(Some(&claimed), Some(&actual)));
        assert!(view_equal(Some(&Value::Null), None));
        assert!(!view_equal(Some(&text("x")), None));
        let nested = object(&[("a", text("x")), ("b", text("y"))]);
        assert!(!view_equal(Some(&claimed), Some(&nested)));
    }

    #[test]
    fn numbers_compare_by_value() {
        let three = Value::Number("3".to_owned());
        let spelled = Value::Number("3.0".to_owned());
        assert!(view_equal(Some(&three), Some(&spelled)));
        assert!(exact_equal(&three, &spelled));
        assert!(!exact_equal(&three, &Value::Number("4".to_owned())));
    }

    #[test]
    fn an_event_payload_is_exact() {
        let claimed = object(&[("a", text("x"))]);
        let more = object(&[("a", text("x")), ("b", Value::Null)]);
        assert!(!exact_equal(&claimed, &more));
        assert!(exact_equal(&claimed, &claimed.clone()));
    }

    #[test]
    fn render_writes_compact_json() {
        let value = object(&[("a", Value::Array(vec![text("x\""), Value::Bool(true)]))]);
        assert_eq!(render(&value), r#"{"a":["x\"",true]}"#);
    }
}
