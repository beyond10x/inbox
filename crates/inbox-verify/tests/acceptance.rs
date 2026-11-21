//! The acceptance statement of story:signature-verifier: an envelope whose `body` changed after
//! signing is refused, and the unchanged envelope is accepted, against a key file the test's
//! operator key signed.

mod common;

use common::{AGENT, LATER, generate, now, over_the_wire, post, ring, signed_key_entry};
use inbox::messaging::{Body, KeyFile};
use inbox_verify::{Refusal, RefusalKind, sign_envelope};

#[test]
fn an_envelope_changed_after_signing_is_refused_and_the_unchanged_one_accepted() {
    let operator = generate();
    let agent = generate();
    let file = KeyFile {
        keys: vec![signed_key_entry(
            "agent-key-1",
            AGENT,
            &agent,
            LATER,
            &operator,
        )],
        delegations: vec![],
    };
    let ring = ring(&file, &operator);

    let mut envelope = post(AGENT, AGENT, "the original body");
    sign_envelope(&mut envelope, "agent-key-1", &agent).expect("the envelope signs");
    let received = over_the_wire(&envelope);

    assert_eq!(ring.verify_envelope(&received, now()), Ok(()));

    let mut changed = received.clone();
    changed.body = Body("a body changed after signing".to_owned());
    let refusal = ring
        .verify_envelope(&changed, now())
        .expect_err("a changed body is refused");
    assert_eq!(refusal, Refusal::SignatureMismatch);
    assert_eq!(refusal.kind(), RefusalKind::SignatureInvalid);
}
