//! Adversary, pass 2: cases against the correction of pass 1. Every key is created at run time.

mod common;

use common::{AGENT, LATER, generate, instant, now, ring, signed_key_entry};
use inbox::messaging::{KeyFile, Principal};
use inbox_verify::{EntryKind, IgnoreReason, Ignored, KeyRing, LoadError, Refusal};

/// The peak resident size of this process, in KiB (Linux).
fn peak_resident_kib() -> u64 {
    let status =
        std::fs::read_to_string("/proc/self/status").expect("Linux exposes /proc/self/status");
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|rest| rest.trim().trim_end_matches("kB").trim().parse().ok())
        .expect("/proc/self/status carries VmHWM")
}

/// `duplicate_member` builds the full dotted path of every value before it visits it, and keeps
/// all paths of one object's or array's children at once. One long member name above a long
/// array makes it hold (name length x item count) bytes: a 32 KiB key file costs more than
/// 128 MiB, and the check runs in `KeyRing::parse` before the decoder would refuse the member.
#[test]
fn reading_a_small_key_file_does_not_allocate_a_thousand_times_its_size() {
    let name = "n".repeat(16 * 1024);
    let items = vec!["0"; 8 * 1024].join(",");
    let text = format!("{{\"keys\":[],\"delegations\":[],\"{name}\":[{items}]}}");
    let operator = generate();
    let public = operator.public_key();
    let before = peak_resident_kib();
    let result = KeyRing::parse(&text, &public);
    let grown = peak_resident_kib().saturating_sub(before);
    assert!(
        matches!(result, Err(LoadError::Decode(_))),
        "the undeclared member is refused by the decoder"
    );
    let bound = u64::try_from(text.len()).unwrap() * 1000 / 1024;
    assert!(
        grown < bound,
        "a key file of {} bytes raised the peak resident size by {grown} KiB (bound {bound} KiB)",
        text.len()
    );
}

/// Renewing a key under its id (a later `valid_until`) leaves the earlier entry validly signed.
/// Putting it back into the file needs no operator key, and step 2 then ignores both: the id
/// answers `UnknownKey`. The verifier fails closed; `docs/design/signing.md` therefore rules
/// that an id is never reused, and a renewal takes a new id (the case below).
#[test]
fn an_earlier_signed_entry_put_back_beside_a_renewal_under_the_same_id_makes_both_ignored() {
    let operator = generate();
    let agent = generate();
    let earlier = signed_key_entry(
        "agent-key",
        AGENT,
        &agent,
        "2026-11-01T00:00:00Z",
        &operator,
    );
    let renewed = signed_key_entry("agent-key", AGENT, &agent, LATER, &operator);
    let agent_principal = Principal(AGENT.to_owned());

    let current = KeyFile {
        keys: vec![renewed.clone()],
        delegations: vec![],
    };
    assert_eq!(
        ring(&current, &operator).principal("agent-key", now()),
        Ok(&agent_principal)
    );
    let earlier_alone = KeyFile {
        keys: vec![earlier.clone()],
        delegations: vec![],
    };
    assert_eq!(
        ring(&earlier_alone, &operator).principal("agent-key", now()),
        Err(Refusal::KeyLapsed),
        "the earlier entry is lapsed on its own"
    );

    let replayed = KeyFile {
        keys: vec![renewed, earlier],
        delegations: vec![],
    };
    let ring = ring(&replayed, &operator);
    assert_eq!(
        ring.ignored(),
        &[
            Ignored {
                kind: EntryKind::Key,
                id: "agent-key".to_owned(),
                reason: IgnoreReason::AmbiguousId,
            },
            Ignored {
                kind: EntryKind::Key,
                id: "agent-key".to_owned(),
                reason: IgnoreReason::AmbiguousId,
            },
        ]
    );
    assert_eq!(ring.principal("agent-key", now()), Err(Refusal::UnknownKey));
}

/// The rule of `docs/design/signing.md`: a renewal takes a new id. Putting the earlier entry back
/// beside it then leaves the renewal live, and the earlier id lapses by its own `valid_until`.
#[test]
fn a_renewal_under_a_new_id_stays_live_when_the_earlier_entry_is_put_back() {
    let operator = generate();
    let agent = generate();
    let earlier = signed_key_entry(
        "agent-key-1",
        AGENT,
        &agent,
        "2026-11-01T00:00:00Z",
        &operator,
    );
    let renewed = signed_key_entry("agent-key-2", AGENT, &agent, LATER, &operator);
    let agent_principal = Principal(AGENT.to_owned());

    let replayed = KeyFile {
        keys: vec![renewed, earlier],
        delegations: vec![],
    };
    let ring = ring(&replayed, &operator);
    assert_eq!(ring.ignored(), &[]);
    assert_eq!(ring.principal("agent-key-2", now()), Ok(&agent_principal));
    assert_eq!(
        ring.principal("agent-key-1", now()),
        Err(Refusal::KeyLapsed)
    );
    assert_eq!(
        ring.principal("agent-key-1", instant("2026-10-31T23:59:59Z")),
        Ok(&agent_principal),
        "the earlier id is live until its own valid_until"
    );
    assert_eq!(
        ring.principal("agent-key-1", instant("2026-11-01T00:00:00Z")),
        Err(Refusal::KeyLapsed)
    );
}
