//! Security review, pass 1, of story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal
//! in the verifier: step 4 of a request (`docs/design/signing.md`, "A request") refuses a text with
//! an empty value or a space or tab at either end even where an entry the operator signed names
//! exactly that text, compares the text byte for byte, and grants no step through a second
//! delegation. Every key is created when the test runs.

use crate::common;

use common::{
    AGENT, HUMAN, LATER, NOW, OTHER, generate, now, ring, signed_delegation, signed_key_entry,
};
use inbox::messaging::{KeyFile, Principal};
use inbox_verify::{Refusal, Request, sign_request_for};

fn a_request() -> Request<'static> {
    Request {
        method: "GET",
        path: "/messaging/views/pending",
        body: b"",
        key_id: "agent-key",
        timestamp: NOW,
    }
}

/// Texts that are no address of an inbox a request acts for, each named by a live entry of the
/// agent that the operator signed: only the grammar of step 4 refuses them.
const NOT_AN_ADDRESS: [&str; 5] = [
    "",
    " human:operator",
    "human:operator ",
    "\thuman:operator",
    "human:operator\t",
];

#[test]
fn a_signed_entry_for_a_text_that_is_no_address_admits_no_request() {
    let operator = generate();
    let agent = generate();
    let file = KeyFile {
        keys: vec![signed_key_entry(
            "agent-key",
            AGENT,
            &agent,
            LATER,
            &operator,
        )],
        delegations: NOT_AN_ADDRESS
            .iter()
            .enumerate()
            .map(|(n, text)| signed_delegation(&format!("odd-{n}"), AGENT, text, LATER, &operator))
            .collect(),
    };
    let ring = ring(&file, &operator);
    assert!(ring.ignored().is_empty(), "{:?}", ring.ignored());
    let request = a_request();
    for text in NOT_AN_ADDRESS {
        // The entry is held and live: the pair alone would admit it.
        assert!(
            ring.acts_for(&Principal(AGENT.to_owned()), text, now()),
            "{text:?}"
        );
        let signature = sign_request_for(&request, Some(text), &agent);
        assert_eq!(
            ring.verify_request_for(&request, Some(text), &signature, now()),
            Err(Refusal::NotActingFor),
            "{text:?}"
        );
    }
}

#[test]
fn the_pair_is_compared_byte_for_byte_and_grants_no_second_step() {
    let operator = generate();
    let agent = generate();
    // NFC `é` in the entry; the request names the same letter in NFD.
    let composed = "human:caf\u{e9}";
    let decomposed = "human:cafe\u{301}";
    let file = KeyFile {
        keys: vec![signed_key_entry(
            "agent-key",
            AGENT,
            &agent,
            LATER,
            &operator,
        )],
        delegations: vec![
            signed_delegation("composed", AGENT, composed, LATER, &operator),
            // AGENT for OTHER, and OTHER for HUMAN: no entry names AGENT for HUMAN.
            signed_delegation("agent-for-other", AGENT, OTHER, LATER, &operator),
            signed_delegation("other-for-human", OTHER, HUMAN, LATER, &operator),
        ],
    };
    let ring = ring(&file, &operator);
    let request = a_request();
    let admitted = |text: &str| {
        let signature = sign_request_for(&request, Some(text), &agent);
        ring.verify_request_for(&request, Some(text), &signature, now())
    };
    let agent_principal = Principal(AGENT.to_owned());
    assert_eq!(admitted(composed), Ok(&agent_principal));
    assert_eq!(admitted(OTHER), Ok(&agent_principal));
    for text in [decomposed, "HUMAN:CAF\u{c9}", HUMAN] {
        assert_eq!(admitted(text), Err(Refusal::NotActingFor), "{text:?}");
    }
}
