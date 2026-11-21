//! The acceptance of story:http-service: two correctly signed requests on a loopback socket, one
//! answered `posted` for an envelope as it was signed, one `SignatureInvalid` for the same envelope
//! with its `body` changed after signing.

mod support;

use inbox::messaging::Body;
use support::{AGENT, OTHER, World, encoded};

#[test]
fn a_signed_envelope_is_posted_and_the_same_envelope_with_its_body_changed_is_signature_invalid() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);

    let mut post = world.post(AGENT, OTHER, 1, "as it was signed");
    let posted = world.send(
        address,
        AGENT,
        "POST",
        "/messaging/commands/post",
        &encoded(&post),
    );
    assert_eq!(posted.status, 202, "{}", posted.body);
    assert_eq!(posted.text("outcome"), "posted");

    post.body = Body("changed after signing".to_owned());
    let refused = world.send(
        address,
        AGENT,
        "POST",
        "/messaging/commands/post",
        &encoded(&post),
    );
    assert_eq!(refused.status, 502, "{}", refused.body);
    assert_eq!(refused.text("outcome"), "signature-invalid");
    assert_eq!(refused.text("error"), "inbox.messaging.SignatureInvalid");

    server.stop().expect("the service stops");
}
