//! The check for a member name carried twice by one JSON object.
//!
//! The generated reader keeps every member as it arrived, duplicates included, and the generated
//! decoders take the first. A text with a duplicate member therefore decodes, and one signature
//! covers two texts that differ to a reader that keeps the last. A receiver refuses such a text
//! with [`duplicate_member`] before it decodes it.

use std::collections::HashSet;

use inbox::server::json::Value;

/// One step from a value to a child of it.
#[derive(Clone, Copy)]
enum Step<'a> {
    /// The member of an object with this name.
    Member(&'a str),
    /// The item of an array at this index.
    Item(usize),
}

/// The path of the first member name an object of `value` carries twice, at any depth; `None`
/// when every object's member names are distinct.
///
/// The path is dotted, as the generated decoders write one (`keys.0.principal`; an array item
/// by its index). An object's own member names are checked before the values inside it;
/// otherwise the walk is in document order.
///
/// Time and memory are linear in the size of the value, so a listener can run it on a caller's
/// body before authentication: the walk keeps its own list rather than recursing, each pending
/// value carries only its depth and the one step that reaches it, the path is joined only when a
/// duplicate is found, and the names of one object are compared through a hash set.
#[must_use]
pub fn duplicate_member(value: &Value) -> Option<String> {
    let mut pending: Vec<(&Value, usize, Option<Step<'_>>)> = vec![(value, 0, None)];
    let mut path: Vec<Step<'_>> = Vec::new();
    while let Some((value, depth, step)) = pending.pop() {
        path.truncate(depth.saturating_sub(1));
        if let Some(step) = step {
            path.push(step);
        }
        match value {
            Value::Object(members) => {
                let mut seen = HashSet::with_capacity(members.len());
                if let Some((name, _)) =
                    members.iter().find(|(name, _)| !seen.insert(name.as_str()))
                {
                    return Some(joined(&path, name));
                }
                pending.extend(
                    members
                        .iter()
                        .rev()
                        .map(|(name, item)| (item, depth + 1, Some(Step::Member(name)))),
                );
            }
            Value::Array(items) => pending.extend(
                items
                    .iter()
                    .enumerate()
                    .rev()
                    .map(|(index, item)| (item, depth + 1, Some(Step::Item(index)))),
            ),
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::Text(_) => {}
        }
    }
    None
}

/// The dotted path of `steps` and then `name`, joined as the generated decoders join one
/// (`json::nested`: a step after an empty path stands alone, every other step follows a `.`).
fn joined(steps: &[Step<'_>], name: &str) -> String {
    let mut at = String::new();
    let mut push = |segment: &str| {
        if !at.is_empty() {
            at.push('.');
        }
        at.push_str(segment);
    };
    for step in steps {
        match step {
            Step::Member(member) => push(member),
            Step::Item(index) => push(&index.to_string()),
        }
    }
    push(name);
    at
}

#[cfg(test)]
mod tests {
    use inbox::server::json;

    use super::duplicate_member;

    /// The path the generated decoders would write: `json::nested` folded over the steps.
    fn nested(steps: &[&str]) -> String {
        steps
            .iter()
            .fold(String::new(), |at, step| json::nested(&at, step))
    }

    #[test]
    fn the_path_is_spelled_as_json_nested_spells_it_with_empty_names_too() {
        let cases: [(&str, &[&str]); 5] = [
            (r#"{"":1,"":2}"#, &[""]),
            (r#"{"":{"":{"a":1,"a":2}}}"#, &["", "", "a"]),
            (r#"{"x":{"":{"a":1,"a":2}}}"#, &["x", "", "a"]),
            (r#"{"x":[[{"":1,"":2}]]}"#, &["x", "0", "0", ""]),
            (r#"[{},{"k.1":{"k":1,"k":2}}]"#, &["1", "k.1", "k"]),
        ];
        for (text, steps) in cases {
            let value = json::parse(text).unwrap();
            assert_eq!(duplicate_member(&value), Some(nested(steps)), "{text}");
        }
    }
}
