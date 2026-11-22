//! Adversary pass 2 on story:a-due-message-takes-no-mark: `inbox_core::due`, public since the
//! wait on `Pending` asks it, against its documentation: `Some(true)` where `expires` is at or
//! before `now` as instants, `Some(false)` after, `None` where either names no instant.

use inbox::primitives::Timestamp;
use inbox_core::due;

fn at(text: &str) -> Timestamp {
    Timestamp(text.to_owned())
}

#[test]
fn due_answers_as_its_documentation_says_for_instants_and_for_text_that_names_none() {
    let now = at("2026-12-01T00:00:00Z");
    for (expires, expected) in [
        ("2026-12-01T00:00:00Z", Some(true)),
        ("2026-12-01T00:00:00.000Z", Some(true)),
        ("2026-12-01T01:00:00+01:00", Some(true)),
        ("2026-11-30t23:59:59.999999999z", Some(true)),
        ("2026-12-01T00:00:00.000000001Z", Some(false)),
        ("2026-11-30T23:00:00-01:00", Some(true)),
        ("2026-11-30T23:00:00.5-01:00", Some(false)),
        ("", None),
        ("tomorrow", None),
        ("2026-02-30T00:00:00Z", None),
        ("2026-12-01T00:00:00", None),
        ("2026-12-01 00:00:00Z", None),
    ] {
        assert_eq!(due(&at(expires), &now), expected, "expires {expires:?}");
    }
    assert_eq!(due(&at("2026-12-01T00:00:00Z"), &at("")), None, "now empty");
    assert_eq!(due(&at(""), &at("")), None, "both empty");
}
