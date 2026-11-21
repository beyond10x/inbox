//! The check for a member name carried twice by one JSON object.
//!
//! The generated reader keeps every member as it arrived, duplicates included, and the generated
//! decoders take the first. A text with a duplicate member therefore decodes, and one signature
//! covers two texts that differ to a reader that keeps the last. A receiver refuses such a text
//! with [`duplicate_member`] before it decodes it.

use std::collections::HashSet;

use inbox::server::json::{self, Value};

/// The path of the first member name an object of `value` carries twice, at any depth; `None`
/// when every object's member names are distinct.
///
/// The path is dotted, as the generated decoders write one (`keys.0.principal`; an array item
/// by its index). An object's own member names are checked before the values inside it;
/// otherwise the walk is in document order. It keeps its own list rather than recursing, so a
/// value of any depth is walked.
#[must_use]
pub fn duplicate_member(value: &Value) -> Option<String> {
    let mut pending: Vec<(&Value, String)> = vec![(value, String::new())];
    while let Some((value, at)) = pending.pop() {
        match value {
            Value::Object(members) => {
                let mut seen = HashSet::with_capacity(members.len());
                if let Some((name, _)) = members.iter().find(|(name, _)| !seen.insert(name)) {
                    return Some(json::nested(&at, name));
                }
                pending.extend(
                    members
                        .iter()
                        .rev()
                        .map(|(name, item)| (item, json::nested(&at, name))),
                );
            }
            Value::Array(items) => pending.extend(
                items
                    .iter()
                    .enumerate()
                    .rev()
                    .map(|(index, item)| (item, json::nested(&at, &index.to_string()))),
            ),
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::Text(_) => {}
        }
    }
    None
}
